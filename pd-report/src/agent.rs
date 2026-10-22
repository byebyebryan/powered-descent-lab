//! Neutral projection into the maintained rich run template. Evaluation owns
//! proof checking; this module consumes its disposition without executing agents.

use anyhow::{Result, ensure};
use pd_core::{
    BoundedRunStopCauseV1, MissionOutcome, PhysicalOutcome, RunContext, SampleRecord, ScenarioSpec,
};
use pd_run::BoundedAgentRunArtifactsV1;

use crate::flight_annotations::{AnnotationNavigation, FlightAnnotations, NavigationLink};

#[derive(Clone, Copy, Debug, Default)]
pub struct AgentReportEvidenceV1 {
    pub integrity_passed: bool,
    pub physical_prefix_verified: bool,
    pub complete_episode_verified: bool,
    pub verified_target_landing: bool,
}

pub fn render_agent_run_report(
    scenario: &ScenarioSpec,
    execution: &BoundedAgentRunArtifactsV1,
    evidence: AgentReportEvidenceV1,
    navigation: AnnotationNavigation,
) -> Result<String> {
    render_agent_run_report_with_reference(scenario, execution, evidence, navigation, None)
}

pub fn render_agent_run_report_with_reference(
    scenario: &ScenarioSpec,
    execution: &BoundedAgentRunArtifactsV1,
    evidence: AgentReportEvidenceV1,
    navigation: AnnotationNavigation,
    reference_overlay: Option<crate::flight_annotations::ReferenceOverlayV1>,
) -> Result<String> {
    let episode = &execution.episode;
    ensure!(
        !evidence.verified_target_landing
            || (evidence.integrity_passed
                && evidence.physical_prefix_verified
                && evidence.complete_episode_verified
                && episode.failure.is_none()
                && episode.stop == BoundedRunStopCauseV1::MissionTerminal
                && episode.run.manifest.physical_outcome == PhysicalOutcome::LandedOnTarget
                && episode.run.manifest.mission_outcome == MissionOutcome::Success),
        "report landing projection lacks the complete physical/mission/integrity/replay tuple"
    );
    let mut samples = episode.run.samples.clone();
    let mut caption = format!(
        "Execution stop: {:?} · Integrity: {} · Physical prefix replay: {} · Complete episode replay: {} · Verified target landing: {}",
        episode.stop,
        evidence.integrity_passed,
        evidence.physical_prefix_verified,
        evidence.complete_episode_verified,
        evidence.verified_target_landing
    );
    if samples
        .last()
        .is_none_or(|sample| sample.physics_step != episode.final_state.physics_step)
    {
        let ctx = RunContext::from_scenario(scenario).map_err(anyhow::Error::msg)?;
        let state = episode.final_state.to_simulation_state();
        samples.push(SampleRecord {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            observation: state.build_observation(&ctx),
            held_command: state.held_command,
        });
        caption.push_str(" · Final observation is a display projection of the saved endpoint; the raw sample ledger is unchanged.");
    }
    let annotations = FlightAnnotations {
        caption,
        navigation,
        reference_overlay,
        ..Default::default()
    };
    let configuration = serde_json::to_value(&execution.descriptor)?;
    crate::render_run_report_with_flight_annotations(
        scenario,
        Some(&configuration),
        &episode.run.manifest,
        &episode.run.events,
        &samples,
        &execution.controller_updates,
        Some(&execution.performance),
        None,
        None,
        None,
        Some(&annotations),
    )
}

/// Relative capture navigation remains useful in a standalone local directory.
pub fn agent_capture_navigation() -> AnnotationNavigation {
    AnnotationNavigation {
        source_links: ["evaluation.json", "seal.json"]
            .map(|file| NavigationLink {
                label: file.to_owned(),
                href: file.to_owned(),
            })
            .to_vec(),
        ..Default::default()
    }
}
