//! Read-only, versioned acceptance for the frozen native Planner V2 capture.
//!
//! The checker validates the saved capture and binds its ordered input
//! identity to the currently tracked `DEFAULT_PLANNER_PACK_PATH` expansion.
//! It never invokes planning, simulation, or source replay. Replay and integrity
//! are evidence recorded in the sealed flight artifacts and checked by the
//! existing capture validator.

use std::path::Path;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use crate::waypoint_v2_pack::{
    WaypointV2BatchCase, WaypointV2BatchReport, WaypointV2PackGroup,
    check_default_planner_v2_binding, validated_waypoint_v2_batch,
};

pub const WAYPOINT_V2_ACCEPTANCE_SCHEMA_ID: &str = "planner_v2_acceptance_v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaypointV2AcceptanceStatus {
    #[serde(rename = "PASSED")]
    Passed,
    #[serde(rename = "FAILED")]
    Failed,
    #[serde(rename = "notaccepted")]
    NotAccepted,
}

impl WaypointV2AcceptanceStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "PASSED",
            Self::Failed => "FAILED",
            Self::NotAccepted => "notaccepted",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Passed => "good",
            Self::Failed => "bad",
            Self::NotAccepted => "warn",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2AcceptanceIssue {
    pub case_id: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2AcceptanceTotals {
    pub case_count: usize,
    pub clear_count: usize,
    pub ordinary_count: usize,
    pub additional_terrain_count: usize,
    pub diagnostic_count: usize,
    pub mandatory_landing_count: usize,
    pub mandatory_landed_count: usize,
    pub clear_direct_count: usize,
    pub terrain_blocked_corrected_count: usize,
    pub diagnostic_landed_count: usize,
    pub diagnostic_finite_stop_count: usize,
    pub supported_count: usize,
    pub unsupported_count: usize,
    pub integrity_passed_count: usize,
    pub supported_replay_passed_count: usize,
}

/// Stable JSON returned by `check-planner-v2` and embedded in the common batch
/// report. `passed` is false for both failed and notaccepted captures; `status`
/// distinguishes those cases for offline historical rendering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaypointV2AcceptanceV1 {
    pub schema_id: String,
    pub status: WaypointV2AcceptanceStatus,
    pub passed: bool,
    pub issues: Vec<WaypointV2AcceptanceIssue>,
    pub totals: WaypointV2AcceptanceTotals,
}

/// Check a saved V2 capture without writing files or rerunning any evidence.
pub fn check_waypoint_v2_acceptance(capture_root: &Path) -> Result<WaypointV2AcceptanceV1> {
    let report = validated_waypoint_v2_batch(capture_root)?;
    assess_waypoint_v2_acceptance(&report)
}

