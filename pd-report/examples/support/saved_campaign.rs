//! Read-only saved-capture adapter. No agent execution, planning or native replay.
use std::{fs, io::Write, path::Path};

use anyhow::{Result, ensure};
use pd_core::{FlightProgramBindingV1, ScenarioSpec};
use pd_report::campaign::{CampaignPageV1, CampaignRowV1};
use pd_report::campaign_view::CampaignPresentationV1;
use pd_run::BoundedAgentRunArtifactsV1;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn read_regular(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && metadata.len() <= max_bytes,
        "expected bounded regular file: {}",
        path.display()
    );
    Ok(fs::read(path)?)
}

pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(bytes)?;
    Ok(())
}

#[derive(Deserialize)]
pub struct SavedEvaluation {
    pub schema_version: u32,
    pub scenario: ScenarioSpec,
    pub physical_sha256: String,
    pub execution: BoundedAgentRunArtifactsV1,
    pub integrity_passed: bool,
    pub replay: SavedReplay,
    pub verified_target_landing: bool,
}

#[derive(Deserialize)]
pub struct SavedReplay {
    pub physical_prefix_verified: bool,
    pub complete_episode_verified: bool,
}

pub fn read_capture(
    root: &Path,
    page: &CampaignPageV1,
    row: &CampaignRowV1,
) -> Result<(SavedEvaluation, String)> {
    let bytes = read_regular(&root.join("evaluation.json"), 64 * 1024 * 1024)?;
    let seal: serde_json::Value =
        serde_json::from_slice(&read_regular(&root.join("seal.json"), 1024 * 1024)?)?;
    let identity = digest(&bytes);
    ensure!(
        seal["schema_version"] == 1 && seal["evaluation_sha256"] == identity,
        "saved capture byte seal differs"
    );
    let value: SavedEvaluation = serde_json::from_slice(&bytes)?;
    let scenario = &value.scenario;
    let binding = FlightProgramBindingV1 {
        sim: scenario.sim.clone(),
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        mission: scenario.mission.clone(),
    };
    ensure!(
        value.schema_version == 1
            && digest(&serde_json::to_vec(&binding)?) == row.physical_sha256
            && value.physical_sha256 == row.physical_sha256,
        "saved physical identity differs"
    );
    let episode = &value.execution.episode;
    let manifest = &episode.run.manifest;
    ensure!(
        value.execution.descriptor == page.agent
            && row.stop == Some(episode.stop)
            && row.physical_outcome.as_ref() == Some(&manifest.physical_outcome)
            && row.mission_outcome.as_ref() == Some(&manifest.mission_outcome)
            && row.execution_valid == episode.failure.is_none()
            && row.integrity_passed == value.integrity_passed
            && row.physical_replay_verified == value.replay.physical_prefix_verified
            && row.complete_replay_verified == value.replay.complete_episode_verified
            && row.verified_target_landing == value.verified_target_landing
            && row.sim_time_s == Some(manifest.sim_time_s)
            && row.fuel_kg == Some(episode.final_state.fuel_kg),
        "saved capture differs from campaign projection"
    );
    Ok((value, identity))
}

pub fn capture_directory(source: &Path, row: &CampaignRowV1) -> Result<std::path::PathBuf> {
    let href = row
        .report_href
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("executed row has no saved report link"))?;
    let path = Path::new(href);
    ensure!(
        path.components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
            && path.file_name().is_some_and(|name| name == "report.html"),
        "unsafe saved report path"
    );
    let parent = source.join(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("missing capture directory"))?,
    );
    ensure!(
        fs::symlink_metadata(&parent)?.is_dir(),
        "capture root must be a regular directory"
    );
    Ok(parent)
}

pub fn preview_projection(
    value: &SavedEvaluation,
    href: String,
) -> Result<(String, CampaignPresentationV1)> {
    let scenario = &value.scenario;
    let episode = &value.execution.episode;
    let mut samples = episode.run.samples.clone();
    let projected = samples
        .last()
        .is_none_or(|sample| sample.physics_step != episode.final_state.physics_step);
    if projected {
        let context = pd_core::RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
        let state = episode.final_state.to_simulation_state();
        samples.push(pd_core::SampleRecord {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            observation: state.build_observation(&context),
            held_command: state.held_command,
        });
    }
    let manifest = &episode.run.manifest;
    let svg = pd_report::build_saved_flight_preview_svg(scenario, Some(manifest), &samples, &[]);
    let target = scenario
        .world
        .landing_pad(scenario.mission.goal.target_pad_id());
    let state = &episode.final_state;
    let view = CampaignPresentationV1 {
        schema_version: 1,
        preview_href: href,
        caption: if projected {
            "Recorded flight · saved endpoint added for display".into()
        } else {
            "Recorded terrain and flight".into()
        },
        target_horizontal_offset_m: target.map(|pad| (state.position_m.x - pad.center_x_m).abs()),
        height_above_upright_touchdown_m: target.map(|pad| {
            state.position_m.y - pad.surface_y_m - scenario.vehicle.geometry.touchdown_base_offset_m
        }),
        endpoint_speed_mps: Some(state.velocity_mps.x.hypot(state.velocity_mps.y)),
    };
    Ok((svg, view))
}
