//! Versioned effective implementation configurations. Historical descriptors
//! retain their original encoding; these contracts are recorded separately.
use crate::ballistic_feedback::{self, WaypointExperiment};
use crate::{
    BodyAwareTerminalPolicyV1, WaypointDirectNominalDirectGenerationPolicyV1, WaypointV2Policy,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy3ResolvedConfigV1 {
    pub schema_version: u32,
    pub policy: WaypointV2Policy,
    pub generation: WaypointDirectNominalDirectGenerationPolicyV1,
    pub terminal: BodyAwareTerminalPolicyV1,
}
impl Policy3ResolvedConfigV1 {
    pub fn from_legacy(
        policy: WaypointV2Policy,
        generation: WaypointDirectNominalDirectGenerationPolicyV1,
    ) -> Result<Self> {
        policy
            .validate_for_execution()
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            schema_version: 1,
            policy,
            generation,
            terminal: BodyAwareTerminalPolicyV1::default(),
        })
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "unsupported Policy 3 resolved configuration"
        );
        self.policy
            .validate_for_execution()
            .map_err(anyhow::Error::msg)?;
        ensure!(
            self.generation == WaypointDirectNominalDirectGenerationPolicyV1::default(),
            "unsupported Policy 3 generation configuration"
        );
        ensure!(
            self.terminal == BodyAwareTerminalPolicyV1::default(),
            "unsupported Policy 3 terminal configuration"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallisticFeedbackConfigV1 {
    pub schema_version: u32,
    pub candidate_id: String,
    pub experiment: WaypointExperiment,
    pub correction_cap: usize,
    pub switches: BTreeMap<String, bool>,
    pub terminal: pd_control::TerminalPdgResolvedConfigV1,
    pub standalone_coast_terminal: pd_control::TerminalPdgResolvedConfigV1,
    pub body_reserve: BodyAwareTerminalPolicyV1,
    pub refresh_ticks: u64,
    pub continuation_ticks: u64,
    pub minimum_descent_angle_rad: f64,
    pub thrust_derate: f64,
    pub maximum_profile_trials: u64,
    pub maximum_local_proposals: usize,
    pub case_wall_limit_s: u64,
    pub maximum_coast_ticks: u64,
    pub numeric_height_margin_m: f64,
    pub maximum_recovery_commands: usize,
    pub maximum_recovery_ticks: u64,
    pub recovery_braking_tilt_rad: f64,
}
impl BallisticFeedbackConfigV1 {
    pub fn from_legacy(correction_cap: usize, experiment: WaypointExperiment) -> Result<Self> {
        ensure!(
            [6, 24].contains(&correction_cap),
            "unsupported candidate correction cap"
        );
        Ok(Self {
            schema_version: 1,
            candidate_id: experiment.candidate_id().into(),
            experiment,
            correction_cap,
            switches: BTreeMap::from([
                ("mechanics".to_owned(), experiment.mechanics()),
                ("exit_consistency".to_owned(), experiment.exit_consistency()),
                ("finite_queries".to_owned(), experiment.finite_queries()),
                (
                    "finite_correction".to_owned(),
                    experiment.finite_correction(),
                ),
                ("phase_queries".to_owned(), experiment.phase_queries()),
                ("coast_transition".to_owned(), experiment.coast_transition()),
                (
                    "terminal_takeover".to_owned(),
                    experiment.terminal_takeover(),
                ),
                ("pad_clearance".to_owned(), experiment.pad_clearance()),
                (
                    "recovery_consistency".to_owned(),
                    experiment.recovery_consistency(),
                ),
                ("queued_recovery".to_owned(), experiment.queued_recovery()),
                (
                    "open_destination_acquisition".to_owned(),
                    experiment.open_destination_acquisition(),
                ),
                (
                    "terminal_safety_fallback".to_owned(),
                    experiment.terminal_safety_fallback(),
                ),
                (
                    "piecewise_early_target".to_owned(),
                    experiment.piecewise_early_target(),
                ),
                ("recovery_lead".to_owned(), experiment.recovery_lead()),
                (
                    "terminal_coordination".to_owned(),
                    experiment.terminal_coordination(),
                ),
                ("effort".to_owned(), experiment.effort()),
                ("recovery".to_owned(), experiment.recovery()),
                ("local_height".to_owned(), experiment.local_height()),
                ("early_target".to_owned(), experiment.early_target()),
                ("landing_duration".to_owned(), experiment.landing_duration()),
                (
                    "landing_countdown".to_owned(),
                    experiment.landing_countdown(),
                ),
                ("coast_terminal".to_owned(), experiment.coast_terminal()),
                (
                    "landing_braking_guard".to_owned(),
                    experiment.landing_braking_guard(),
                ),
                (
                    "landing_body_centering".to_owned(),
                    experiment.landing_body_centering(),
                ),
            ]),
            terminal: ballistic_feedback::terminal_configuration(experiment),
            standalone_coast_terminal: pd_control::TerminalPdgController::default()
                .resolved_configuration(),
            body_reserve: BodyAwareTerminalPolicyV1::default(),
            refresh_ticks: ballistic_feedback::REFRESH_TICKS,
            continuation_ticks: ballistic_feedback::CONTINUATION_TICKS,
            minimum_descent_angle_rad: ballistic_feedback::MIN_ANGLE,
            thrust_derate: ballistic_feedback::THRUST_DERATE,
            maximum_profile_trials: ballistic_feedback::MAX_PROFILE_TRIALS,
            maximum_local_proposals: ballistic_feedback::MAX_LOCAL_PROPOSALS,
            case_wall_limit_s: 60,
            maximum_coast_ticks: ballistic_feedback::coast_terminal::MAX_COAST_TICKS,
            numeric_height_margin_m: ballistic_feedback::clearance::NUMERIC_HEIGHT_MARGIN_M,
            maximum_recovery_commands: ballistic_feedback::avoidance::MAX_COMMANDS,
            maximum_recovery_ticks: ballistic_feedback::avoidance::MAX_RECOVERY_TICKS,
            recovery_braking_tilt_rad: ballistic_feedback::avoidance::BRAKING_TILT,
        })
    }
    pub fn validate(&self) -> Result<()> {
        self.terminal.validate().map_err(anyhow::Error::msg)?;
        self.standalone_coast_terminal
            .validate()
            .map_err(anyhow::Error::msg)?;
        ensure!(
            *self == Self::from_legacy(self.correction_cap, self.experiment)?,
            "unsupported or incomplete effective ballistic configuration"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_references_and_all_legacy_ablations_resolve_without_hidden_flags() {
        use clap::ValueEnum;
        for experiment in WaypointExperiment::value_variants() {
            for cap in [6, 24] {
                let config = BallisticFeedbackConfigV1::from_legacy(cap, *experiment).unwrap();
                config.validate().unwrap();
                let decoded: BallisticFeedbackConfigV1 =
                    serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
                assert_eq!(decoded, config);
                let mut changed = config.clone();
                changed.refresh_ticks += 2;
                assert!(changed.validate().is_err());
                let mut changed = config.clone();
                changed.terminal.ballistic_terminal_coordination_enabled =
                    !changed.terminal.ballistic_terminal_coordination_enabled;
                assert!(changed.validate().is_err());
            }
        }
        assert!(BallisticFeedbackConfigV1::from_legacy(7, WaypointExperiment::Ridge).is_err());
    }
    #[test]
    fn policy3_acceptance_does_not_admit_historical_policies() {
        let config = Policy3ResolvedConfigV1::from_legacy(
            WaypointV2Policy::revision_3(),
            WaypointDirectNominalDirectGenerationPolicyV1::default(),
        )
        .unwrap();
        config.validate().unwrap();
        let mut changed = config.clone();
        changed.policy = WaypointV2Policy::revision_2();
        assert!(changed.validate().is_err());
        let mut changed = config;
        changed.schema_version = 2;
        assert!(changed.validate().is_err());
    }
    #[test]
    fn configured_policy3_reproduces_legacy_execution_and_rejects_mismatched_generation() {
        let request = crate::test_inputs::planner_request("v2_clear_685");
        let config = Policy3ResolvedConfigV1::from_legacy(
            WaypointV2Policy::revision_3(),
            request.policy.clone(),
        )
        .unwrap();
        let mut configured =
            crate::WaypointV2Session::start_configured(request.clone(), &config).unwrap();
        while !matches!(
            configured.advance_piece().unwrap(),
            crate::WaypointV2SessionProgress::Terminal { .. }
        ) {}
        let mut actual = configured.finish().unwrap().clone();
        let mut expected = crate::run_waypoint_v2_flight(&request, &config.policy).unwrap();
        actual.timings = crate::WaypointV2Timings::default();
        expected.timings = crate::WaypointV2Timings::default();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        let mut changed = config;
        changed.generation.analytical_policy.minimum_clearance_m += 1.0;
        assert!(crate::WaypointV2Session::start_configured(request, &changed).is_err());
    }
    #[test]
    fn ballistic_origin_rejects_input_drift_and_fresh_neutral_execution() {
        let request = crate::test_inputs::planner_request("v2_clear_685");
        let origin = ballistic_feedback::BallisticOriginV1::from_source(&request, None, 0).unwrap();
        let config = BallisticFeedbackConfigV1::from_legacy(6, WaypointExperiment::Ridge).unwrap();
        assert!(ballistic_feedback::execute_configured(&request, &origin, true, &config).is_err());
        let mut changed = request;
        changed.scenario.vehicle.initial_fuel_kg -= 1.0;
        assert!(ballistic_feedback::execute_configured(&changed, &origin, false, &config).is_err());
    }
    #[test]
    fn retained_ballistic_origin_rebuilds_actual_commands_and_rejects_corrupt_prefix() {
        let request = crate::test_inputs::planner_request("v2_plateau_reference_900");
        let baseline =
            crate::run_waypoint_v2_flight(&request, &WaypointV2Policy::revision_3()).unwrap();
        assert!(baseline.cycles.len() > 1);
        let origin =
            ballistic_feedback::BallisticOriginV1::from_source(&request, Some(&baseline), 1)
                .unwrap();
        assert_eq!(
            origin.evidence().final_state,
            baseline.cycles[1].current_state
        );
        assert!(origin.evidence().final_state.physics_step > 0);
        assert!(!origin.evidence().actions.is_empty());
        let mut corrupt = baseline.clone();
        corrupt.segments[0].end_state.fuel_kg -= 1.0;
        assert!(
            ballistic_feedback::BallisticOriginV1::from_source(&request, Some(&corrupt), 1)
                .is_err()
        );
        let mut corrupt = baseline;
        corrupt.final_source_replay_passed = false;
        assert!(
            ballistic_feedback::BallisticOriginV1::from_source(&request, Some(&corrupt), 1)
                .is_err()
        );
    }
}