/// Assess an already validated native V2 report. The saved capture validator
/// proves that report rows match the expanded-input, flight, summary, and raw
/// artifact bytes; this function adds the tracked-default binding and outcome
/// gates. It does not simulate or replay a saved case.
pub fn assess_waypoint_v2_acceptance(
    report: &WaypointV2BatchReport,
) -> Result<WaypointV2AcceptanceV1> {
    if matches!(report.policy_version, 1 | 2) {
        let mut verdict = base_verdict(
            WaypointV2AcceptanceStatus::NotAccepted,
            totals_from_cases(&report.cases),
        );
        verdict.issues.push(WaypointV2AcceptanceIssue {
            case_id: None,
            code: "policy_not_accepted".into(),
            message: format!(
                "Policy {} is a retained historical capture; only policy 3 is eligible for this acceptance check.",
                report.policy_version
            ),
        });
        return Ok(verdict);
    }

    ensure!(
        report.policy_version == 3,
        "unsupported planner policy version"
    );
    let (expected_inputs, expected_identity) = check_default_planner_v2_binding()?;
    ensure!(
        report.pack_id == "planner_v2_lab_suite"
            && report.case_count == expected_inputs.len()
            && report.cases.len() == expected_inputs.len(),
        "Planner V2 acceptance requires the complete frozen default case set"
    );
    ensure!(
        report.input_identity == expected_identity
            && report.pack_snapshot_sha256 == expected_identity.pack_file_sha256,
        "Planner V2 capture identity does not match the tracked default pack expansion"
    );
    for (input, case) in expected_inputs.iter().zip(&report.cases) {
        ensure!(
            case_matches_frozen_input(case, input),
            "Planner V2 case identity, order, or group differs from the tracked default pack at {}",
            input.case_id
        );
    }

    let totals = totals_from_cases(&report.cases);
    let mut issues = Vec::new();
    if report.provenance.unchanged_during_capture != Some(true)
        || report.provenance.source_after.as_ref() != Some(&report.provenance.source_before)
    {
        issues.push(issue(
            None,
            "capture_source_drift",
            "Capture provenance must record unchanged inputs and identical before/after source states.",
        ));
    }

    let supported_diagnostic_stops = [
        "no_nominal",
        "nominal_rejected",
        "no_clearing",
        "correction_limit",
        "deadline",
        "no_progress",
    ];

    for (input, case) in expected_inputs.iter().zip(&report.cases) {
        let is_unsupported = input.expected_preflight.as_deref() == Some("unsupported");
        let is_core = !matches!(input.group, WaypointV2PackGroup::Diagnostic);
        if case.integrity_passed != Some(true) {
            issues.push(issue(
                Some(&case.case_id),
                "integrity_failed",
                "Saved case integrity did not pass; inspect the case summary and flight evidence.",
            ));
        }

        if is_unsupported {
            if case.status != "preflight_rejected"
                || case.planning_stop.as_deref() != Some("unsupported")
                || case.physical_outcome.is_some()
                || case.mission_outcome.is_some()
                || case.final_source_replay_passed == Some(true)
                || case.error.is_some()
            {
                issues.push(issue(
                    Some(&case.case_id),
                    "unsupported_has_execution_claim",
                    "Expected unsupported preflight must remain unsimulated, with no physical, mission, or successful replay claim.",
                ));
            }
            continue;
        }

        if case.status != "simulated" {
            issues.push(issue(
                Some(&case.case_id),
                "supported_case_not_simulated",
                "Supported case must have a verified simulated result; inspect its saved flight and replay evidence.",
            ));
        }
        if case.final_source_replay_passed != Some(true) {
            issues.push(issue(
                Some(&case.case_id),
                "source_replay_failed",
                "Saved final-source replay did not pass; inspect the case flight and run summary.",
            ));
        }
        if case.error.is_some()
            || matches!(
                case.planning_stop.as_deref(),
                Some("implementation_error" | "invalid_input" | "unsupported")
            )
        {
            issues.push(issue(
                Some(&case.case_id),
                "planner_execution_error",
                "Case recorded an implementation, invalid-input, or unexpected unsupported stop.",
            ));
        }
        if case.physical_outcome.as_deref() == Some("crashed") {
            issues.push(issue(
                Some(&case.case_id),
                "physical_crash",
                "Case crashed; inspect the saved flight report and contact evidence.",
            ));
        }

        let landed = verified_target_landing(case);
        if is_core {
            if !landed {
                issues.push(issue(
                    Some(&case.case_id),
                    "mandatory_target_landing_missing",
                    format!(
                        "Mandatory core case did not verify a target landing (physical={}, mission={}, planning_stop={}); inspect the linked case report.",
                        case.physical_outcome.as_deref().unwrap_or("missing"),
                        case.mission_outcome.as_deref().unwrap_or("missing"),
                        case.planning_stop.as_deref().unwrap_or("missing")
                    ),
                ));
            }
            match input.group {
                WaypointV2PackGroup::Clear => {
                    if case.correction_count != Some(0) {
                        issues.push(issue(
                            Some(&case.case_id),
                            "clear_case_required_zero_corrections",
                            "Clear control must land directly with zero corrections.",
                        ));
                    }
                    if case.initial_nominal_terrain_blocked != Some(false) {
                        issues.push(issue(
                            Some(&case.case_id),
                            "clear_case_nominal_was_blocked",
                            "Clear control must record an initially unblocked nominal path.",
                        ));
                    }
                }
                WaypointV2PackGroup::Ordinary | WaypointV2PackGroup::AdditionalTerrain => {
                    if case.initial_nominal_terrain_blocked != Some(true) {
                        issues.push(issue(
                            Some(&case.case_id),
                            "terrain_nominal_not_blocked",
                            "Terrain case must record an initially terrain-blocked nominal path.",
                        ));
                    }
                    if case.correction_count.is_none_or(|count| count < 1) {
                        issues.push(issue(
                            Some(&case.case_id),
                            "terrain_case_missing_correction",
                            "Terrain case must record at least one executed correction.",
                        ));
                    }
                }
                WaypointV2PackGroup::Diagnostic => unreachable!("core excludes diagnostics"),
            }
        } else if !landed && !finite_supported_diagnostic_stop(case, &supported_diagnostic_stops) {
            issues.push(issue(
                Some(&case.case_id),
                "diagnostic_outcome_not_finite",
                "Supported diagnostic must verify a target landing or an allowed finite planning stop with flying physical state and an in-progress mission.",
            ));
        }
    }

    if totals.case_count != 44
        || totals.clear_count != 11
        || totals.ordinary_count != 16
        || totals.additional_terrain_count != 9
        || totals.diagnostic_count != 8
        || totals.mandatory_landing_count != 36
        || totals.supported_count != 42
        || totals.unsupported_count != 2
    {
        // These counts are derived from case rows and the frozen identity. A
        // mismatch therefore indicates a structural inconsistency, not a
        // planner outcome regression.
        anyhow::bail!("Planner V2 acceptance denominators differ from frozen default pack");
    }

    let status = if issues.is_empty() {
        WaypointV2AcceptanceStatus::Passed
    } else {
        WaypointV2AcceptanceStatus::Failed
    };
    let passed = status == WaypointV2AcceptanceStatus::Passed;
    Ok(WaypointV2AcceptanceV1 {
        schema_id: WAYPOINT_V2_ACCEPTANCE_SCHEMA_ID.into(),
        status,
        passed,
        issues,
        totals,
    })
}

