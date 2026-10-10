//! Thin physical episode binding. Observations, rewards, exploration, sampling,
//! optimization and checkpoint selection belong to the Python experiment.
use std::path::{Path, PathBuf};

use pd_core::{Command, Observation, RunContext, ScenarioSpec};
use pd_run::{
    AgentDescriptorV1, BoundedAgentRunArtifactsV1, CaptureModeV1, Episode, RunPerformanceStats,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

fn error(value: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(value.to_string())
}
fn json<T: Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(error)
}
fn decode<T: for<'de> Deserialize<'de>>(value: &str) -> PyResult<T> {
    serde_json::from_str(value).map_err(error)
}

/// Stable order documented in experiments/rl/README.md. No generator metadata.
fn observation(o: Observation) -> Vec<f64> {
    vec![
        o.sim_time_s,
        o.physics_step as f64,
        o.position_m.x,
        o.position_m.y,
        o.velocity_mps.x,
        o.velocity_mps.y,
        o.attitude_rad,
        o.angular_rate_radps,
        o.mass_kg,
        o.fuel_kg,
        o.gravity_mps2,
        o.target_dx_m,
        o.height_above_target_m,
        o.target_surface_y_m,
        o.target_pad_half_width_m,
        o.touchdown_clearance_m,
        o.min_hull_clearance_m,
    ]
}

#[pyclass]
struct NativeEpisode {
    scenario: ScenarioSpec,
    episode: Option<Episode>,
}

#[pymethods]
impl NativeEpisode {
    #[new]
    #[pyo3(signature = (scenario_json, captured=false, agent_id="python-agent"))]
    fn new(scenario_json: &str, captured: bool, agent_id: &str) -> PyResult<Self> {
        let scenario: ScenarioSpec = decode(scenario_json)?;
        let ctx = RunContext::from_scenario(&scenario).map_err(error)?;
        let limits = pd_run::scenario_horizon_limits(&ctx).map_err(error)?;
        let mode = if captured {
            CaptureModeV1::Captured
        } else {
            CaptureModeV1::Summary
        };
        let episode = Episode::reset(&scenario, agent_id, limits, mode).map_err(error)?;
        Ok(Self {
            scenario,
            episode: Some(episode),
        })
    }

    fn observation(&self) -> PyResult<Vec<f64>> {
        Ok(observation(self.active()?.observation()))
    }

    fn physical_json(&self) -> PyResult<String> {
        let m = self.active()?.mission();
        json(
            &serde_json::json!({"sim":m.sim,"vehicle":m.vehicle,"gravity_mps2":m.world.gravity_mps2,
            "initial_state":m.initial_state,"mission":m.mission,"target_pad":m.target_pad,
            "terrain_domain_m":[m.world.terrain.points().first().map(|p|p.x),
                m.world.terrain.points().last().map(|p|p.x)]}),
        )
    }

    fn terrain_heights(&self, xs: Vec<f64>) -> PyResult<Vec<f64>> {
        if xs.len() > 4096 || xs.iter().any(|x| !x.is_finite()) {
            return Err(error(
                "terrain queries require at most 4096 finite coordinates",
            ));
        }
        let terrain = &self.active()?.mission().world.terrain;
        Ok(xs.into_iter().map(|x| terrain.sample_height(x)).collect())
    }

    #[pyo3(signature = (throttle_frac, target_attitude_rad, compute_time_us=None))]
    fn step(
        &mut self,
        throttle_frac: f64,
        target_attitude_rad: f64,
        compute_time_us: Option<u64>,
    ) -> PyResult<(Vec<f64>, bool, u64, String, String)> {
        let result = self
            .active_mut()?
            .step_frame(
                pd_run::AgentFrame::command_only(Command {
                    throttle_frac,
                    target_attitude_rad,
                }),
                compute_time_us,
            )
            .map_err(error)?;
        Ok((
            observation(result.observation),
            result.stop.is_some(),
            result.physics_steps_advanced,
            json(&result.physical_outcome)?,
            json(&result.mission_outcome)?,
        ))
    }

    fn retained_record_counts(&self) -> PyResult<(usize, usize, usize, usize)> {
        Ok(self.active()?.retained_record_counts())
    }

    /// Summary mode carries physical outcomes, never saved-command proof.
    fn finish_json(&mut self) -> PyResult<String> {
        let episode = self
            .episode
            .take()
            .ok_or_else(|| error("episode already consumed"))?;
        json(&episode.finish().map_err(error)?)
    }

