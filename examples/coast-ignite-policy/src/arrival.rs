//! Full native episodes for the opt-in arrival/composition characterization.
use super::*;
use pd_control::maneuver::{ArrivalLandingController, ArrivalLandingProgram};

#[pyclass]
pub struct ArrivalEpisode {
    scenario: ScenarioSpec,
    episode: Option<Episode>,
    controller: ArrivalLandingController,
    descriptor: AgentDescriptorV1,
    fingerprint: Sha256,
    started: Instant,
    scope: Option<EvidenceScope>,
    before: Option<EvidenceSnapshotV1>,
    native_success: bool,
}

#[pymethods]
impl ArrivalEpisode {
    #[new]
    #[pyo3(signature=(scenario_json, program_json, captured=false, descriptor_json=None, input_files=None))]
    fn new(
        scenario_json: &str,
        program_json: &str,
        captured: bool,
        descriptor_json: Option<&str>,
        input_files: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let scenario: ScenarioSpec = decode(scenario_json)?;
        let program: ArrivalLandingProgram = decode(program_json)?;
        let controller =
            ArrivalLandingController::new(&scenario, program.clone()).map_err(error)?;
        let descriptor = descriptor_json
            .map(decode)
            .transpose()?
            .unwrap_or(AgentDescriptorV1 {
                schema_version: 1,
                implementation: "airborne-arrival-composition-v1".into(),
                checkpoint_sha256: None,
                configuration: serde_json::to_value(program).map_err(error)?,
            });
        descriptor.validate().map_err(error)?;
        let end = (scenario.sim.max_time_s * 120.0).floor() as u64;
        let episode = Episode::reset(
            &scenario,
            &descriptor.implementation,
            BoundedRunLimitsV1 {
                hard_end_physics_step: end,
                command_coverage_end_physics_step: end,
            },
            if captured {
                CaptureModeV1::Captured
            } else {
                CaptureModeV1::Summary
            },
        )
        .map_err(error)?;
        if episode.stop().is_some() {
            return Err(error("arrival requires active airborne original"));
        }
        let scope = captured.then(|| super::scope(input_files.unwrap_or_default()));
        let before = scope.as_ref().map(checked_snapshot).transpose()?;
        Ok(Self {
            scenario,
            episode: Some(episode),
            controller,
            descriptor,
            fingerprint: Sha256::new(),
            started: Instant::now(),
            scope,
            before,
            native_success: false,
        })
    }

    fn execute(&mut self, py: Python<'_>, work_limit: u64) -> PyResult<String> {
        while self.live()?.stop().is_none() {
            py.check_signals()?;
            let o = self.live()?.observation();
            let used = o.physics_step + self.controller.work_steps();
            // One kernel update is bounded by its finite candidate bank and
            // 12-second rollouts. Reserve two million steps before entering it.
            if work_limit.saturating_sub(used) < 2_000_002 {
                return Err(error("insufficient per-update work reservation"));
            }
            let frame = self
                .controller
                .frame(&o, work_limit - used - 2)
                .map_err(error)?;
            self.fingerprint
                .update(serde_json::to_vec(&frame).map_err(error)?);
            let result = self
                .episode
                .as_mut()
                .unwrap()
                .step_frame(frame, None)
                .map_err(error)?;
            self.fingerprint
                .update(serde_json::to_vec(&result).map_err(error)?);
            self.native_success = result.physical_outcome == PhysicalOutcome::LandedOnTarget
                && result.mission_outcome == MissionOutcome::Success;
            if matches!(
                result.stop,
                Some(pd_core::BoundedRunStopCauseV1::ExecutionInvalid)
            ) {
                return Err(error("native execution fault"));
            }
        }
        json(&self.info()?)
    }
    fn status_json(&self) -> PyResult<String> {
        json(&self.info()?)
    }
    fn finish_json(&mut self) -> PyResult<String> {
        let (r, info) = self.finish()?;
        json(&serde_json::json!({"summary":r.summary,"info":info}))
    }
    fn save_capture(&mut self, output: &str) -> PyResult<String> {
        let before = self
            .before
            .clone()
            .ok_or_else(|| error("Summary has no capture scope"))?;
        let (result, info) = self.finish()?;
        let execution = BoundedAgentRunArtifactsV1 {
            descriptor: self.descriptor.clone(),
            episode: result.capture.ok_or_else(|| error("missing capture"))?,
            controller_updates: result.agent_updates.unwrap(),
            performance: RunPerformanceStats {
                wall_time_us: self.started.elapsed().as_micros() as u64,
                thread_cpu_time_us: None,
            },
        };
        if replay_fingerprint(&self.scenario, &execution).map_err(error)?
            != format!("{:x}", self.fingerprint.clone().finalize())
        {
            return Err(error("capture fingerprint differs"));
        }
        let evaluation = pd_eval::assess_execution(
            &self.scenario,
            execution,
            before,
            checked_snapshot(self.scope.as_ref().unwrap())?,
            true,
        )
        .map_err(error)?;
        if !evaluation.integrity_passed || !evaluation.replay.complete_episode_verified {
            return Err(error("physical/integrity proof failed"));
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
impl ArrivalEpisode {
    fn live(&self) -> PyResult<&Episode> {
        self.episode
            .as_ref()
            .ok_or_else(|| error("episode consumed"))
    }
    fn info(&self) -> PyResult<serde_json::Value> {
        let e = self.live()?;
        let o = e.observation();
        Ok(
            serde_json::json!({"physics_steps":o.physics_step,"stop":e.stop(),"fingerprint":format!("{:x}",self.fingerprint.clone().finalize()),"native_success":self.native_success,"observation":o,"feedback":self.controller.feedback(),"arrival":self.controller.arrival_feedback(),"landing_search":self.controller.landing_search(),"model_steps":self.controller.arrival_feedback().map_or(0,|a|a.model_steps),"forecast_steps":self.controller.landing_search().forecast_steps}),
        )
    }
    fn finish(&mut self) -> PyResult<(pd_run::EpisodeResultV1, serde_json::Value)> {
        if self.live()?.stop().is_none() {
            return Err(error("cannot finish active episode"));
        }
        let mut info = self.info()?;
        let observation = self.live()?.observation();
        let result = self.episode.take().unwrap().finish().map_err(error)?;
        if result.summary.failure.is_some() {
            return Err(error("native software fault"));
        }
        self.controller
            .native_end(&observation, (&result.summary).into())
            .map_err(error)?;
        info["feedback"] = serde_json::to_value(self.controller.feedback()).map_err(error)?;
        info["arrival"] =
            serde_json::to_value(self.controller.arrival_feedback()).map_err(error)?;
        Ok((result, info))
    }
}