fn base_verdict(
    status: WaypointV2AcceptanceStatus,
    totals: WaypointV2AcceptanceTotals,
) -> WaypointV2AcceptanceV1 {
    WaypointV2AcceptanceV1 {
        schema_id: WAYPOINT_V2_ACCEPTANCE_SCHEMA_ID.into(),
        status,
        passed: false,
        issues: Vec::new(),
        totals,
    }
}

fn totals_from_cases(cases: &[WaypointV2BatchCase]) -> WaypointV2AcceptanceTotals {
    let mut totals = WaypointV2AcceptanceTotals {
        case_count: cases.len(),
        ..WaypointV2AcceptanceTotals::default()
    };
    for case in cases {
        match case.group {
            WaypointV2PackGroup::Clear => totals.clear_count += 1,
            WaypointV2PackGroup::Ordinary => totals.ordinary_count += 1,
            WaypointV2PackGroup::AdditionalTerrain => totals.additional_terrain_count += 1,
            WaypointV2PackGroup::Diagnostic => totals.diagnostic_count += 1,
        }
        if case.integrity_passed == Some(true) {
            totals.integrity_passed_count += 1;
        }
        if case.expected_preflight.as_deref() == Some("unsupported") {
            totals.unsupported_count += 1;
        } else {
            totals.supported_count += 1;
            if case.final_source_replay_passed == Some(true) {
                totals.supported_replay_passed_count += 1;
            }
        }
        let landed = verified_target_landing(case);
        if !matches!(case.group, WaypointV2PackGroup::Diagnostic) {
            totals.mandatory_landing_count += 1;
            if landed {
                totals.mandatory_landed_count += 1;
            }
        }
        if case.group == WaypointV2PackGroup::Clear
            && case.correction_count == Some(0)
            && case.initial_nominal_terrain_blocked == Some(false)
            && landed
        {
            totals.clear_direct_count += 1;
        }
        if matches!(
            case.group,
            WaypointV2PackGroup::Ordinary | WaypointV2PackGroup::AdditionalTerrain
        ) && case.initial_nominal_terrain_blocked == Some(true)
            && case.correction_count.is_some_and(|count| count >= 1)
            && landed
        {
            totals.terrain_blocked_corrected_count += 1;
        }
        if case.group == WaypointV2PackGroup::Diagnostic {
            if landed {
                totals.diagnostic_landed_count += 1;
            } else if case.expected_preflight.as_deref() != Some("unsupported")
                && finite_supported_diagnostic_stop(
                    case,
                    &[
                        "no_nominal",
                        "nominal_rejected",
                        "no_clearing",
                        "correction_limit",
                        "deadline",
                        "no_progress",
                    ],
                )
            {
                totals.diagnostic_finite_stop_count += 1;
            }
        }
    }
    totals
}

fn finite_supported_diagnostic_stop(case: &WaypointV2BatchCase, allowed_stops: &[&str]) -> bool {
    case.status == "simulated"
        && allowed_stops.contains(&case.planning_stop.as_deref().unwrap_or(""))
        && case.physical_outcome.as_deref() == Some("flying")
        && case.mission_outcome.as_deref() == Some("in_progress")
}

fn case_matches_frozen_input(
    case: &WaypointV2BatchCase,
    input: &crate::waypoint_v2_pack::WaypointV2PackInput,
) -> bool {
    case.case_id == input.case_id
        && case.source_set == input.source_set
        && case.source_group == input.source_group
        && case.group == input.group
        && case.family == input.family
        && case.base_case_id == input.base_case_id
        && case.source_pad_id == input.source_pad_id
        && case.target_pad_id == input.target_pad_id
        && case.expected_preflight == input.expected_preflight
}

fn verified_target_landing(case: &WaypointV2BatchCase) -> bool {
    case.physical_outcome.as_deref() == Some("landed_on_target")
        && case.mission_outcome.as_deref() == Some("success")
        && case.planning_stop.as_deref() == Some("landed")
        && case.integrity_passed == Some(true)
        && case.final_source_replay_passed == Some(true)
}

