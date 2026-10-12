//! Saved-episode projection only. No input generation, evaluation or replay.
use pd_core::{RunContext, SampleRecord, ScenarioSpec};
use pd_run::BoundedAgentRunArtifactsV1;

use crate::campaign_view::CampaignPresentationV1;

pub fn build_agent_campaign_preview(
    scenario: &ScenarioSpec,
    execution: &BoundedAgentRunArtifactsV1,
    preview_href: &str,
) -> Result<(CampaignPresentationV1, String), String> {
    let episode = &execution.episode;
    let mut samples = episode.run.samples.clone();
    let state = &episode.final_state;
    if samples
        .last()
        .is_none_or(|s| s.physics_step != state.physics_step)
    {
        let ctx = RunContext::from_scenario(scenario)?;
        let sim = state.to_simulation_state();
        samples.push(SampleRecord {
            sim_time_s: sim.sim_time_s,
            physics_step: sim.physics_step,
            observation: sim.build_observation(&ctx),
            held_command: sim.held_command,
        });
    }
    let svg =
        crate::build_saved_flight_preview_svg(scenario, Some(&episode.run.manifest), &samples, &[]);
    let target = scenario
        .world
        .landing_pad(scenario.mission.goal.target_pad_id());
    let view = CampaignPresentationV1 {
        schema_version: 1,
        preview_href: preview_href.into(),
        caption: "Recorded terrain and flight · direction normalized for display".into(),
        target_horizontal_offset_m: target.map(|p| (state.position_m.x - p.center_x_m).abs()),
        height_above_upright_touchdown_m: target.map(|p| {
            state.position_m.y - p.surface_y_m - scenario.vehicle.geometry.touchdown_base_offset_m
        }),
        endpoint_speed_mps: Some(state.velocity_mps.x.hypot(state.velocity_mps.y)),
    };
    if !view.validate() {
        return Err("invalid saved campaign preview projection".into());
    }
    Ok((view, svg))
}
