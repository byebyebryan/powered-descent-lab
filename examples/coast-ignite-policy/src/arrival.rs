//! Full native episodes for the opt-in arrival/composition characterization.
use super::calibration::{WitnessMonitor, WitnessRecipe, witness_command};
use super::*;
use pd_control::arrival_actuation::{AllocationRegime, ArrivalActuationProfile, ArrivalAllocation};
use pd_control::maneuver::{ArrivalLandingController, ArrivalLandingProgram};

enum Executor {
    Guidance(Box<ArrivalLandingController>),
    Witness {
        ctx: Box<pd_core::RunContext>,
        continuation_scenario: Option<Box<ScenarioSpec>>,
        continuation: Option<Box<ArrivalLandingController>>,
        recipe: WitnessRecipe,
        monitor: Box<WitnessMonitor>,
    },
}
impl Executor {
    fn frame(
        &mut self,
        o: &Observation,
        remaining: u64,
    ) -> Result<pd_control::ControllerFrame, String> {
        match self {
            Self::Guidance(c) => c.frame(o, remaining),
            Self::Witness {
                ctx,
                continuation_scenario,
                continuation,
                recipe,
                monitor,
            } => {
                monitor.inspect(o);
                if monitor.status != pd_control::ArrivalStatus::Active
                    && let Some(scenario) = continuation_scenario
                {
                    if continuation.is_none() {
                        *continuation = Some(Box::new(
                            ArrivalLandingController::landing_from_tick(scenario, o.physics_step)?,
                        ));
                    }
                    return continuation.as_mut().unwrap().frame(o, remaining);
                }
                let mut f =
                    pd_control::ControllerFrame::command_only(witness_command(ctx, o, recipe)?);
                f.phase = Some(
                    if o.physics_step < recipe.physics_ticks {
                        "independent_witness_prefix"
                    } else {
                        "witness_observation_tail"
                    }
                    .into(),
                );
                Ok(f)
            }
        }
    }
    fn feedback(&self) -> Option<ManeuverFeedbackV2> {
        match self {
            Self::Guidance(c) => c.feedback(),
            Self::Witness { continuation, .. } => continuation.as_ref().and_then(|c| c.feedback()),
        }
    }
    fn arrival(&self) -> serde_json::Value {
        match self {
            Self::Guidance(c) => serde_json::to_value(c.arrival_feedback()).unwrap(),
            Self::Witness { monitor, .. } => monitor.json(),
        }
    }
    fn model_steps(&self) -> u64 {
        match self {
            Self::Guidance(c) => c.arrival_feedback().map_or(0, |a| a.model_steps),
            _ => 0,
        }
    }
    fn forecast_steps(&self) -> u64 {
        match self {
            Self::Guidance(c) => c.landing_search().forecast_steps,
            Self::Witness { continuation, .. } => continuation
                .as_ref()
                .map_or(0, |c| c.landing_search().forecast_steps),
        }
    }
    fn work_steps(&self) -> u64 {
        self.model_steps() + self.forecast_steps()
    }
    fn landing_search(&self) -> serde_json::Value {
        match self {
            Self::Guidance(c) => serde_json::to_value(c.landing_search()).unwrap(),
            Self::Witness { continuation, .. } => continuation.as_ref().map_or_else(
                || serde_json::to_value(LandingSearchFeedback::default()).unwrap(),
                |c| serde_json::to_value(c.landing_search()).unwrap(),
            ),
        }
    }
    fn allocation(&self) -> Option<ArrivalAllocation> {
        match self {
            Self::Guidance(c) => c.arrival_feedback()?.actuation.as_ref()?.last,
            _ => None,
        }
    }
    fn native_end(&mut self, o: &Observation, end: NativeEndV1) -> Result<(), String> {
        match self {
            Self::Guidance(c) => c.native_end(o, end),
            Self::Witness {
                monitor,
                continuation,
                ..
            } => {
                monitor.inspect(o);
                if monitor.status == pd_control::ArrivalStatus::Active {
                    monitor.status = pd_control::ArrivalStatus::NativeEnd;
                    monitor.resolution = Some(o.clone());
                }
                if let Some(c) = continuation {
                    c.native_end(o, end)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Default, Serialize)]
struct PlantAllocation {
    updates: u64,
    aligned_updates: u64,
    aligned_tracking_updates: u64,
    maximum_aligned_forward_error_mps2: f64,
    maximum_aligned_requested_error_mps2: f64,
    maximum_aligned_tracking_error_mps2: f64,
    maximum_vector_projection_error_mps2: f64,
}
impl PlantAllocation {
    fn observe(
        &mut self,
        before: &Observation,
        after: &Observation,
        steps: u64,
        a: ArrivalAllocation,
    ) {
        if steps != 2 {
            return;
        }
        self.updates += 1;
        let actual = pd_core::Vec2::new(
            (after.velocity_mps.x - before.velocity_mps.x) * 60.0,
            (after.velocity_mps.y - before.velocity_mps.y) * 60.0 + before.gravity_mps2,
        );
        self.maximum_vector_projection_error_mps2 = self
            .maximum_vector_projection_error_mps2
            .max((actual - a.projected_accel_mps2).length());
        if (before.attitude_rad - a.command.target_attitude_rad).abs() < 1e-9
            && (after.attitude_rad - a.command.target_attitude_rad).abs() < 1e-9
        {
            self.aligned_updates += 1;
            self.maximum_aligned_forward_error_mps2 = self
                .maximum_aligned_forward_error_mps2
                .max((actual.length() - a.aligned_mean_accel_mps2).abs());
            self.maximum_aligned_requested_error_mps2 = self
                .maximum_aligned_requested_error_mps2
                .max((actual.length() - a.projected_magnitude_mps2).abs());
            if a.regime == AllocationRegime::Tracking {
                self.aligned_tracking_updates += 1;
                self.maximum_aligned_tracking_error_mps2 = self
                    .maximum_aligned_tracking_error_mps2
                    .max((actual.length() - a.projected_magnitude_mps2).abs());
            }
        }
    }
}

#[pyclass]
pub struct ArrivalEpisode {
    scenario: ScenarioSpec,
    episode: Option<Episode>,
    controller: Executor,
    profile: Option<ArrivalActuationProfile>,
    witness_recipe: Option<WitnessRecipe>,
    plant: Option<PlantAllocation>,
    descriptor: AgentDescriptorV1,
    fingerprint: Sha256,
    started: Instant,
    scope: Option<EvidenceScope>,
    before: Option<EvidenceSnapshotV1>,
    native_success: bool,
    minimum_fuel_timing: bool,
}

#[pymethods]
impl ArrivalEpisode {
    #[new]
    #[allow(clippy::too_many_arguments)] // Preserve the existing Python argument order.
    #[pyo3(signature=(scenario_json, program_json, captured=false, descriptor_json=None, input_files=None, actuation_profile=None, witness_recipe_json=None, timing_selection=None))]
    fn new(
        scenario_json: &str,
        program_json: &str,
        captured: bool,
        descriptor_json: Option<&str>,
        input_files: Option<Vec<String>>,
        actuation_profile: Option<&str>,
        witness_recipe_json: Option<&str>,
        timing_selection: Option<&str>,
    ) -> PyResult<Self> {
        let scenario: ScenarioSpec = decode(scenario_json)?;
        let program: ArrivalLandingProgram = decode(program_json)?;
        let profile: Option<ArrivalActuationProfile> = actuation_profile
            .map(|s| decode(&format!("\"{s}\"")))
            .transpose()?;
        let recipe: Option<WitnessRecipe> = witness_recipe_json.map(decode).transpose()?;
        let minimum_fuel_timing = match timing_selection {
            None | Some("first_feasible_v1") => false,
            Some("minimum_fuel_grid_v1") => true,
            Some(_) => return Err(error("unknown timing selection profile")),
        };
        if minimum_fuel_timing && recipe.is_some() {
            return Err(error("fuel-ranked timing requires the guidance executor"));
        }
        if profile.is_some() && recipe.is_some() {
            return Err(error(
                "witness and guidance profiles are mutually exclusive",
            ));
        }
        let mut controller = if let Some(recipe) = &recipe {
            if recipe.physics_ticks == 0
                || recipe.physics_ticks > 2400
                || !recipe.physics_ticks.is_multiple_of(2)
                || !recipe.prepare_ticks.is_multiple_of(2)
                || recipe.prepare_ticks >= recipe.physics_ticks
            {
                return Err(error("invalid bounded witness program"));
            }
            let ctx = pd_core::RunContext::from_scenario(&scenario).map_err(error)?;
            let o = pd_core::SimulationState::new(&ctx)
                .map_err(error)?
                .build_observation(&ctx);
            let request = program
                .arrival
                .clone()
                .ok_or_else(|| error("witness requires a bound arrival request"))?;
            let mut admission =
                pd_control::AirborneArrivalController::new(&scenario, 0).map_err(error)?;
            if admission.submit(&request, &o).map_err(error)? != AdmissionV2::Accepted {
                return Err(error("witness request admission failed"));
            }
            Executor::Witness {
                ctx: Box::new(ctx),
                continuation_scenario: program.land.then(|| Box::new(scenario.clone())),
                continuation: None,
                recipe: recipe.clone(),
                monitor: Box::new(WitnessMonitor::new(request, o)),
            }
        } else if let Some(profile) = profile {
            Executor::Guidance(Box::new(
                ArrivalLandingController::with_actuation_profile(
                    &scenario,
                    program.clone(),
                    profile,
                )
                .map_err(error)?,
            ))
        } else {
            Executor::Guidance(Box::new(
                ArrivalLandingController::new(&scenario, program.clone()).map_err(error)?,
            ))
        };
        if minimum_fuel_timing && let Executor::Guidance(c) = &mut controller {
            c.use_minimum_fuel_timing().map_err(error)?;
        }
        let mut descriptor =
            descriptor_json
                .map(decode)
                .transpose()?
                .unwrap_or(AgentDescriptorV1 {
                    schema_version: 1,
                    implementation: "airborne-arrival-composition-v1".into(),
                    checkpoint_sha256: None,
                    configuration: serde_json::to_value(program).map_err(error)?,
                });
        if minimum_fuel_timing {
            descriptor
                .configuration
                .as_object_mut()
                .ok_or_else(|| error("timing descriptor configuration must be an object"))?
                .insert("timing_selection".into(), "minimum_fuel_grid_v1".into());
        }
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
            minimum_fuel_timing,
            profile,
            witness_recipe: recipe,
            plant: profile.map(|_| PlantAllocation::default()),
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
            let allocation = if frame.phase.as_deref() == Some("airborne_arrival") {
                self.controller.allocation()
            } else {
                None
            };
            self.fingerprint
                .update(serde_json::to_vec(&frame).map_err(error)?);
            let result = self
                .episode
                .as_mut()
                .unwrap()
                .step_frame(frame, None)
                .map_err(error)?;
            if let (Some(plant), Some(allocation)) = (&mut self.plant, allocation) {
                plant.observe(
                    &o,
                    &result.observation,
                    result.physics_steps_advanced,
                    allocation,
                );
            }
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
        let mut value = serde_json::json!({"physics_steps":o.physics_step,"stop":e.stop(),"fingerprint":format!("{:x}",self.fingerprint.clone().finalize()),"native_success":self.native_success,"observation":o,"feedback":self.controller.feedback(),"arrival":self.controller.arrival(),"landing_search":self.controller.landing_search(),"model_steps":self.controller.model_steps(),"forecast_steps":self.controller.forecast_steps()});
        if self.minimum_fuel_timing {
            value["timing_selection"] = "minimum_fuel_grid_v1".into();
        }
        if let Some(profile) = self.profile {
            value["actuation_profile"] = serde_json::to_value(profile).map_err(error)?;
            value["plant_allocation"] = serde_json::to_value(&self.plant).map_err(error)?;
        }
        if let Some(recipe) = &self.witness_recipe {
            value["witness_recipe"] = serde_json::to_value(recipe).map_err(error)?;
        }
        Ok(value)
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
        info["arrival"] = self.controller.arrival();
        Ok((result, info))
    }
}