fn issue(
    case_id: Option<&str>,
    code: &str,
    message: impl Into<String>,
) -> WaypointV2AcceptanceIssue {
    WaypointV2AcceptanceIssue {
        case_id: case_id.map(str::to_owned),
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::{
        waypoint_direct_body_aware_terminal::sha256_bytes,
        waypoint_v2_pack::{
            DEFAULT_PLANNER_PACK_PATH, WAYPOINT_V2_BATCH_SCHEMA_ID, WaypointV2BatchCase,
            WaypointV2BatchProvenance, WaypointV2PackGroup, WaypointV2SourceState,
        },
    };
    use anyhow::Context;

    fn source_state(tag: &str) -> WaypointV2SourceState {
        WaypointV2SourceState {
            git_commit: Some(tag.into()),
            git_dirty: Some(false),
            rust_source_tree_sha256: "a".repeat(64),
            executable_sha256: "b".repeat(64),
        }
    }

    fn synthetic_case(
        input: &crate::waypoint_v2_pack::WaypointV2PackInput,
        diagnostic_landed: bool,
    ) -> WaypointV2BatchCase {
        let unsupported = input.expected_preflight.as_deref() == Some("unsupported");
        let diagnostic = input.group == WaypointV2PackGroup::Diagnostic;
        let landed = !unsupported && (!diagnostic || diagnostic_landed);
        let correction_count = match input.group {
            WaypointV2PackGroup::Clear => 0,
            WaypointV2PackGroup::Ordinary | WaypointV2PackGroup::AdditionalTerrain => 1,
            WaypointV2PackGroup::Diagnostic => u32::from(landed),
        };
        let planning_stop = if unsupported {
            "unsupported"
        } else if landed {
            "landed"
        } else {
            "no_clearing"
        };
        let physical_outcome = if unsupported {
            None
        } else if landed {
            Some("landed_on_target".into())
        } else {
            Some("flying".into())
        };
        let mission_outcome = if unsupported {
            None
        } else if landed {
            Some("success".into())
        } else {
            Some("in_progress".into())
        };
        WaypointV2BatchCase {
            case_id: input.case_id.clone(),
            source_set: input.source_set.clone(),
            source_group: input.source_group.clone(),
            group: input.group.clone(),
            family: input.family.clone(),
            base_case_id: input.base_case_id.clone(),
            source_pad_id: input.source_pad_id.clone(),
            target_pad_id: input.target_pad_id.clone(),
            expected_preflight: input.expected_preflight.clone(),
            status: if unsupported {
                "preflight_rejected"
            } else {
                "simulated"
            }
            .into(),
            outcome: Some(
                physical_outcome
                    .clone()
                    .unwrap_or_else(|| planning_stop.to_owned()),
            ),
            planning_stop: Some(planning_stop.into()),
            reason: None,
            correction_count: Some(correction_count),
            initial_nominal_terrain_blocked: match input.group {
                WaypointV2PackGroup::Clear => Some(false),
                WaypointV2PackGroup::Ordinary | WaypointV2PackGroup::AdditionalTerrain => {
                    Some(true)
                }
                WaypointV2PackGroup::Diagnostic => Some(true),
            },
            integrity_passed: Some(true),
            final_source_replay_passed: Some(!unsupported),
            physical_outcome,
            mission_outcome,
            planning_s: Some(0.0),
            execution_s: Some(0.0),
            replay_s: Some(0.0),
            input_path: String::new(),
            scenario_path: format!("runs/{}/scenario.json", input.case_id),
            flight_path: format!("runs/{}/flight.json", input.case_id),
            summary_path: format!("runs/{}/summary.json", input.case_id),
            rich_report_path: None,
            annotated_report_path: format!("runs/{}/index.html", input.case_id),
            artifact_sha256: BTreeMap::new(),
            error: None,
        }
    }

    fn synthetic_accepted_report() -> WaypointV2BatchReport {
        let (inputs, input_identity) = check_default_planner_v2_binding().unwrap();
        let cases = inputs
            .iter()
            .map(|input| synthetic_case(input, true))
            .collect::<Vec<_>>();
        let summary = crate::waypoint_v2_pack::summarize(&cases).unwrap();
        let source_before = source_state("historical-source");
        WaypointV2BatchReport {
            schema_id: WAYPOINT_V2_BATCH_SCHEMA_ID.into(),
            pack_id: "planner_v2_lab_suite".into(),
            name: "Planner V2 lab suite".into(),
            status: "completed".into(),
            policy_version: 3,
            case_count: cases.len(),
            cases,
            summary,
            pack_snapshot_sha256: input_identity.pack_file_sha256.clone(),
            expanded_inputs_snapshot_sha256: input_identity
                .rust_typed_expanded_inputs_sha256
                .clone(),
            pack_snapshot_path: "pack.json".into(),
            expanded_inputs_snapshot_path: "expanded-inputs.json".into(),
            input_identity,
            provenance: WaypointV2BatchProvenance {
                source_before: source_before.clone(),
                source_after: Some(source_before),
                unchanged_during_capture: Some(true),
            },
        }
    }

    fn case_index(report: &WaypointV2BatchReport, case_id: &str) -> usize {
        report
            .cases
            .iter()
            .position(|case| case.case_id == case_id)
            .unwrap()
    }

    #[test]
    fn synthetic_full_record_set_passes_frozen_policy_three_gates() {
        let verdict = assess_waypoint_v2_acceptance(&synthetic_accepted_report()).unwrap();
        assert!(verdict.passed);
        assert_eq!(verdict.status, WaypointV2AcceptanceStatus::Passed);
        assert_eq!(verdict.schema_id, "planner_v2_acceptance_v1");
        assert_eq!(verdict.totals.case_count, 44);
        assert_eq!(verdict.totals.mandatory_landing_count, 36);
        assert_eq!(verdict.totals.mandatory_landed_count, 36);
        assert_eq!(verdict.totals.clear_direct_count, 11);
        assert_eq!(verdict.totals.terrain_blocked_corrected_count, 25);
        assert_eq!(verdict.totals.supported_count, 42);
        assert_eq!(verdict.totals.unsupported_count, 2);
        assert_eq!(verdict.totals.integrity_passed_count, 44);
        assert_eq!(verdict.totals.supported_replay_passed_count, 42);
        assert_eq!(verdict.totals.diagnostic_landed_count, 6);
    }

    #[test]
    fn mandatory_landing_clear_and_terrain_rules_report_case_reasons() {
        let mut report = synthetic_accepted_report();
        let ordinary = case_index(&report, "v2_ridge_early");
        report.cases[ordinary].physical_outcome = Some("flying".into());
        report.cases[ordinary].mission_outcome = Some("in_progress".into());
        report.cases[ordinary].planning_stop = Some("no_clearing".into());
        report.cases[ordinary].outcome = Some("flying".into());
        let verdict = assess_waypoint_v2_acceptance(&report).unwrap();
        assert!(!verdict.passed);
        assert!(verdict.issues.iter().any(|issue| {
            issue.case_id.as_deref() == Some("v2_ridge_early")
                && issue.code == "mandatory_target_landing_missing"
        }));

        let mut report = synthetic_accepted_report();
        let clear = case_index(&report, "v2_clear_685");
        report.cases[clear].correction_count = Some(1);
        let verdict = assess_waypoint_v2_acceptance(&report).unwrap();
        assert!(verdict.issues.iter().any(|issue| {
            issue.case_id.as_deref() == Some("v2_clear_685")
                && issue.code == "clear_case_required_zero_corrections"
        }));

        let mut report = synthetic_accepted_report();
        let terrain = case_index(&report, "fresh_ridge_early_900");
        report.cases[terrain].correction_count = Some(0);
        let verdict = assess_waypoint_v2_acceptance(&report).unwrap();
        assert!(verdict.issues.iter().any(|issue| {
            issue.case_id.as_deref() == Some("fresh_ridge_early_900")
                && issue.code == "terrain_case_missing_correction"
        }));
    }

    #[test]
    fn supported_diagnostic_finite_stop_and_added_correction_remain_allowed() {
        let mut report = synthetic_accepted_report();
        let diagnostic = report
            .cases
            .iter()
            .position(|case| {
                case.group == WaypointV2PackGroup::Diagnostic && case.expected_preflight.is_none()
            })
            .unwrap();
        report.cases[diagnostic].planning_stop = Some("no_clearing".into());
        report.cases[diagnostic].physical_outcome = Some("flying".into());
        report.cases[diagnostic].mission_outcome = Some("in_progress".into());
        report.cases[diagnostic].outcome = Some("flying".into());
        report.cases[diagnostic].correction_count = Some(2);
        let verdict = assess_waypoint_v2_acceptance(&report).unwrap();
        assert!(verdict.passed, "{:?}", verdict.issues);
        assert_eq!(verdict.totals.diagnostic_finite_stop_count, 1);
    }

    #[test]
    fn unsupported_preflight_has_no_physical_mission_or_replay_claim() {
        let report = synthetic_accepted_report();
        let unsupported = report
            .cases
            .iter()
            .find(|case| case.expected_preflight.as_deref() == Some("unsupported"))
            .unwrap();
        assert_eq!(unsupported.status, "preflight_rejected");
        assert_eq!(unsupported.planning_stop.as_deref(), Some("unsupported"));
        assert!(unsupported.physical_outcome.is_none());
        assert!(unsupported.mission_outcome.is_none());
        assert_eq!(unsupported.final_source_replay_passed, Some(false));
        assert!(assess_waypoint_v2_acceptance(&report).unwrap().passed);
    }

    #[test]
    fn integrity_replay_errors_crashes_and_unverified_results_fail() {
        let mut report = synthetic_accepted_report();
        let id = "v2_diag_long_plateau";
        let index = case_index(&report, id);
        report.cases[index].integrity_passed = Some(false);
        let verdict = assess_waypoint_v2_acceptance(&report).unwrap();
        assert!(
            verdict
                .issues
                .iter()
                .any(|issue| issue.code == "integrity_failed")
        );

        let mut report = synthetic_accepted_report();
        let index = case_index(&report, id);
        report.cases[index].final_source_replay_passed = Some(false);
        assert!(
            assess_waypoint_v2_acceptance(&report)
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "source_replay_failed")
        );

        let mut report = synthetic_accepted_report();
        let index = case_index(&report, id);
        report.cases[index].status = "simulation_unverified".into();
        assert!(
            assess_waypoint_v2_acceptance(&report)
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "supported_case_not_simulated")
        );

        let mut report = synthetic_accepted_report();
        let index = case_index(&report, id);
        report.cases[index].planning_stop = Some("implementation_error".into());
        report.cases[index].error = Some("synthetic error".into());
        assert!(
            assess_waypoint_v2_acceptance(&report)
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "planner_execution_error")
        );

        let mut report = synthetic_accepted_report();
        let index = case_index(&report, id);
        report.cases[index].physical_outcome = Some("crashed".into());
        report.cases[index].mission_outcome = Some("failed_crash".into());
        assert!(
            assess_waypoint_v2_acceptance(&report)
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "physical_crash")
        );
    }

    #[test]
    fn frozen_identity_and_metadata_are_structural_and_historical_policies_notaccepted() {
        let mut report = synthetic_accepted_report();
        report.cases[0].case_id = "replacement-case".into();
        assert!(assess_waypoint_v2_acceptance(&report).is_err());

        let mut report = synthetic_accepted_report();
        report.cases[0].group = WaypointV2PackGroup::AdditionalTerrain;
        assert!(assess_waypoint_v2_acceptance(&report).is_err());

        let mut report = synthetic_accepted_report();
        report.input_identity.rust_typed_expanded_inputs_sha256 = "0".repeat(64);
        assert!(assess_waypoint_v2_acceptance(&report).is_err());

        let mut report = synthetic_accepted_report();
        report.policy_version = 4;
        assert!(assess_waypoint_v2_acceptance(&report).is_err());

        let mut report = synthetic_accepted_report();
        report.policy_version = 2;
        let verdict = assess_waypoint_v2_acceptance(&report).unwrap();
        assert_eq!(verdict.status, WaypointV2AcceptanceStatus::NotAccepted);
        assert!(!verdict.passed);
        assert_eq!(verdict.issues[0].code, "policy_not_accepted");
    }

    #[test]
    fn unchanged_capture_provenance_requires_matching_recorded_source_states() {
        let mut report = synthetic_accepted_report();
        report.provenance.source_after = None;
        assert!(
            assess_waypoint_v2_acceptance(&report)
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "capture_source_drift")
        );

        let mut report = synthetic_accepted_report();
        report.provenance.source_after = Some(source_state("different-after"));
        assert!(
            assess_waypoint_v2_acceptance(&report)
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "capture_source_drift")
        );

        let mut report = synthetic_accepted_report();
        report.provenance.unchanged_during_capture = Some(false);
        assert!(
            assess_waypoint_v2_acceptance(&report)
                .unwrap()
                .issues
                .iter()
                .any(|issue| issue.code == "capture_source_drift")
        );
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "pd-v2-acceptance-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn write_synthetic_capture(capture: &Path) -> Result<()> {
        use crate::waypoint_v2_pack::WaypointV2BatchReport;
        use crate::{WaypointV2FlightResult, WaypointV2Timings};

        let (inputs, input_identity) = check_default_planner_v2_binding()?;
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("workspace root")?;
        let pack_bytes = fs::read(repo.join(DEFAULT_PLANNER_PACK_PATH))?;
        let expanded_bytes = serde_json::to_vec_pretty(&inputs)?;
        fs::create_dir_all(capture.join("runs"))?;
        fs::write(capture.join("pack.json"), &pack_bytes)?;
        fs::write(capture.join("expanded-inputs.json"), &expanded_bytes)?;

        let mut cases = Vec::with_capacity(inputs.len());
        for input in &inputs {
            let unsupported = input.expected_preflight.as_deref() == Some("unsupported");
            let stop = if unsupported {
                pd_plan::waypoint_v2::WaypointV2Stop::Unsupported
            } else {
                pd_plan::waypoint_v2::WaypointV2Stop::NoNominal
            };
            let stop_text = serde_json::to_value(stop)?.as_str().unwrap().to_owned();
            let input_identity = format!("synthetic-check-{}", input.case_id);
            let result = WaypointV2FlightResult {
                policy: pd_plan::waypoint_v2::WaypointV2Policy::revision_3(),
                input_identity: input_identity.clone(),
                planning_stop: stop,
                reason: None,
                correction_count: match input.group {
                    WaypointV2PackGroup::Clear => 0,
                    WaypointV2PackGroup::Ordinary | WaypointV2PackGroup::AdditionalTerrain => 1,
                    WaypointV2PackGroup::Diagnostic => 0,
                },
                initial_nominal_terrain_blocked: input.group != WaypointV2PackGroup::Clear,
                integrity_passed: true,
                physical_outcome: None,
                mission_outcome: None,
                absolute_deadline_physics_step: None,
                cycles: Vec::new(),
                segments: Vec::new(),
                ordinary_flight: None,
                final_source_replay_passed: false,
                manifest: None,
                failed_local_row: None,
                timings: WaypointV2Timings::default(),
            };
            let scenario_path = format!("runs/{}/scenario.json", input.case_id);
            let flight_path = format!("runs/{}/flight.json", input.case_id);
            let summary_path = format!("runs/{}/summary.json", input.case_id);
            let run_dir = capture.join("runs").join(&input.case_id);
            fs::create_dir(&run_dir)?;
            let scenario_bytes = serde_json::to_vec_pretty(&input.scenario)?;
            let flight_bytes = serde_json::to_vec_pretty(&result)?;
            let run_summary = serde_json::json!({
                "schema_id": "waypoint_v2_flight_summary_v1",
                "input_identity": input_identity,
                "policy": result.policy,
                "result": {
                    "planning_stop": result.planning_stop,
                    "reason": result.reason,
                    "correction_count": result.correction_count,
                    "initial_nominal_terrain_blocked": result.initial_nominal_terrain_blocked,
                    "integrity_passed": result.integrity_passed,
                    "physical_outcome": result.physical_outcome,
                    "mission_outcome": result.mission_outcome,
                    "final_source_replay_passed": result.final_source_replay_passed,
                    "timings": result.timings,
                },
                "run_summary": null,
                "timings": {"output_s": 0.0, "total_s": 0.0},
            });
            let summary_bytes = serde_json::to_vec_pretty(&run_summary)?;
            fs::write(capture.join(&scenario_path), &scenario_bytes)?;
            fs::write(capture.join(&flight_path), &flight_bytes)?;
            fs::write(capture.join(&summary_path), &summary_bytes)?;
            let artifact_sha256 = BTreeMap::from([
                (scenario_path.clone(), sha256_bytes(&scenario_bytes)?),
                (flight_path.clone(), sha256_bytes(&flight_bytes)?),
                (summary_path.clone(), sha256_bytes(&summary_bytes)?),
            ]);
            let mut case = synthetic_case(input, false);
            case.status = "preflight_rejected".into();
            case.outcome = Some(stop_text.clone());
            case.planning_stop = Some(stop_text);
            case.physical_outcome = None;
            case.mission_outcome = None;
            case.final_source_replay_passed = Some(false);
            case.scenario_path = scenario_path;
            case.flight_path = flight_path;
            case.summary_path = summary_path;
            case.rich_report_path = None;
            case.input_path = match input.source_set.as_str() {
                "practical_suite" => {
                    "fixtures/research/waypoint_v2_practical_suite_plan_v1.json".into()
                }
                "fresh_terrain_inputs" => {
                    "fixtures/research/waypoint_v2_fresh_terrain_inputs_v1.json".into()
                }
                _ => unreachable!("frozen pack source"),
            };
            case.artifact_sha256 = artifact_sha256;
            cases.push(case);
        }
        let summary = crate::waypoint_v2_pack::summarize(&cases)?;
        let source_before = source_state("synthetic-capture-source");
        let report = WaypointV2BatchReport {
            schema_id: WAYPOINT_V2_BATCH_SCHEMA_ID.into(),
            pack_id: "planner_v2_lab_suite".into(),
            name: "Synthetic planner V2 capture".into(),
            status: "completed".into(),
            policy_version: 3,
            case_count: cases.len(),
            cases,
            summary,
            pack_snapshot_sha256: sha256_bytes(&pack_bytes)?,
            expanded_inputs_snapshot_sha256: sha256_bytes(&expanded_bytes)?,
            pack_snapshot_path: "pack.json".into(),
            expanded_inputs_snapshot_path: "expanded-inputs.json".into(),
            input_identity,
            provenance: WaypointV2BatchProvenance {
                source_before: source_before.clone(),
                source_after: Some(source_before),
                unchanged_during_capture: Some(true),
            },
        };
        fs::write(
            capture.join("summary.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        Ok(())
    }

    fn snapshot_tree(root: &Path) -> std::io::Result<BTreeMap<PathBuf, Vec<u8>>> {
        fn walk(
            root: &Path,
            current: &Path,
            files: &mut BTreeMap<PathBuf, Vec<u8>>,
        ) -> std::io::Result<()> {
            if !current.exists() {
                return Ok(());
            }
            for entry in fs::read_dir(current)? {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    walk(root, &path, files)?;
                } else {
                    files.insert(path.strip_prefix(root).unwrap().into(), fs::read(path)?);
                }
            }
            Ok(())
        }
        let mut files = BTreeMap::new();
        walk(root, root, &mut files)?;
        Ok(files)
    }

    #[test]
    fn saved_checker_is_read_only_and_failed_publisher_preserves_all_existing_views() {
        let temp = temp_dir("preserve");
        let fake_repo = temp.join("repo");
        let capture = fake_repo.join("outputs/eval/planner_v2_lab_suite/capture-test");
        write_synthetic_capture(&capture).unwrap();
        let capture_before = snapshot_tree(&capture).unwrap();
        let verdict = check_waypoint_v2_acceptance(&capture).unwrap();
        assert!(!verdict.passed);
        assert_eq!(verdict.status, WaypointV2AcceptanceStatus::Failed);
        assert_eq!(snapshot_tree(&capture).unwrap(), capture_before);

        let tracked_repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("workspace root")
            .unwrap();
        let registered_pack = fake_repo.join(DEFAULT_PLANNER_PACK_PATH);
        fs::create_dir_all(registered_pack.parent().unwrap()).unwrap();
        fs::copy(
            tracked_repo.join(DEFAULT_PLANNER_PACK_PATH),
            registered_pack,
        )
        .unwrap();
        let sentinel_paths = [
            fake_repo.join("outputs/eval/planner_v2_lab_suite/current.json"),
            fake_repo.join("outputs/reports/eval/planner_v2_lab_suite/index.html"),
            fake_repo.join("outputs/reports/eval/planner_v2_lab_suite/render.json"),
            fake_repo.join("outputs/reports/eval/index.html"),
            fake_repo.join("outputs/reports/index.html"),
        ];
        for (index, path) in sentinel_paths.iter().enumerate() {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("protected-{index}")).unwrap();
        }
        let output_root = fake_repo.join("outputs");
        let outputs_before = snapshot_tree(&output_root).unwrap();
        let published =
            crate::planner_eval_site::publish_planner_batch(&fake_repo, &capture).unwrap();
        assert!(published.is_none());
        assert_eq!(snapshot_tree(&output_root).unwrap(), outputs_before);
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn corrupt_publisher_capture_fails_before_any_view_write() {
        let temp = temp_dir("corrupt-preserve");
        let fake_repo = temp.join("repo");
        let capture = fake_repo.join("outputs/eval/planner_v2_lab_suite/capture-corrupt");
        fs::create_dir_all(&capture).unwrap();
        fs::write(capture.join("summary.json"), b"not-json").unwrap();

        let tracked_repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("workspace root")
            .unwrap();
        let registered_pack = fake_repo.join(DEFAULT_PLANNER_PACK_PATH);
        fs::create_dir_all(registered_pack.parent().unwrap()).unwrap();
        fs::copy(
            tracked_repo.join(DEFAULT_PLANNER_PACK_PATH),
            registered_pack,
        )
        .unwrap();

        let sentinel_paths = [
            fake_repo.join("outputs/eval/planner_v2_lab_suite/current.json"),
            fake_repo.join("outputs/reports/eval/planner_v2_lab_suite/index.html"),
            fake_repo.join("outputs/reports/eval/planner_v2_lab_suite/render.json"),
            fake_repo.join("outputs/reports/eval/index.html"),
            fake_repo.join("outputs/reports/index.html"),
        ];
        for (index, path) in sentinel_paths.iter().enumerate() {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("protected-{index}")).unwrap();
        }
        let output_root = fake_repo.join("outputs");
        let outputs_before = snapshot_tree(&output_root).unwrap();
        assert!(crate::planner_eval_site::publish_planner_batch(&fake_repo, &capture).is_err());
        assert_eq!(snapshot_tree(&output_root).unwrap(), outputs_before);
        fs::remove_dir_all(temp).unwrap();
    }
}