    #[pyo3(signature = (descriptor_json, evidence_before_json, evidence_after_json, output, wall_time_us, report=true))]
    fn save_capture(
        &mut self,
        descriptor_json: &str,
        evidence_before_json: &str,
        evidence_after_json: &str,
        output: &str,
        wall_time_us: u64,
        report: bool,
    ) -> PyResult<String> {
        let descriptor: AgentDescriptorV1 = decode(descriptor_json)?;
        descriptor.validate().map_err(error)?;
        let episode = self
            .episode
            .take()
            .ok_or_else(|| error("episode already consumed"))?
            .finish()
            .map_err(error)?;
        let execution = BoundedAgentRunArtifactsV1 {
            descriptor,
            episode: episode
                .capture
                .ok_or_else(|| error("summary mode has no command capture"))?,
            controller_updates: episode
                .agent_updates
                .ok_or_else(|| error("missing command updates"))?,
            performance: RunPerformanceStats {
                wall_time_us,
                thread_cpu_time_us: None,
            },
        };
        let evaluation = pd_eval::assess_execution(
            &self.scenario,
            execution,
            decode(evidence_before_json)?,
            decode(evidence_after_json)?,
            true,
        )
        .map_err(error)?;
        pd_eval::save_evaluation(Path::new(output), &evaluation).map_err(error)?;
        if report {
            let html = pd_report::render_agent_run_report(
                &evaluation.scenario,
                &evaluation.execution,
                pd_report::AgentReportEvidenceV1 {
                    integrity_passed: evaluation.integrity_passed,
                    physical_prefix_verified: evaluation.replay.physical_prefix_verified,
                    complete_episode_verified: evaluation.replay.complete_episode_verified,
                    verified_target_landing: evaluation.verified_target_landing,
                },
                pd_report::agent_capture_navigation(),
            )
            .map_err(error)?;
            let path = Path::new(output).join("report.html");
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(error)?;
            file.write_all(html.as_bytes()).map_err(error)?;
        }
        json(
            &serde_json::json!({"physical_sha256":evaluation.physical_sha256,
            "manifest":evaluation.execution.episode.run.manifest,"stop":evaluation.execution.episode.stop,
            "failure":evaluation.execution.episode.failure,"integrity_passed":evaluation.integrity_passed,
            "replay":evaluation.replay,"verified_target_landing":evaluation.verified_target_landing}),
        )
    }
}

impl NativeEpisode {
    fn active(&self) -> PyResult<&Episode> {
        self.episode
            .as_ref()
            .ok_or_else(|| error("episode already consumed"))
    }
    fn active_mut(&mut self) -> PyResult<&mut Episode> {
        self.episode
            .as_mut()
            .ok_or_else(|| error("episode already consumed"))
    }
}

#[pyfunction]
fn generate_mission(family: &str, seed: u64, index: u64) -> PyResult<String> {
    let recipe = match family {
        "value-noise" => pd_missions::MissionRecipeV1::default(),
        "layered" => pd_missions::MissionRecipeV1::layered(),
        _ => return Err(error("unknown terrain family")),
    };
    json(&pd_missions::generate_mission(&recipe, seed, index).map_err(error)?)
}

#[pyfunction]
fn evidence_snapshot(sources: Vec<String>, inputs: Vec<String>) -> PyResult<String> {
    let scope = pd_eval::EvidenceScope {
        sources: sources.into_iter().map(PathBuf::from).collect(),
        inputs: inputs.into_iter().map(PathBuf::from).collect(),
    };
    json(&scope.snapshot().map_err(error)?)
}

#[pyfunction]
fn verify_capture(path: &str) -> PyResult<String> {
    let evaluation = pd_eval::load_verified_evaluation(Path::new(path)).map_err(error)?;
    json(
        &serde_json::json!({"integrity_passed":evaluation.integrity_passed,
        "replay":evaluation.replay,"verified_target_landing":evaluation.verified_target_landing}),
    )
}

#[pyfunction]
fn load_collection_missions(
    path: &str,
    indices: Vec<u64>,
    usage_id: &str,
    exposure: &str,
) -> PyResult<String> {
    let root = Path::new(path);
    let manifest = pd_missions::read_collection_manifest(root).map_err(error)?;
    let exposure = match exposure {
        "tuning" => pd_missions::CollectionExposureV1::Tuning,
        "evaluation" => pd_missions::CollectionExposureV1::Evaluation,
        _ => return Err(error("exposure must be tuning or evaluation")),
    };
    if indices.is_empty() || indices.len() > 100_000 {
        return Err(error("collection schedule must contain 1..100000 indices"));
    }
    let entries: std::collections::BTreeMap<_, _> = manifest
        .entries
        .iter()
        .map(|entry| (entry.index, entry))
        .collect();
    if indices.iter().any(|index| !entries.contains_key(index)) {
        return Err(error(
            "scheduled index is absent from the resolved collection",
        ));
    }
    pd_missions::record_collection_usage(root, usage_id, exposure,
        "Consumed declared collection inputs in independent Python composition; consult run request for schedule").map_err(error)?;
    let missions: Result<Vec<_>, _> = indices
        .into_iter()
        .map(|index| pd_missions::load_collection_mission(root, &manifest, entries[&index]))
        .collect();
    json(&missions.map_err(error)?)
}

#[pymodule]
fn pd_native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<NativeEpisode>()?;
    module.add_function(wrap_pyfunction!(generate_mission, module)?)?;
    module.add_function(wrap_pyfunction!(evidence_snapshot, module)?)?;
    module.add_function(wrap_pyfunction!(verify_capture, module)?)?;
    module.add_function(wrap_pyfunction!(load_collection_missions, module)?)?;
    Ok(())
}
