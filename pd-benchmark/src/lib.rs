//! Approach composition owns incumbent dependencies. Neutral libraries and the
//! RL experiment do not import this crate.
use pd_core::{BoundedRunLimitsV1, FlightProgramBindingV1, ScenarioSpec, TerrainDefinition, Vec2};
use pd_eval::{AttemptExecutionErrorV1, EpisodeEvaluationV1, EvidenceScope};
use pd_planner::ballistic_feedback::{BallisticOriginV1, WaypointExperiment, execute_configured};
use pd_planner::{BallisticFeedbackConfigV1, WaypointDirectNominalDirectGenerationRequest};
use pd_run::{AgentDescriptorV1, PreparedProgramV1};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncumbentAdapterV1 {
    pub schema_version: u32,
    pub mirror_leftward: bool,
    pub configuration: BallisticFeedbackConfigV1,
}

impl IncumbentAdapterV1 {
    pub fn new(mirror_leftward: bool, correction_cap: usize) -> Result<Self, String> {
        Ok(Self {
            schema_version: 1,
            mirror_leftward,
            configuration: BallisticFeedbackConfigV1::from_legacy(
                correction_cap,
                WaypointExperiment::AcquisitionTerminalSafety,
            )
            .map_err(|e| e.to_string())?,
        })
    }

    pub fn descriptor(&self) -> AgentDescriptorV1 {
        AgentDescriptorV1 {
            schema_version: 1,
            implementation: format!(
                "{}-cap{}-{}-adapter-v1",
                self.configuration.candidate_id,
                self.configuration.correction_cap,
                if self.mirror_leftward {
                    "reflection"
                } else {
                    "raw"
                }
            ),
            configuration: serde_json::to_value(self).expect("finite resolved configuration"),
            checkpoint_sha256: None,
        }
    }

    pub fn execute(
        &self,
        scenario: &ScenarioSpec,
        scope: &EvidenceScope,
    ) -> Result<(EpisodeEvaluationV1, serde_json::Value), AttemptExecutionErrorV1> {
        let fail = |e: String| AttemptExecutionErrorV1::Execution(e);
        if self.schema_version != 1 {
            return Err(fail("unsupported incumbent adapter version".into()));
        }
        self.configuration
            .validate()
            .map_err(|e| fail(e.to_string()))?;
        let ctx = pd_core::RunContext::from_scenario(scenario).map_err(fail)?;
        let source = ctx
            .world
            .landing_pads
            .iter()
            .find(|p| p.id == "source")
            .ok_or_else(|| {
                AttemptExecutionErrorV1::Unsupported("source pad is required by incumbent".into())
            })?;
        let reflected = self.mirror_leftward && ctx.target_pad.center_x_m < source.center_x_m;
        let input = if reflected {
            reflect_scenario(scenario)
        } else {
            scenario.clone()
        };
        let request = WaypointDirectNominalDirectGenerationRequest {
            scenario: input,
            source_pad_id: "source".into(),
            target_pad_id: ctx.target_pad.id.clone(),
            probe_id: "procedural-benchmark".into(),
            policy: Default::default(),
        };
        let preflight = pd_planner::preflight_waypoint_v2_flight(
            &request,
            &pd_planner::WaypointV2Policy::revision_3(),
        );
        if !preflight.supported {
            return Err(AttemptExecutionErrorV1::Unsupported(
                preflight
                    .reason
                    .unwrap_or_else(|| "incumbent input unsupported".into()),
            ));
        }
        let before = scope.snapshot().map_err(fail)?;
        let started = Instant::now();
        let origin =
            BallisticOriginV1::from_source(&request, None, 0).map_err(|e| fail(e.to_string()))?;
        let result = execute_configured(&request, &origin, false, &self.configuration)
            .map_err(|e| fail(e.to_string()))?;
        let end = result.live.evidence.final_state.physics_step;
        let interval = ctx.sim.control_interval_steps();
        let coverage = end.div_ceil(interval) * interval;
        let mut updates = result.updates.clone();
        if reflected {
            for update in &mut updates {
                update.command.target_attitude_rad *= -1.0;
            }
        }
        let program = PreparedProgramV1 {
            schema_version: 1,
            binding: FlightProgramBindingV1 {
                sim: ctx.sim.clone(),
                world: ctx.world.clone(),
                vehicle: ctx.vehicle.clone(),
                initial_state: ctx.initial_state.clone(),
                mission: ctx.mission.clone(),
            },
            descriptor: self.descriptor(),
            limits: BoundedRunLimitsV1 {
                command_coverage_end_physics_step: coverage,
                hard_end_physics_step: end,
            },
            updates,
        };
        let mut execution =
            pd_run::run_prepared_program(&ctx, &program).map_err(|e| fail(e.to_string()))?;
        if !reflected && execution.episode.final_state != result.live.evidence.final_state {
            return Err(fail(
                "raw adapter native endpoint differs from incumbent execution".into(),
            ));
        }
        execution.performance.wall_time_us =
            started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        // CPU timing for just command replay would misrepresent total solver cost.
        execution.performance.thread_cpu_time_us = None;
        let after = scope.snapshot().map_err(fail)?;
        let evaluation =
            pd_eval::assess_execution(scenario, execution, before, after, true).map_err(fail)?;
        let detail = serde_json::json!({"physical_sha256":evaluation.physical_sha256,
            "reflected":reflected,"approach_stop":result.stop,"terrain_domain_stop":result.terrain_domain_stop,
            "refreshes":result.refreshes,"handoffs":result.handoffs,
            "incumbent_final_state":result.live.evidence.final_state,
            "native_endpoint_equal_to_incumbent":!reflected});
        Ok((evaluation, detail))
    }
}

/// One global physical reflection, never a mission-ID or case-specific repair.
/// Execution/proof are subsequently performed on the original mission inputs.
pub fn reflect_scenario(scenario: &ScenarioSpec) -> ScenarioSpec {
    let mut reflected = scenario.clone();
    let points = scenario
        .world
        .terrain
        .points()
        .iter()
        .rev()
        .map(|p| Vec2::new(-p.x, p.y))
        .collect();
    reflected.world.terrain = TerrainDefinition::Heightfield { points_m: points };
    for pad in &mut reflected.world.landing_pads {
        pad.center_x_m *= -1.0;
    }
    reflected.initial_state.position_m.x *= -1.0;
    reflected.initial_state.velocity_mps.x *= -1.0;
    reflected.initial_state.attitude_rad *= -1.0;
    reflected.initial_state.angular_rate_radps *= -1.0;
    reflected
}
