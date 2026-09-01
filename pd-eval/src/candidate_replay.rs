//! Versioned data contracts for the R1 planner-candidate replay diagnostic.
//!
//! This module owns the sealed input/configuration/result shapes and the pure
//! fail-closed case diagnosis used by the research-only executor.  The small
//! preparation boundary below resolves and seals input-only R1 material; it
//! deliberately does not execute a controller or inspect any outcome data.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result, anyhow, bail};
use pd_control::ControllerSpec;
use pd_core::{
    RoutePlan, RoutePlanningRequest, RouteTopology, RunContext, ScenarioSpec, validate_route,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    ProgressIntervalDevelopmentCorpusV1, RouteCapabilityInputV1, RouteExecutionEvidence,
    RouteExecutionEvidenceStatus, RouteExecutionResolutionKind, SourceTransitionCadenceParity,
    SourceTransitionDevelopmentInputs, SourceTransitionEvidence, SourceTransitionEvidenceStatus,
    assemble_route_execution_evidence_from_controlled_artifacts,
    assemble_source_transition_evidence_from_controlled_artifacts, canonical_digest,
    compare_source_transition_cadence_parity, route_execution_evidence_digest,
    route_execution_physical_digest, run_source_transition_cadence_pair,
    source_transition_canonical_digest, source_transition_evidence_digest,
    source_transition_physical_digest, source_transition_provenance_for_route_plan,
    source_transition_resolved_input_digest, validate_persisted_route_execution_evidence,
    validate_persisted_source_transition_evidence, with_physics_rate_evidence_overlay,
};

pub const CANDIDATE_REPLAY_SCHEMA_ID: &str = "paired_candidate_replay_v1";
pub const CANDIDATE_REPLAY_SCHEMA_VERSION: u32 = 1;
pub const CANDIDATE_REPLAY_MAX_CANDIDATES: usize = 8;
pub const CANDIDATE_REPLAY_REPEAT_COUNT: usize = 2;
pub const CANDIDATE_REPLAY_PAIRING_ID: &str = "d0_cadence_parity_v1";
pub const CANDIDATE_REPLAY_REASON_SUPPORTED: &str = "supported/contract_complete";
pub const CANDIDATE_REPLAY_REASON_UNSUPPORTED_WAYPOINT_DEADLINE: &str =
    "unsupported/containment/waypoint_deadline";
pub const CANDIDATE_REPLAY_REASON_UNSUPPORTED_DEADLINE: &str =
    CANDIDATE_REPLAY_REASON_UNSUPPORTED_WAYPOINT_DEADLINE;
pub const CANDIDATE_REPLAY_REASON_UNSUPPORTED_CRASH: &str =
    "unsupported/physics/crash_before_completion";
pub const CANDIDATE_REPLAY_REASON_UNKNOWN_DIRECT: &str = "unknown/scope/direct_route";
pub const CANDIDATE_REPLAY_REASON_UNKNOWN_MISSION: &str = "unknown/scope/mission_goal_incompatible";
pub const CANDIDATE_REPLAY_REASON_INVALID_NONDETERMINISTIC: &str =
    "invalid/replay/nondeterministic";

/// Fixed behavior-bearing pairing configuration for R1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateReplayConfigurationV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub pairing_id: String,
    pub max_candidates: usize,
    pub repeat_count: usize,
    pub max_examined_paths: usize,
    pub max_retained_candidates: usize,
    pub cadence_lanes: Vec<String>,
    pub physics_rate_overlay: String,
    pub identity_normalization: String,
}

impl Default for CandidateReplayConfigurationV1 {
    fn default() -> Self {
        Self {
            schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
            schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
            pairing_id: CANDIDATE_REPLAY_PAIRING_ID.to_owned(),
            max_candidates: CANDIDATE_REPLAY_MAX_CANDIDATES,
            repeat_count: CANDIDATE_REPLAY_REPEAT_COUNT,
            max_examined_paths: pd_plan::CANDIDATE_EXPOSURE_MAX_EXAMINED_PATHS,
            max_retained_candidates: pd_plan::CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES,
            cadence_lanes: vec!["ordinary".to_owned(), "physics".to_owned()],
            physics_rate_overlay: "sample_retention_physics_rate_v1".to_owned(),
            identity_normalization: "run_and_artifact_identity_only_v1".to_owned(),
        }
    }
}

impl CandidateReplayConfigurationV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self != &Self::default() {
            return Err("candidate replay configuration does not match R1".to_owned());
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, String> {
        self.validate()?;
        canonical_digest(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateRunIdentityV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub run_id: String,
    pub base_row_id: String,
    pub base_resolved_input_digest: String,
    pub exposure_digest: String,
    pub candidate_rank: usize,
    pub plan_digest: String,
    pub pairing_config_digest: String,
    pub cadence_lane: String,
    pub repeat_index: usize,
    pub identity_digest: String,
}

impl CandidateRunIdentityV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        row_id: &str,
        base_resolved_input_digest: &str,
        exposure_digest: &str,
        candidate_rank: usize,
        plan_digest: &str,
        pairing_config_digest: &str,
        cadence_lane: &str,
        repeat_index: usize,
    ) -> Result<Self, String> {
        if !matches!(cadence_lane, "ordinary" | "physics") {
            return Err("candidate run cadence lane is not supported".to_owned());
        }
        let mut identity = Self {
            schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
            schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
            run_id: format!(
                "{row_id}__r1_rank_{candidate_rank:03}__{cadence_lane}__repeat_{repeat_index:02}"
            ),
            base_row_id: row_id.to_owned(),
            base_resolved_input_digest: base_resolved_input_digest.to_owned(),
            exposure_digest: exposure_digest.to_owned(),
            candidate_rank,
            plan_digest: plan_digest.to_owned(),
            pairing_config_digest: pairing_config_digest.to_owned(),
            cadence_lane: cadence_lane.to_owned(),
            repeat_index,
            identity_digest: String::new(),
        };
        identity.identity_digest = canonical_digest(&identity)?;
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != CANDIDATE_REPLAY_SCHEMA_ID
            || self.schema_version != CANDIDATE_REPLAY_SCHEMA_VERSION
        {
            return Err("candidate run identity schema mismatch".to_owned());
        }
        if self.base_row_id.trim().is_empty()
            || self.base_resolved_input_digest.trim().is_empty()
            || self.exposure_digest.trim().is_empty()
            || self.plan_digest.trim().is_empty()
            || self.pairing_config_digest.trim().is_empty()
            || !matches!(self.cadence_lane.as_str(), "ordinary" | "physics")
        {
            return Err("candidate run identity has an empty or unknown field".to_owned());
        }
        if self.candidate_rank >= CANDIDATE_REPLAY_MAX_CANDIDATES
            || self.repeat_index >= CANDIDATE_REPLAY_REPEAT_COUNT
        {
            return Err("candidate run identity is outside the R1 bounds".to_owned());
        }
        let expected_pairing_config_digest = CandidateReplayConfigurationV1::default().digest()?;
        if self.pairing_config_digest != expected_pairing_config_digest {
            return Err("candidate run identity pairing configuration mismatch".to_owned());
        }
        let expected_run_id = format!(
            "{}__r1_rank_{:03}__{}__repeat_{:02}",
            self.base_row_id, self.candidate_rank, self.cadence_lane, self.repeat_index
        );
        if self.run_id != expected_run_id {
            return Err("candidate run identity run_id is not canonical".to_owned());
        }
        let mut material = self.clone();
        material.identity_digest.clear();
        if self.identity_digest.is_empty() || self.identity_digest != canonical_digest(&material)? {
            return Err("candidate run identity digest mismatch".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReplayDecisionV1 {
    Supported,
    Unsupported,
    Unknown,
    Invalid,
}

impl CandidateReplayDecisionV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
            Self::Invalid => "invalid",
        }
    }

    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Supported)
    }

    pub const fn is_unsupported(self) -> bool {
        matches!(self, Self::Unsupported)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReplayCaseDiagnosisV1 {
    SelectedPairCompatible,
    ExecutorSelectionGapWitnessed,
    NoExposedPairCompatible,
    UnknownIncompleteCandidateDiagnostic,
    Invalid,
}

impl CandidateReplayCaseDiagnosisV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SelectedPairCompatible => "selected_pair_compatible",
            Self::ExecutorSelectionGapWitnessed => "executor_selection_gap_witnessed",
            Self::NoExposedPairCompatible => "no_exposed_pair_compatible",
            Self::UnknownIncompleteCandidateDiagnostic => "unknown_incomplete_candidate_diagnostic",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateReplayRepeatV1 {
    pub repeat_index: usize,
    pub ordinary_identity: CandidateRunIdentityV1,
    pub physics_identity: CandidateRunIdentityV1,
    /// Resolved input digests include the fresh candidate run identity.  The
    /// ordinary and physics values are kept separately so a replay cannot
    /// silently reuse the base row's resolved provenance.
    pub ordinary_resolved_input_digest: String,
    pub physics_resolved_input_digest: String,
    /// D0 evidence is extracted from the physics-lane run.  These fields copy
    /// the resolved-input provenance carried by its source and route records.
    pub source_provenance_resolved_input_digest: String,
    pub route_provenance_resolved_input_digest: String,
    pub parity: SourceTransitionCadenceParity,
    pub source_evidence_digest: String,
    pub source_physical_digest: String,
    pub route_evidence_digest: String,
    pub route_physical_digest: String,
    pub normalized_source_digest: String,
    pub normalized_route_digest: String,
    pub decision: CandidateReplayDecisionV1,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateReplayCandidateV1 {
    pub rank: usize,
    pub plan_digest: String,
    pub repeats: Vec<CandidateReplayRepeatV1>,
    pub decision: CandidateReplayDecisionV1,
    pub reason: String,
}

impl CandidateReplayCandidateV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.plan_digest.trim().is_empty() {
            return Err(format!("candidate rank {} has no plan digest", self.rank));
        }
        validate_candidate_reason(self.decision, &self.reason)?;
        match self.decision {
            CandidateReplayDecisionV1::Supported | CandidateReplayDecisionV1::Unsupported
                if self.repeats.len() != CANDIDATE_REPLAY_REPEAT_COUNT =>
            {
                return Err(format!("candidate rank {} requires two repeats", self.rank));
            }
            CandidateReplayDecisionV1::Unknown
                if !matches!(self.repeats.len(), 0 | CANDIDATE_REPLAY_REPEAT_COUNT) =>
            {
                return Err(format!(
                    "candidate rank {} has an incomplete unknown repeat set",
                    self.rank
                ));
            }
            CandidateReplayDecisionV1::Invalid
                if self.repeats.len() > CANDIDATE_REPLAY_REPEAT_COUNT =>
            {
                return Err(format!(
                    "candidate rank {} exceeds the invalid repeat bound",
                    self.rank
                ));
            }
            _ => {}
        }
        let mut reference_repeat: Option<&CandidateReplayRepeatV1> = None;
        for (repeat_index, repeat) in self.repeats.iter().enumerate() {
            if repeat.repeat_index != repeat_index
                || repeat.ordinary_identity.repeat_index != repeat_index
                || repeat.physics_identity.repeat_index != repeat_index
                || repeat.ordinary_identity.candidate_rank != self.rank
                || repeat.physics_identity.candidate_rank != self.rank
                || repeat.ordinary_identity.plan_digest != self.plan_digest
                || repeat.physics_identity.plan_digest != self.plan_digest
            {
                return Err(format!(
                    "candidate rank {} repeat identity mismatch",
                    self.rank
                ));
            }
            if repeat.ordinary_identity.cadence_lane != "ordinary"
                || repeat.physics_identity.cadence_lane != "physics"
                || repeat.ordinary_identity.base_row_id != repeat.physics_identity.base_row_id
                || repeat.ordinary_identity.base_resolved_input_digest
                    != repeat.physics_identity.base_resolved_input_digest
                || repeat.ordinary_identity.exposure_digest
                    != repeat.physics_identity.exposure_digest
                || repeat.ordinary_identity.pairing_config_digest
                    != repeat.physics_identity.pairing_config_digest
            {
                return Err(format!(
                    "candidate rank {} paired identity provenance mismatch",
                    self.rank
                ));
            }
            for (name, digest) in [
                (
                    "ordinary_resolved_input_digest",
                    repeat.ordinary_resolved_input_digest.as_str(),
                ),
                (
                    "physics_resolved_input_digest",
                    repeat.physics_resolved_input_digest.as_str(),
                ),
                (
                    "source_provenance_resolved_input_digest",
                    repeat.source_provenance_resolved_input_digest.as_str(),
                ),
                (
                    "route_provenance_resolved_input_digest",
                    repeat.route_provenance_resolved_input_digest.as_str(),
                ),
            ] {
                if digest.trim().is_empty() {
                    return Err(format!(
                        "candidate rank {} repeat {repeat_index} has no {name}",
                        self.rank
                    ));
                }
            }
            if repeat.source_provenance_resolved_input_digest
                != repeat.physics_resolved_input_digest
                || repeat.route_provenance_resolved_input_digest
                    != repeat.physics_resolved_input_digest
                || repeat.ordinary_resolved_input_digest == repeat.physics_resolved_input_digest
            {
                return Err(format!(
                    "candidate rank {} repeat {repeat_index} evidence provenance is not physics-lane consistent",
                    self.rank
                ));
            }
            if repeat.ordinary_resolved_input_digest
                == repeat.ordinary_identity.base_resolved_input_digest
                || repeat.physics_resolved_input_digest
                    == repeat.physics_identity.base_resolved_input_digest
                || repeat.source_provenance_resolved_input_digest
                    == repeat.physics_identity.base_resolved_input_digest
                || repeat.route_provenance_resolved_input_digest
                    == repeat.physics_identity.base_resolved_input_digest
            {
                return Err(format!(
                    "candidate rank {} repeat {repeat_index} reused base resolved-input provenance",
                    self.rank
                ));
            }
            repeat.ordinary_identity.validate()?;
            repeat.physics_identity.validate()?;
            if repeat.source_evidence_digest.trim().is_empty()
                || repeat.source_physical_digest.trim().is_empty()
                || repeat.route_evidence_digest.trim().is_empty()
                || repeat.route_physical_digest.trim().is_empty()
                || repeat.normalized_source_digest.trim().is_empty()
                || repeat.normalized_route_digest.trim().is_empty()
            {
                return Err(format!(
                    "candidate rank {} repeat digests are incomplete",
                    self.rank
                ));
            }
            if self.decision != CandidateReplayDecisionV1::Invalid
                && (repeat.decision != self.decision || repeat.reason != self.reason)
            {
                return Err(format!(
                    "candidate rank {} repeat decision is not deterministic",
                    self.rank
                ));
            }
            if self.reason == CANDIDATE_REPLAY_REASON_INVALID_NONDETERMINISTIC
                && repeat.decision == CandidateReplayDecisionV1::Invalid
            {
                return Err(format!(
                    "candidate rank {} nondeterministic repeat is itself invalid",
                    self.rank
                ));
            }
            let parity_decision = if self.reason == CANDIDATE_REPLAY_REASON_INVALID_NONDETERMINISTIC
            {
                repeat.decision
            } else {
                self.decision
            };
            validate_repeat_parity(parity_decision, &repeat.parity, self.rank)?;
            validate_candidate_reason(repeat.decision, &repeat.reason)?;
            if self.decision != CandidateReplayDecisionV1::Invalid
                && let Some(reference) = reference_repeat
            {
                if repeat.normalized_source_digest != reference.normalized_source_digest
                    || repeat.normalized_route_digest != reference.normalized_route_digest
                    || repeat.source_physical_digest != reference.source_physical_digest
                    || repeat.route_physical_digest != reference.route_physical_digest
                    || repeat.decision != reference.decision
                    || repeat.reason != reference.reason
                {
                    return Err(format!(
                        "candidate rank {} repeats are not deterministic",
                        self.rank
                    ));
                }
            } else {
                reference_repeat = Some(repeat);
            }
        }
        if self.reason == CANDIDATE_REPLAY_REASON_INVALID_NONDETERMINISTIC {
            if self.repeats.len() != CANDIDATE_REPLAY_REPEAT_COUNT {
                return Err(format!(
                    "candidate rank {} nondeterministic result requires two repeats",
                    self.rank
                ));
            }
            let first = &self.repeats[0];
            let second = &self.repeats[1];
            let mismatch = first.normalized_source_digest != second.normalized_source_digest
                || first.normalized_route_digest != second.normalized_route_digest
                || first.source_physical_digest != second.source_physical_digest
                || first.route_physical_digest != second.route_physical_digest
                || first.decision != second.decision
                || first.reason != second.reason;
            if !mismatch {
                return Err(format!(
                    "candidate rank {} nondeterministic result has no observed mismatch",
                    self.rank
                ));
            }
        }
        Ok(())
    }
}

fn validate_repeat_parity(
    decision: CandidateReplayDecisionV1,
    parity: &SourceTransitionCadenceParity,
    rank: usize,
) -> Result<(), String> {
    if parity.passed {
        if !parity.normalized_inputs_equal
            || !parity.actions_equal
            || !parity.controller_updates_equal
            || !parity.events_equal
            || !parity.terminal_step_reason_equal
            || !parity.outcomes_equal
            || !parity.terminal_state_equal
            || !parity.shared_cadence_samples_equal
            || !parity.physics_samples_contiguous
            || !parity.mismatch_reasons.is_empty()
        {
            return Err(format!(
                "candidate rank {rank} has an inconsistent parity record"
            ));
        }
    } else if decision != CandidateReplayDecisionV1::Invalid {
        return Err(format!(
            "candidate rank {rank} has a non-invalid decision with failed parity"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateReplayCaseV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub row_id: String,
    pub corpus: ProgressIntervalDevelopmentCorpusV1,
    pub base_input_digest: String,
    pub base_resolved_input_digest: String,
    pub exposure_digest: String,
    pub selected_plan_digest: String,
    pub pairing_config_digest: String,
    pub exposed_candidate_count: usize,
    pub exposure_complete: bool,
    pub exposure_path_truncated: bool,
    pub exposure_retention_truncated: bool,
    pub evaluation_budget: usize,
    pub evaluated_candidate_count: usize,
    pub supported_candidate_count: usize,
    pub unsupported_candidate_count: usize,
    pub unknown_candidate_count: usize,
    pub invalid_candidate_count: usize,
    pub selected_decision: CandidateReplayDecisionV1,
    pub diagnosis: CandidateReplayCaseDiagnosisV1,
    pub candidates: Vec<CandidateReplayCandidateV1>,
    pub case_digest: String,
}

impl CandidateReplayCaseV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != CANDIDATE_REPLAY_SCHEMA_ID
            || self.schema_version != CANDIDATE_REPLAY_SCHEMA_VERSION
        {
            return Err("candidate replay case schema mismatch".to_owned());
        }
        if self.row_id.trim().is_empty()
            || self.base_input_digest.trim().is_empty()
            || self.base_resolved_input_digest.trim().is_empty()
            || self.exposure_digest.trim().is_empty()
            || self.selected_plan_digest.trim().is_empty()
            || self.pairing_config_digest.trim().is_empty()
        {
            return Err("candidate replay case has an empty identity field".to_owned());
        }
        if self.evaluation_budget != CANDIDATE_REPLAY_MAX_CANDIDATES
            || self.exposed_candidate_count == 0
            || self.exposed_candidate_count > pd_plan::CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES
            || self.evaluated_candidate_count != self.candidates.len()
            || self.evaluated_candidate_count > self.evaluation_budget
            || self.evaluated_candidate_count > self.exposed_candidate_count
        {
            return Err("candidate replay case counts are inconsistent".to_owned());
        }
        if self.exposure_complete == self.exposure_path_truncated
            || (self.exposure_retention_truncated
                && self.exposed_candidate_count
                    != pd_plan::CANDIDATE_EXPOSURE_MAX_RETAINED_CANDIDATES)
        {
            return Err("candidate replay exposure flags are inconsistent".to_owned());
        }
        let expected_pairing_config_digest = CandidateReplayConfigurationV1::default().digest()?;
        if self.pairing_config_digest != expected_pairing_config_digest {
            return Err("candidate replay pairing configuration mismatch".to_owned());
        }
        for (rank, candidate) in self.candidates.iter().enumerate() {
            if candidate.rank != rank {
                return Err("candidate replay ranks are not contiguous".to_owned());
            }
            candidate.validate()?;
            for repeat in &candidate.repeats {
                for identity in [&repeat.ordinary_identity, &repeat.physics_identity] {
                    if identity.base_row_id != self.row_id
                        || identity.base_resolved_input_digest != self.base_resolved_input_digest
                        || identity.exposure_digest != self.exposure_digest
                        || identity.pairing_config_digest != self.pairing_config_digest
                    {
                        return Err(format!(
                            "candidate rank {rank} identity provenance does not match case"
                        ));
                    }
                }
                if repeat.ordinary_resolved_input_digest == self.base_resolved_input_digest
                    || repeat.physics_resolved_input_digest == self.base_resolved_input_digest
                    || repeat.source_provenance_resolved_input_digest
                        == self.base_resolved_input_digest
                    || repeat.route_provenance_resolved_input_digest
                        == self.base_resolved_input_digest
                {
                    return Err(format!(
                        "candidate rank {rank} reused base resolved-input provenance"
                    ));
                }
            }
        }
        let mut plan_digests = std::collections::BTreeSet::new();
        for candidate in &self.candidates {
            if !plan_digests.insert(candidate.plan_digest.clone()) {
                return Err("candidate replay contains duplicate plan digests".to_owned());
            }
        }
        if let Some(selected) = self.candidates.first() {
            if selected.plan_digest != self.selected_plan_digest {
                return Err("selected candidate identity is inconsistent".to_owned());
            }
            if self.selected_decision != selected.decision {
                return Err("selected candidate decision is inconsistent".to_owned());
            }
        } else if !matches!(
            self.selected_decision,
            CandidateReplayDecisionV1::Unknown | CandidateReplayDecisionV1::Invalid
        ) {
            return Err("selected candidate is missing for a terminal decision".to_owned());
        }
        if self.candidates.len() > 1
            && matches!(
                self.selected_decision,
                CandidateReplayDecisionV1::Supported
                    | CandidateReplayDecisionV1::Unknown
                    | CandidateReplayDecisionV1::Invalid
            )
        {
            return Err("candidate replay evaluated after a stopping decision".to_owned());
        }
        for (index, candidate) in self.candidates.iter().enumerate().skip(1) {
            if matches!(
                candidate.decision,
                CandidateReplayDecisionV1::Supported | CandidateReplayDecisionV1::Invalid
            ) && index + 1 != self.candidates.len()
            {
                return Err("candidate replay evaluated after a stopping alternative".to_owned());
            }
        }
        let counts = decision_counts(&self.candidates);
        if [
            self.supported_candidate_count,
            self.unsupported_candidate_count,
            self.unknown_candidate_count,
            self.invalid_candidate_count,
        ] != counts
        {
            return Err("candidate replay decision counts are inconsistent".to_owned());
        }
        validate_case_diagnosis(self)?;
        let mut material = self.clone();
        material.case_digest.clear();
        if self.case_digest.is_empty() || self.case_digest != canonical_digest(&material)? {
            return Err("candidate replay case digest mismatch".to_owned());
        }
        Ok(())
    }

    pub fn seal(&mut self) -> Result<(), String> {
        self.case_digest.clear();
        self.case_digest = canonical_digest(self)?;
        self.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateReplaySummaryV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub pairing_config_digest: String,
    pub case_count: usize,
    pub baseline_count: usize,
    pub diagnostic_count: usize,
    pub complete_case_count: usize,
    pub evaluated_candidate_count: usize,
    pub diagnosis_counts: BTreeMap<String, usize>,
    pub candidate_decision_counts: BTreeMap<String, usize>,
    pub case_digests: Vec<String>,
    pub summary_digest: String,
}

impl CandidateReplaySummaryV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != CANDIDATE_REPLAY_SCHEMA_ID
            || self.schema_version != CANDIDATE_REPLAY_SCHEMA_VERSION
        {
            return Err("candidate replay summary schema mismatch".to_owned());
        }
        if self.source_d0_input_digest.trim().is_empty()
            || self.pairing_config_digest.trim().is_empty()
            || self.case_count != self.case_digests.len()
            || self.baseline_count.checked_add(self.diagnostic_count) != Some(self.case_count)
            || self.complete_case_count > self.case_count
        {
            return Err("candidate replay summary counts are inconsistent".to_owned());
        }
        let expected_pairing_config_digest = CandidateReplayConfigurationV1::default().digest()?;
        if self.pairing_config_digest != expected_pairing_config_digest {
            return Err("candidate replay summary pairing configuration mismatch".to_owned());
        }
        if self
            .case_digests
            .iter()
            .any(|digest| digest.trim().is_empty())
        {
            return Err("candidate replay summary contains an empty case digest".to_owned());
        }
        if self.case_digests.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(
                "candidate replay summary case digests are not sorted and unique".to_owned(),
            );
        }
        let known_diagnoses = [
            CandidateReplayCaseDiagnosisV1::SelectedPairCompatible.as_str(),
            CandidateReplayCaseDiagnosisV1::ExecutorSelectionGapWitnessed.as_str(),
            CandidateReplayCaseDiagnosisV1::NoExposedPairCompatible.as_str(),
            CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic.as_str(),
            CandidateReplayCaseDiagnosisV1::Invalid.as_str(),
        ];
        let diagnosis_total =
            self.diagnosis_counts
                .iter()
                .try_fold(0_usize, |total, (diagnosis, count)| {
                    if !known_diagnoses.contains(&diagnosis.as_str()) {
                        return None;
                    }
                    total.checked_add(*count)
                });
        if diagnosis_total != Some(self.case_count) {
            return Err("candidate replay diagnosis counts are inconsistent".to_owned());
        }
        let known_decisions = [
            CandidateReplayDecisionV1::Supported.as_str(),
            CandidateReplayDecisionV1::Unsupported.as_str(),
            CandidateReplayDecisionV1::Unknown.as_str(),
            CandidateReplayDecisionV1::Invalid.as_str(),
        ];
        if self
            .candidate_decision_counts
            .keys()
            .any(|decision| !known_decisions.contains(&decision.as_str()))
        {
            return Err("candidate replay decision counts contain an unknown key".to_owned());
        }
        let candidate_total = self
            .candidate_decision_counts
            .values()
            .try_fold(0_usize, |total, count| total.checked_add(*count));
        if candidate_total != Some(self.evaluated_candidate_count) {
            return Err("candidate replay evaluated candidate counts are inconsistent".to_owned());
        }
        let mut material = self.clone();
        material.summary_digest.clear();
        if self.summary_digest.is_empty() || self.summary_digest != canonical_digest(&material)? {
            return Err("candidate replay summary digest mismatch".to_owned());
        }
        Ok(())
    }

    pub fn seal(&mut self) -> Result<(), String> {
        self.summary_digest.clear();
        self.summary_digest = canonical_digest(self)?;
        self.validate()
    }
}

/// Input-only R1 preparation.  This is intentionally a complete snapshot of
/// the resolved row, selected planner result, and candidate exposure so the
/// later executor can consume one sealed boundary without reopening a pack or
/// consulting any recorded outcome artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateReplayPreparationV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub row_id: String,
    pub corpus: ProgressIntervalDevelopmentCorpusV1,
    pub scenario: ScenarioSpec,
    pub controller: ControllerSpec,
    pub request: RoutePlanningRequest,
    pub selected_plan: RoutePlan,
    pub capability_input: RouteCapabilityInputV1,
    pub base_input_digest: String,
    pub base_resolved_input_digest: String,
    pub exposure: pd_plan::PlannerCandidateExposureV1,
    pub preparation_digest: String,
}

impl CandidateReplayPreparationV1 {
    /// Validate the complete input-only boundary, including a fresh planner
    /// selection check.  No simulation, controller execution, or outcome data
    /// is read by this method.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != CANDIDATE_REPLAY_SCHEMA_ID
            || self.schema_version != CANDIDATE_REPLAY_SCHEMA_VERSION
        {
            return Err("candidate replay preparation schema mismatch".to_owned());
        }
        if self.source_d0_input_digest.trim().is_empty()
            || self.row_id.trim().is_empty()
            || self.base_input_digest.trim().is_empty()
            || self.base_resolved_input_digest.trim().is_empty()
        {
            return Err("candidate replay preparation has an empty identity field".to_owned());
        }
        self.scenario.validate()?;
        self.request.validate().map_err(|error| error.to_string())?;
        validate_route(&self.request, &self.selected_plan.route)
            .map_err(|error| format!("selected route validation failed: {error}"))?;
        if self.scenario.mission.transfer_route.as_ref() != Some(&self.selected_plan.route) {
            return Err("selected route does not match the scenario mission route".to_owned());
        }
        let expected_request = planning_request(&self.scenario, &self.selected_plan);
        if self.request != expected_request {
            return Err("preparation request does not match the resolved scenario".to_owned());
        }
        let current_plan = pd_plan::plan(&self.request)
            .map_err(|error| format!("current planner selection failed: {error}"))?;
        if self.selected_plan != current_plan {
            return Err("persisted selected plan differs from current plan()".to_owned());
        }
        if self.base_resolved_input_digest
            != source_transition_resolved_input_digest(
                &self.row_id,
                &self.scenario,
                &self.selected_plan,
                &self.controller,
            )
        {
            return Err("base resolved-input digest is not canonical".to_owned());
        }
        self.capability_input.validate()?;
        if self.capability_input.input_digest != self.base_input_digest {
            return Err("base input digest does not match capability input".to_owned());
        }
        if self.capability_input.provenance.request_digest != self.selected_plan.request_digest
            || self.capability_input.provenance.route_plan_digest != self.selected_plan.plan_digest
        {
            return Err("capability input provenance does not match selected plan".to_owned());
        }

        // Rank zero parity alone is insufficient for a persisted preparation:
        // every alternative must still be the exact bounded exposure emitted
        // by the current planner request.  Recompute the additive planner
        // diagnostic at the reload boundary so a self-resealed exposure
        // cannot inject or reorder a candidate.
        let expected_exposure = pd_plan::expose_candidates(&self.request)
            .map_err(|error| format!("current planner exposure failed: {error}"))?;
        expected_exposure.validate()?;
        self.exposure.validate()?;
        if self.exposure != expected_exposure {
            return Err("candidate exposure is not the exact current planner exposure".to_owned());
        }
        let mut material = self.clone();
        material.preparation_digest.clear();
        if self.preparation_digest.is_empty()
            || self.preparation_digest != canonical_digest(&material)?
        {
            return Err("candidate replay preparation digest mismatch".to_owned());
        }
        Ok(())
    }

    pub fn seal(&mut self) -> Result<(), String> {
        self.preparation_digest.clear();
        self.preparation_digest = canonical_digest(self)?;
        self.validate()
    }
}

#[derive(Clone)]
struct CandidateReplayResolvedInput {
    source_d0_input_digest: String,
    row_id: String,
    corpus: ProgressIntervalDevelopmentCorpusV1,
    scenario: ScenarioSpec,
    controller: ControllerSpec,
    selected_plan: RoutePlan,
}

fn planning_request(scenario: &ScenarioSpec, route_plan: &RoutePlan) -> RoutePlanningRequest {
    RoutePlanningRequest {
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        source_pad_id: route_plan.route.source_pad_id.clone(),
        target_pad_id: route_plan.route.target_pad_id.clone(),
        policy: route_plan.policy.clone(),
    }
}

fn select_candidate_replay_input(
    inputs: &SourceTransitionDevelopmentInputs,
    case_id: &str,
) -> Result<CandidateReplayResolvedInput> {
    let mut matches = Vec::new();
    for run in &inputs.baseline_runs {
        if run.descriptor.run_id == case_id {
            let selected_plan =
                run.descriptor.route_plan.clone().ok_or_else(|| {
                    anyhow!("resolved baseline case '{case_id}' has no route plan")
                })?;
            matches.push(CandidateReplayResolvedInput {
                source_d0_input_digest: inputs.manifest.input_digest.clone(),
                row_id: run.descriptor.run_id.clone(),
                corpus: ProgressIntervalDevelopmentCorpusV1::Baseline,
                scenario: run.scenario.clone(),
                controller: run.descriptor.controller_spec.clone(),
                selected_plan,
            });
        }
    }
    for case in &inputs.diagnostic_cases {
        if case.run_id == case_id {
            matches.push(CandidateReplayResolvedInput {
                source_d0_input_digest: inputs.manifest.input_digest.clone(),
                row_id: case.run_id.clone(),
                corpus: ProgressIntervalDevelopmentCorpusV1::Diagnostic,
                scenario: case.scenario.clone(),
                controller: case.controller.clone(),
                selected_plan: case.route_plan.clone(),
            });
        }
    }
    match matches.len() {
        1 => Ok(matches.pop().expect("one matching input exists")),
        0 => {
            bail!("candidate replay case filter did not match exactly one resolved case: {case_id}")
        }
        count => bail!("candidate replay case filter matched {count} resolved cases: {case_id}"),
    }
}

fn candidate_replay_inputs_in_manifest_order(
    inputs: &SourceTransitionDevelopmentInputs,
) -> Result<Vec<CandidateReplayResolvedInput>> {
    let mut resolved =
        Vec::with_capacity(inputs.baseline_runs.len() + inputs.diagnostic_cases.len());
    for run in &inputs.baseline_runs {
        let selected_plan = run.descriptor.route_plan.clone().ok_or_else(|| {
            anyhow!(
                "resolved baseline case '{}' has no route plan",
                run.descriptor.run_id
            )
        })?;
        resolved.push(CandidateReplayResolvedInput {
            source_d0_input_digest: inputs.manifest.input_digest.clone(),
            row_id: run.descriptor.run_id.clone(),
            corpus: ProgressIntervalDevelopmentCorpusV1::Baseline,
            scenario: run.scenario.clone(),
            controller: run.descriptor.controller_spec.clone(),
            selected_plan,
        });
    }
    for case in &inputs.diagnostic_cases {
        resolved.push(CandidateReplayResolvedInput {
            source_d0_input_digest: inputs.manifest.input_digest.clone(),
            row_id: case.run_id.clone(),
            corpus: ProgressIntervalDevelopmentCorpusV1::Diagnostic,
            scenario: case.scenario.clone(),
            controller: case.controller.clone(),
            selected_plan: case.route_plan.clone(),
        });
    }
    Ok(resolved)
}

fn validate_case_id_output_component(case_id: &str) -> Result<()> {
    if case_id.trim().is_empty() {
        bail!("candidate replay case filter must not be empty");
    }
    if case_id == "." || case_id == ".." || case_id.contains('/') || case_id.contains('\\') {
        bail!("candidate replay case ID must be one output-directory path component");
    }
    Ok(())
}

fn candidate_replay_case_dir(output_dir: &Path, case_id: &str) -> Result<std::path::PathBuf> {
    validate_case_id_output_component(case_id)?;
    Ok(output_dir.join("case").join(case_id))
}

fn require_empty_output_dir(output_dir: &Path) -> Result<()> {
    match fs::read_dir(output_dir) {
        Ok(mut entries) => {
            if entries.next().transpose()?.is_some() {
                bail!(
                    "candidate replay output directory must be empty: {}",
                    output_dir.display()
                );
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to inspect candidate replay output directory {}",
                output_dir.display()
            )
        }),
    }
}

fn prepare_candidate_replay_case_from_selected(
    selected: CandidateReplayResolvedInput,
    output_dir: &Path,
    case_id: &str,
) -> Result<CandidateReplayPreparationV1> {
    let request = planning_request(&selected.scenario, &selected.selected_plan);
    request
        .validate()
        .map_err(|error| anyhow!(error.to_string()))?;
    validate_route(&request, &selected.selected_plan.route)
        .map_err(|error| anyhow!("selected route validation failed: {error}"))?;
    let current_plan = pd_plan::plan(&request)
        .map_err(|error| anyhow!("current planner selection failed: {error}"))?;
    if current_plan != selected.selected_plan {
        bail!("persisted selected plan differs from current plan() for {case_id}");
    }
    let capability_input =
        RouteCapabilityInputV1::from_request_and_plan(&request, &selected.selected_plan)
            .map_err(anyhow::Error::msg)?;
    let base_input_digest = capability_input.input_digest.clone();
    let base_resolved_input_digest = source_transition_resolved_input_digest(
        &selected.row_id,
        &selected.scenario,
        &selected.selected_plan,
        &selected.controller,
    );
    let exposure = pd_plan::expose_candidates(&request)
        .map_err(|error| anyhow!("candidate exposure failed: {error}"))?;
    exposure.validate().map_err(anyhow::Error::msg)?;
    if exposure.request_digest != selected.selected_plan.request_digest
        || exposure.selected_plan_digest != selected.selected_plan.plan_digest
        || exposure
            .candidates
            .first()
            .is_none_or(|candidate| candidate.plan != selected.selected_plan)
    {
        bail!("candidate exposure selected plan parity failed for {case_id}");
    }
    let mut preparation = CandidateReplayPreparationV1 {
        schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
        schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
        source_d0_input_digest: selected.source_d0_input_digest,
        row_id: selected.row_id,
        corpus: selected.corpus,
        scenario: selected.scenario,
        controller: selected.controller,
        request,
        selected_plan: selected.selected_plan,
        capability_input,
        base_input_digest,
        base_resolved_input_digest,
        exposure,
        preparation_digest: String::new(),
    };
    preparation.seal().map_err(anyhow::Error::msg)?;
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create candidate replay preparation directory {}",
            output_dir.display()
        )
    })?;
    let exposure_path = output_dir.join("exposure.json");
    let _round_trip_exposure: pd_plan::PlannerCandidateExposureV1 =
        write_json_round_trip(&exposure_path, &preparation.exposure, |value| {
            value.validate().map_err(anyhow::Error::msg)
        })?;
    let preparation_path = output_dir.join("preparation.json");
    let round_trip_preparation: CandidateReplayPreparationV1 =
        write_json_round_trip(&preparation_path, &preparation, |value| {
            value.validate().map_err(anyhow::Error::msg)
        })?;
    Ok(round_trip_preparation)
}

/// Resolve and seal one R1 input row after validating the complete D0
/// development manifest and both input corpora.  Filtering happens only after
/// that full validation.  The output directory contains only input artifacts:
/// no controller/simulator execution or outcome overlay is consulted.
pub fn prepare_candidate_replay_case(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
    case_id: &str,
) -> Result<CandidateReplayPreparationV1> {
    if case_id.trim().is_empty() {
        bail!("candidate replay case filter must not be empty");
    }
    let inputs = crate::resolve_source_transition_development_inputs(manifest_path, repo_root)?;
    let selected = select_candidate_replay_input(&inputs, case_id)?;
    prepare_candidate_replay_case_from_selected(selected, output_dir, case_id)
}

/// Run one bounded R1 candidate-replay case and persist its sealed artifacts.
///
/// The case directory is deliberately owned by the requested row ID.  The
/// complete D0 development inputs are resolved by the preparation boundary
/// before the row filter is applied; execution then starts at rank zero and
/// follows the locked fail-closed stopping rules.  This API is intentionally
/// single-case only.  It cannot accidentally turn a research probe into an
/// unfiltered corpus run.
pub fn run_candidate_replay_case(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
    case_id: &str,
) -> Result<CandidateReplaySummaryV1> {
    validate_case_id_output_component(case_id)?;
    require_empty_output_dir(output_dir)?;
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create candidate replay output directory {}",
            output_dir.display()
        )
    })?;
    let case_dir = candidate_replay_case_dir(output_dir, case_id)?;
    let preparation = prepare_candidate_replay_case(manifest_path, repo_root, &case_dir, case_id)?;
    let case = run_candidate_replay_case_from_preparation(&preparation, &case_dir)?;
    let summary = build_candidate_replay_summary(
        &preparation.source_d0_input_digest,
        std::slice::from_ref(&case),
    )?;
    write_json_round_trip(&output_dir.join("summary.json"), &summary, |value| {
        value.validate().map_err(anyhow::Error::msg)
    })
}

/// Run every validated development input in canonical baseline-then-
/// diagnostic order.  This is an explicit bulk research command; it does not
/// alter the single-case API and publishes the aggregate summary only after
/// every case has completed successfully.
pub fn run_candidate_replay_development(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
) -> Result<CandidateReplaySummaryV1> {
    require_empty_output_dir(output_dir)?;
    let inputs = crate::resolve_source_transition_development_inputs(manifest_path, repo_root)?;
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create candidate replay output directory {}",
            output_dir.display()
        )
    })?;
    let resolved_inputs = candidate_replay_inputs_in_manifest_order(&inputs)?;
    let mut cases = Vec::with_capacity(resolved_inputs.len());
    for selected in resolved_inputs {
        let case_id = selected.row_id.clone();
        let case_dir = candidate_replay_case_dir(output_dir, &case_id)?;
        let preparation =
            prepare_candidate_replay_case_from_selected(selected, &case_dir, &case_id)?;
        if preparation.source_d0_input_digest != inputs.manifest.input_digest {
            bail!("candidate replay bulk preparation source digest drift for {case_id}");
        }
        cases.push(run_candidate_replay_case_from_preparation(
            &preparation,
            &case_dir,
        )?);
    }
    let summary = build_candidate_replay_summary(&inputs.manifest.input_digest, &cases)?;
    write_json_round_trip(&output_dir.join("summary.json"), &summary, |value| {
        value.validate().map_err(anyhow::Error::msg)
    })
}

fn run_candidate_replay_case_from_preparation(
    preparation: &CandidateReplayPreparationV1,
    case_dir: &Path,
) -> Result<CandidateReplayCaseV1> {
    let exposed_candidate_count = preparation.exposure.candidates.len();
    let candidates = evaluate_candidates_in_order(exposed_candidate_count, |rank| {
        let candidate = execute_candidate_replay_candidate(preparation, rank, case_dir)?;
        candidate
            .validate()
            .map_err(|validation| anyhow!("candidate rank {rank} is invalid: {validation}"))?;
        let candidate_dir = case_dir.join("candidate").join(format!("rank_{rank:03}"));
        fs::create_dir_all(&candidate_dir).with_context(|| {
            format!(
                "failed to create candidate replay rank directory {}",
                candidate_dir.display()
            )
        })?;
        write_json_round_trip(&candidate_dir.join("candidate.json"), &candidate, |value| {
            value.validate().map_err(anyhow::Error::msg)
        })
    })?;

    let selected_decision = candidates
        .first()
        .map(|candidate| candidate.decision)
        .ok_or_else(|| anyhow!("candidate replay evaluated no selected candidate"))?;
    let alternatives = candidates
        .iter()
        .skip(1)
        .map(|candidate| candidate.decision)
        .collect::<Vec<_>>();
    let diagnosis = diagnose_candidate_selection(
        selected_decision,
        &alternatives,
        preparation.exposure.complete,
        preparation.exposure.path_truncated,
        preparation.exposure.retention_truncated,
        exposed_candidate_count,
    );
    let counts = decision_counts(&candidates);
    let mut case = CandidateReplayCaseV1 {
        schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
        schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
        row_id: preparation.row_id.clone(),
        corpus: preparation.corpus,
        base_input_digest: preparation.base_input_digest.clone(),
        base_resolved_input_digest: preparation.base_resolved_input_digest.clone(),
        exposure_digest: preparation.exposure.exposure_digest.clone(),
        selected_plan_digest: preparation.selected_plan.plan_digest.clone(),
        pairing_config_digest: CandidateReplayConfigurationV1::default()
            .digest()
            .map_err(anyhow::Error::msg)?,
        exposed_candidate_count,
        exposure_complete: preparation.exposure.complete,
        exposure_path_truncated: preparation.exposure.path_truncated,
        exposure_retention_truncated: preparation.exposure.retention_truncated,
        evaluation_budget: CANDIDATE_REPLAY_MAX_CANDIDATES,
        evaluated_candidate_count: candidates.len(),
        supported_candidate_count: counts[0],
        unsupported_candidate_count: counts[1],
        unknown_candidate_count: counts[2],
        invalid_candidate_count: counts[3],
        selected_decision,
        diagnosis,
        candidates,
        case_digest: String::new(),
    };
    case.seal().map_err(anyhow::Error::msg)?;
    let persisted_case = write_json_round_trip(&case_dir.join("case.json"), &case, |value| {
        value.validate().map_err(anyhow::Error::msg)
    })?;
    validate_candidate_replay_case_bundle(case_dir, preparation, &persisted_case)
}

fn read_json_validate<T, F>(path: &Path, validate: F) -> Result<T>
where
    T: DeserializeOwned + Serialize,
    F: Fn(&T) -> Result<()>,
{
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read persisted artifact {}", path.display()))?;
    let value: T = serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "failed to deserialize persisted artifact {}",
            path.display()
        )
    })?;
    let canonical = serde_json::to_vec_pretty(&value).with_context(|| {
        format!(
            "failed to reserialize persisted artifact {}",
            path.display()
        )
    })?;
    if canonical != bytes {
        bail!(
            "persisted artifact {} is not canonical typed JSON",
            path.display()
        );
    }
    validate(&value).map_err(|error| {
        anyhow!(
            "persisted artifact {} failed validation: {error}",
            path.display()
        )
    })?;
    Ok(value)
}

fn directory_entries(path: &Path) -> Result<BTreeSet<String>> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BTreeSet::new());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read artifact directory {}", path.display()));
        }
    };
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry
            .with_context(|| format!("failed to inspect artifact directory {}", path.display()))?;
        let name = entry.file_name().into_string().map_err(|_| {
            anyhow!(
                "artifact directory {} contains a non-UTF-8 entry",
                path.display()
            )
        })?;
        names.insert(name);
    }
    Ok(names)
}

fn require_directory_entries(
    path: &Path,
    expected: &BTreeSet<String>,
    context: &str,
) -> Result<()> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to inspect {context} directory {}", path.display()))?;
    if !metadata.is_dir() {
        bail!(
            "{context} artifact path is not a directory: {}",
            path.display()
        );
    }
    let actual = directory_entries(path)?;
    if &actual != expected {
        bail!(
            "{context} directory {} entries differ: expected {:?}, found {:?}",
            path.display(),
            expected,
            actual
        );
    }
    Ok(())
}

fn validate_candidate_replay_case_bundle(
    case_dir: &Path,
    expected_preparation: &CandidateReplayPreparationV1,
    expected_case: &CandidateReplayCaseV1,
) -> Result<CandidateReplayCaseV1> {
    expected_preparation
        .validate()
        .map_err(|error| anyhow!("in-memory candidate replay preparation is invalid: {error}"))?;
    expected_case
        .validate()
        .map_err(|error| anyhow!("in-memory candidate replay case is invalid: {error}"))?;

    let persisted_exposure: pd_plan::PlannerCandidateExposureV1 = read_json_validate(
        &case_dir.join("exposure.json"),
        |value: &pd_plan::PlannerCandidateExposureV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    if persisted_exposure != expected_preparation.exposure {
        bail!("persisted candidate exposure differs from the in-memory preparation");
    }

    let persisted_preparation: CandidateReplayPreparationV1 = read_json_validate(
        &case_dir.join("preparation.json"),
        |value: &CandidateReplayPreparationV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    if persisted_preparation != *expected_preparation {
        bail!("persisted candidate preparation differs from the in-memory preparation");
    }
    if persisted_preparation.exposure != persisted_exposure {
        bail!("persisted preparation and exposure artifacts disagree");
    }

    let persisted_case: CandidateReplayCaseV1 = read_json_validate(
        &case_dir.join("case.json"),
        |value: &CandidateReplayCaseV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    if persisted_case != *expected_case {
        bail!("persisted candidate replay case differs from the in-memory case");
    }
    validate_case_preparation_links(&persisted_case, &persisted_preparation)?;

    let top_level_entries = [
        "case.json",
        "candidate",
        "exposure.json",
        "preparation.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    require_directory_entries(case_dir, &top_level_entries, "candidate replay case")?;

    let candidate_root = case_dir.join("candidate");
    let expected_rank_entries = persisted_case
        .candidates
        .iter()
        .map(|candidate| format!("rank_{:03}", candidate.rank))
        .collect::<BTreeSet<_>>();
    require_directory_entries(
        &candidate_root,
        &expected_rank_entries,
        "candidate replay candidate",
    )?;

    for (rank, candidate) in persisted_case.candidates.iter().enumerate() {
        let exposed = persisted_preparation
            .exposure
            .candidates
            .get(rank)
            .ok_or_else(|| anyhow!("candidate rank {rank} is not present in persisted exposure"))?;
        if candidate.plan_digest != exposed.plan_digest
            || candidate.plan_digest != exposed.plan.plan_digest
        {
            bail!("candidate rank {rank} does not link to its exposed plan");
        }
        let rank_dir = candidate_root.join(format!("rank_{rank:03}"));
        let persisted_candidate: CandidateReplayCandidateV1 = read_json_validate(
            &rank_dir.join("candidate.json"),
            |value: &CandidateReplayCandidateV1| value.validate().map_err(anyhow::Error::msg),
        )?;
        if persisted_candidate != *candidate {
            bail!("persisted candidate rank {rank} differs from the case artifact");
        }
        validate_candidate_repeat_bundle(
            &rank_dir,
            &persisted_candidate,
            exposed,
            &persisted_preparation,
        )?;
    }
    Ok(persisted_case)
}

fn validate_case_preparation_links(
    case: &CandidateReplayCaseV1,
    preparation: &CandidateReplayPreparationV1,
) -> Result<()> {
    let exposure = &preparation.exposure;
    if case.row_id != preparation.row_id
        || case.base_input_digest != preparation.base_input_digest
        || case.base_resolved_input_digest != preparation.base_resolved_input_digest
        || case.exposure_digest != exposure.exposure_digest
        || case.selected_plan_digest != preparation.selected_plan.plan_digest
        || case.pairing_config_digest
            != CandidateReplayConfigurationV1::default()
                .digest()
                .map_err(anyhow::Error::msg)?
        || case.exposed_candidate_count != exposure.candidates.len()
        || case.exposure_complete != exposure.complete
        || case.exposure_path_truncated != exposure.path_truncated
        || case.exposure_retention_truncated != exposure.retention_truncated
    {
        bail!("candidate replay case does not link to its preparation artifacts");
    }
    if case.corpus != preparation.corpus {
        bail!("candidate replay case corpus differs from its preparation");
    }
    Ok(())
}

fn expected_candidate_identity(
    preparation: &CandidateReplayPreparationV1,
    candidate: &pd_plan::PlannerCandidateV1,
    rank: usize,
    cadence_lane: &str,
    repeat_index: usize,
) -> Result<CandidateRunIdentityV1> {
    let pairing_config_digest = CandidateReplayConfigurationV1::default()
        .digest()
        .map_err(anyhow::Error::msg)?;
    CandidateRunIdentityV1::new(
        &preparation.row_id,
        &preparation.base_resolved_input_digest,
        &preparation.exposure.exposure_digest,
        rank,
        &candidate.plan_digest,
        &pairing_config_digest,
        cadence_lane,
        repeat_index,
    )
    .map_err(anyhow::Error::msg)
}

fn validate_identity_links(
    identity: &CandidateRunIdentityV1,
    expected: &CandidateRunIdentityV1,
    context: &str,
) -> Result<()> {
    if identity != expected {
        bail!("{context} identity differs from the canonical candidate identity");
    }
    identity
        .validate()
        .map_err(|error| anyhow!("{context} identity is invalid: {error}"))?;
    Ok(())
}

fn candidate_execution_scenarios(
    preparation: &CandidateReplayPreparationV1,
    candidate: &pd_plan::PlannerCandidateV1,
) -> (ScenarioSpec, ScenarioSpec) {
    let mut scenario = preparation.scenario.clone();
    scenario.mission.transfer_route = Some(candidate.plan.route.clone());
    let physics_scenario = with_physics_rate_evidence_overlay(&scenario);
    (scenario, physics_scenario)
}

fn validate_candidate_repeat_bundle(
    rank_dir: &Path,
    candidate: &CandidateReplayCandidateV1,
    exposed: &pd_plan::PlannerCandidateV1,
    preparation: &CandidateReplayPreparationV1,
) -> Result<()> {
    let repeat_root = rank_dir;
    let mut expected_repeat_entries = BTreeSet::from(["candidate.json".to_owned()]);
    for repeat in &candidate.repeats {
        expected_repeat_entries.insert(format!("repeat_{:02}", repeat.repeat_index));
    }
    let repeat_dirs = directory_entries(repeat_root)?
        .into_iter()
        .filter(|name| name.starts_with("repeat_"))
        .collect::<BTreeSet<_>>();
    for name in &repeat_dirs {
        expected_repeat_entries.insert(name.clone());
    }
    require_directory_entries(repeat_root, &expected_repeat_entries, "candidate rank")?;

    let mut repeat_indices = Vec::new();
    for name in repeat_dirs {
        let Some(index) = name
            .strip_prefix("repeat_")
            .filter(|value| value.len() == 2)
            .and_then(|value| value.parse::<usize>().ok())
        else {
            bail!("candidate rank directory contains an invalid repeat directory {name}");
        };
        if index >= CANDIDATE_REPLAY_REPEAT_COUNT {
            bail!("candidate rank directory contains out-of-range repeat {index}");
        }
        repeat_indices.push(index);
    }
    repeat_indices.sort_unstable();

    for repeat in &candidate.repeats {
        let repeat_dir = repeat_root.join(format!("repeat_{:02}", repeat.repeat_index));
        validate_complete_repeat_bundle(repeat, candidate, exposed, preparation, &repeat_dir)?;
    }
    let recorded_count = candidate.repeats.len();
    let unrecorded = repeat_indices
        .iter()
        .copied()
        .filter(|index| *index >= recorded_count)
        .collect::<Vec<_>>();
    if !unrecorded.is_empty() {
        if candidate.decision != CandidateReplayDecisionV1::Invalid
            || unrecorded.len() != 1
            || unrecorded[0] != recorded_count
        {
            bail!("candidate rank has an unrecorded repeat outside a partial invalid attempt");
        }
        validate_partial_repeat_bundle(
            candidate,
            exposed,
            preparation,
            &repeat_root.join(format!("repeat_{:02}", recorded_count)),
            recorded_count,
        )?;
    }
    Ok(())
}

fn validate_complete_repeat_bundle(
    repeat: &CandidateReplayRepeatV1,
    candidate: &CandidateReplayCandidateV1,
    exposed: &pd_plan::PlannerCandidateV1,
    preparation: &CandidateReplayPreparationV1,
    repeat_dir: &Path,
) -> Result<()> {
    let expected_entries = [
        "ordinary_identity.json",
        "parity.json",
        "physics_identity.json",
        "route_evidence.json",
        "source_evidence.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    require_directory_entries(repeat_dir, &expected_entries, "candidate replay repeat")?;

    let ordinary_identity: CandidateRunIdentityV1 = read_json_validate(
        &repeat_dir.join("ordinary_identity.json"),
        |value: &CandidateRunIdentityV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    let physics_identity: CandidateRunIdentityV1 = read_json_validate(
        &repeat_dir.join("physics_identity.json"),
        |value: &CandidateRunIdentityV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    let expected_ordinary = expected_candidate_identity(
        preparation,
        exposed,
        candidate.rank,
        "ordinary",
        repeat.repeat_index,
    )?;
    let expected_physics = expected_candidate_identity(
        preparation,
        exposed,
        candidate.rank,
        "physics",
        repeat.repeat_index,
    )?;
    validate_identity_links(&ordinary_identity, &expected_ordinary, "ordinary repeat")?;
    validate_identity_links(&physics_identity, &expected_physics, "physics repeat")?;
    if ordinary_identity != repeat.ordinary_identity || physics_identity != repeat.physics_identity
    {
        bail!("persisted repeat identities differ from the candidate record");
    }

    let parity: SourceTransitionCadenceParity =
        read_json_validate(&repeat_dir.join("parity.json"), |_| Ok(()))?;
    let parity_decision = if candidate.reason == CANDIDATE_REPLAY_REASON_INVALID_NONDETERMINISTIC {
        repeat.decision
    } else {
        candidate.decision
    };
    validate_repeat_parity(parity_decision, &parity, candidate.rank).map_err(anyhow::Error::msg)?;
    if parity != repeat.parity {
        bail!("persisted repeat parity differs from the candidate record");
    }

    let source: SourceTransitionEvidence = read_json_validate(
        &repeat_dir.join("source_evidence.json"),
        |value: &SourceTransitionEvidence| {
            validate_persisted_source_transition_evidence(value).map_err(anyhow::Error::msg)
        },
    )?;
    let route: RouteExecutionEvidence = read_json_validate(
        &repeat_dir.join("route_evidence.json"),
        |value: &RouteExecutionEvidence| {
            validate_persisted_route_execution_evidence(value).map_err(anyhow::Error::msg)
        },
    )?;
    validate_repeat_evidence_links(
        repeat,
        candidate,
        exposed,
        preparation,
        &ordinary_identity,
        &physics_identity,
        &source,
        &route,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_repeat_evidence_links(
    repeat: &CandidateReplayRepeatV1,
    candidate: &CandidateReplayCandidateV1,
    exposed: &pd_plan::PlannerCandidateV1,
    preparation: &CandidateReplayPreparationV1,
    ordinary_identity: &CandidateRunIdentityV1,
    physics_identity: &CandidateRunIdentityV1,
    source: &SourceTransitionEvidence,
    route: &RouteExecutionEvidence,
) -> Result<()> {
    if source.evidence_digest != repeat.source_evidence_digest
        || source.physical_digest != repeat.source_physical_digest
        || route.evidence_digest != repeat.route_evidence_digest
        || route.physical_digest != repeat.route_physical_digest
    {
        bail!("persisted evidence digest does not match the candidate repeat");
    }
    if source_transition_evidence_digest(source) != source.evidence_digest
        || source_transition_physical_digest(source).map_err(anyhow::Error::msg)?
            != source.physical_digest
        || route_execution_evidence_digest(route) != route.evidence_digest
        || route_execution_physical_digest(route) != route.physical_digest
    {
        bail!("persisted evidence digest recomputation does not match");
    }
    if route.source_transition != *source {
        bail!("route evidence does not contain the persisted source evidence");
    }
    if source.provenance.resolved_input_digest != repeat.physics_resolved_input_digest
        || route.provenance.resolved_input_digest != repeat.physics_resolved_input_digest
        || source.provenance.resolved_input_digest != repeat.source_provenance_resolved_input_digest
        || route.provenance.resolved_input_digest != repeat.route_provenance_resolved_input_digest
    {
        bail!("persisted evidence provenance does not match the candidate repeat");
    }
    if source.provenance.request_digest != exposed.plan.request_digest
        || source.provenance.route_plan_digest != exposed.plan.plan_digest
        || source.provenance.policy_digest
            != source_transition_canonical_digest(&exposed.plan.policy)
    {
        bail!("source evidence provenance does not match the exposed plan");
    }
    if route.provenance.request_digest != source.provenance.request_digest
        || route.provenance.policy_digest != source.provenance.policy_digest
        || route.provenance.route_plan_digest != source.provenance.route_plan_digest
        || route.provenance.source_target_geometry_digest
            != source.provenance.source_target_geometry_digest
        || route.provenance.raw_bundle_digest != source.provenance.raw_bundle_digest
        || route.provenance.samples_digest != source.provenance.samples_digest
        || route.provenance.action_log_digest != source.provenance.action_log_digest
        || route.provenance.source_evidence_digest != source.evidence_digest
    {
        bail!("source and route provenance fields disagree");
    }
    let expected_artifact_identity = format!(
        "{}:{}",
        physics_identity.run_id, physics_identity.identity_digest
    );
    if source.provenance.artifact_identity != expected_artifact_identity
        || route.provenance.artifact_identity != expected_artifact_identity
    {
        bail!("persisted evidence artifact identity is not the physics candidate identity");
    }

    let (scenario, physics_scenario) = candidate_execution_scenarios(preparation, exposed);
    let expected_ordinary_resolved = source_transition_resolved_input_digest(
        &ordinary_identity.run_id,
        &scenario,
        &exposed.plan,
        &preparation.controller,
    );
    let expected_physics_resolved = source_transition_resolved_input_digest(
        &physics_identity.run_id,
        &physics_scenario,
        &exposed.plan,
        &preparation.controller,
    );
    if repeat.ordinary_resolved_input_digest != expected_ordinary_resolved
        || repeat.physics_resolved_input_digest != expected_physics_resolved
    {
        bail!("candidate repeat resolved-input digests are not canonical");
    }

    let normalized_source =
        normalized_source_evidence_digest(source).map_err(anyhow::Error::msg)?;
    let normalized_route = normalized_route_evidence_digest(route).map_err(anyhow::Error::msg)?;
    if repeat.normalized_source_digest != normalized_source
        || repeat.normalized_route_digest != normalized_route
    {
        bail!("candidate repeat normalized evidence digest does not match");
    }
    let (decision, reason) = classify_candidate_evidence(source, route);
    if repeat.decision != decision || repeat.reason != reason {
        bail!("candidate repeat decision is not derived from persisted neutral evidence");
    }
    if candidate.decision != CandidateReplayDecisionV1::Invalid
        && (candidate.decision != repeat.decision || candidate.reason != repeat.reason)
    {
        bail!("candidate decision differs from persisted repeat evidence");
    }
    Ok(())
}

fn validate_partial_repeat_bundle(
    candidate: &CandidateReplayCandidateV1,
    exposed: &pd_plan::PlannerCandidateV1,
    preparation: &CandidateReplayPreparationV1,
    repeat_dir: &Path,
    repeat_index: usize,
) -> Result<()> {
    let entries = directory_entries(repeat_dir)?;
    let allowed = BTreeSet::from([
        "ordinary_identity.json".to_owned(),
        "parity.json".to_owned(),
        "physics_identity.json".to_owned(),
    ]);
    if entries.is_empty() || entries.difference(&allowed).next().is_some() {
        bail!("partial invalid repeat contains unsupported or no artifact files");
    }
    if !entries.contains("ordinary_identity.json") || !entries.contains("physics_identity.json") {
        bail!("partial invalid repeat must contain both candidate identities");
    }
    let ordinary_identity: CandidateRunIdentityV1 = read_json_validate(
        &repeat_dir.join("ordinary_identity.json"),
        |value: &CandidateRunIdentityV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    let physics_identity: CandidateRunIdentityV1 = read_json_validate(
        &repeat_dir.join("physics_identity.json"),
        |value: &CandidateRunIdentityV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    validate_identity_links(
        &ordinary_identity,
        &expected_candidate_identity(
            preparation,
            exposed,
            candidate.rank,
            "ordinary",
            repeat_index,
        )?,
        "partial ordinary repeat",
    )?;
    validate_identity_links(
        &physics_identity,
        &expected_candidate_identity(
            preparation,
            exposed,
            candidate.rank,
            "physics",
            repeat_index,
        )?,
        "partial physics repeat",
    )?;
    if let Some(parity) = entries
        .contains("parity.json")
        .then(|| repeat_dir.join("parity.json"))
    {
        let parity: SourceTransitionCadenceParity = read_json_validate(&parity, |_| Ok(()))?;
        validate_repeat_parity(CandidateReplayDecisionV1::Invalid, &parity, candidate.rank)
            .map_err(anyhow::Error::msg)?;
    }
    Ok(())
}

fn build_candidate_replay_summary(
    source_d0_input_digest: &str,
    cases: &[CandidateReplayCaseV1],
) -> Result<CandidateReplaySummaryV1> {
    if cases.is_empty() {
        bail!("candidate replay bulk evaluation produced no cases");
    }
    let pairing_config_digest = CandidateReplayConfigurationV1::default()
        .digest()
        .map_err(anyhow::Error::msg)?;
    let mut diagnosis_counts = BTreeMap::new();
    for diagnosis in [
        CandidateReplayCaseDiagnosisV1::SelectedPairCompatible,
        CandidateReplayCaseDiagnosisV1::ExecutorSelectionGapWitnessed,
        CandidateReplayCaseDiagnosisV1::NoExposedPairCompatible,
        CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic,
        CandidateReplayCaseDiagnosisV1::Invalid,
    ] {
        diagnosis_counts.insert(diagnosis.as_str().to_owned(), 0);
    }
    let mut candidate_decision_counts = BTreeMap::new();
    for decision in [
        CandidateReplayDecisionV1::Supported,
        CandidateReplayDecisionV1::Unsupported,
        CandidateReplayDecisionV1::Unknown,
        CandidateReplayDecisionV1::Invalid,
    ] {
        candidate_decision_counts.insert(decision.as_str().to_owned(), 0);
    }
    let mut baseline_count = 0;
    let mut diagnostic_count = 0;
    let mut complete_case_count = 0;
    let mut evaluated_candidate_count = 0_usize;
    let mut case_digests = Vec::with_capacity(cases.len());
    for case in cases {
        case.validate().map_err(anyhow::Error::msg)?;
        match case.corpus {
            ProgressIntervalDevelopmentCorpusV1::Baseline => baseline_count += 1,
            ProgressIntervalDevelopmentCorpusV1::Diagnostic => diagnostic_count += 1,
        }
        if !matches!(
            case.diagnosis,
            CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic
                | CandidateReplayCaseDiagnosisV1::Invalid
        ) {
            complete_case_count += 1;
        }
        evaluated_candidate_count = evaluated_candidate_count
            .checked_add(case.evaluated_candidate_count)
            .ok_or_else(|| anyhow!("candidate replay evaluated candidate count overflow"))?;
        *diagnosis_counts
            .get_mut(case.diagnosis.as_str())
            .expect("all case diagnoses are declared above") += 1;
        for candidate in &case.candidates {
            *candidate_decision_counts
                .get_mut(candidate.decision.as_str())
                .expect("all candidate decisions are declared above") += 1;
        }
        case_digests.push(case.case_digest.clone());
    }
    case_digests.sort();
    if case_digests.windows(2).any(|pair| pair[0] == pair[1]) {
        bail!("candidate replay bulk cases contain duplicate case digests");
    }
    let mut summary = CandidateReplaySummaryV1 {
        schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
        schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
        source_d0_input_digest: source_d0_input_digest.to_owned(),
        pairing_config_digest,
        case_count: cases.len(),
        baseline_count,
        diagnostic_count,
        complete_case_count,
        evaluated_candidate_count,
        diagnosis_counts,
        candidate_decision_counts,
        case_digests,
        summary_digest: String::new(),
    };
    summary.seal().map_err(anyhow::Error::msg)?;
    Ok(summary)
}

/// Evaluate candidate ranks in order while keeping execution-specific work in
/// the caller.  This pure sequencing boundary is what enforces R1's stop
/// rules: rank zero is always reached; only an unsupported selection opens
/// the alternative lane; unknown alternatives continue; the first supported
/// or invalid alternative stops the lane.
fn evaluate_candidates_in_order<F>(
    exposed_candidate_count: usize,
    mut evaluate: F,
) -> Result<Vec<CandidateReplayCandidateV1>>
where
    F: FnMut(usize) -> Result<CandidateReplayCandidateV1>,
{
    let candidate_count = exposed_candidate_count.min(CANDIDATE_REPLAY_MAX_CANDIDATES);
    if candidate_count == 0 {
        bail!("candidate replay exposure has no candidates to evaluate");
    }
    let mut candidates = Vec::with_capacity(candidate_count);
    for rank in 0..candidate_count {
        let candidate = evaluate(rank)?;
        if candidate.rank != rank {
            bail!(
                "candidate replay evaluator returned rank {} for requested rank {rank}",
                candidate.rank
            );
        }
        let decision = candidate.decision;
        candidates.push(candidate);
        if rank == 0 {
            if decision != CandidateReplayDecisionV1::Unsupported {
                break;
            }
        } else if matches!(
            decision,
            CandidateReplayDecisionV1::Supported | CandidateReplayDecisionV1::Invalid
        ) {
            break;
        }
    }
    Ok(candidates)
}

/// Execute one already-prepared candidate through the frozen ordinary/physics
/// cadence pair.  This is intentionally a one-candidate unit: case ordering,
/// stopping precedence, and summary aggregation remain the caller's concern.
///
/// The candidate scenario is cloned and its mission route is the only
/// execution input changed.  Direct routes and mission-incompatible waypoint
/// routes are represented as scoped unknowns without invoking the executor.
pub fn execute_candidate_replay_candidate(
    preparation: &CandidateReplayPreparationV1,
    rank: usize,
    output_dir: &Path,
) -> Result<CandidateReplayCandidateV1> {
    if rank >= CANDIDATE_REPLAY_MAX_CANDIDATES {
        bail!("candidate replay rank {rank} is outside the first eight candidates");
    }
    preparation
        .validate()
        .map_err(|error| anyhow!("candidate replay preparation is invalid: {error}"))?;
    let candidate = preparation
        .exposure
        .candidates
        .get(rank)
        .ok_or_else(|| anyhow!("candidate replay rank {rank} is not exposed"))?;
    if candidate.rank != rank || candidate.plan_digest != candidate.plan.plan_digest {
        return Ok(invalid_candidate(
            rank,
            candidate.plan_digest.clone(),
            "invalid/replay/selected_plan_mismatch",
            Vec::new(),
        ));
    }

    if candidate.plan.topology == RouteTopology::Direct || candidate.plan.route.waypoints.is_empty()
    {
        return Ok(scoped_unknown_candidate(
            rank,
            candidate.plan_digest.clone(),
            CANDIDATE_REPLAY_REASON_UNKNOWN_DIRECT,
        ));
    }

    if validate_route(&preparation.request, &candidate.plan.route).is_err() {
        return Ok(invalid_candidate(
            rank,
            candidate.plan_digest.clone(),
            "invalid/replay/route_validation",
            Vec::new(),
        ));
    }

    let mut scenario = preparation.scenario.clone();
    scenario.mission.transfer_route = Some(candidate.plan.route.clone());
    if scenario.validate().is_err() {
        return Ok(scoped_unknown_candidate(
            rank,
            candidate.plan_digest.clone(),
            CANDIDATE_REPLAY_REASON_UNKNOWN_MISSION,
        ));
    }

    let pairing_config_digest = CandidateReplayConfigurationV1::default()
        .digest()
        .map_err(anyhow::Error::msg)?;
    let mut repeats = Vec::with_capacity(CANDIDATE_REPLAY_REPEAT_COUNT);
    for repeat_index in 0..CANDIDATE_REPLAY_REPEAT_COUNT {
        let ordinary_identity = CandidateRunIdentityV1::new(
            &preparation.row_id,
            &preparation.base_resolved_input_digest,
            &preparation.exposure.exposure_digest,
            rank,
            &candidate.plan_digest,
            &pairing_config_digest,
            "ordinary",
            repeat_index,
        )
        .map_err(anyhow::Error::msg)?;
        let physics_identity = CandidateRunIdentityV1::new(
            &preparation.row_id,
            &preparation.base_resolved_input_digest,
            &preparation.exposure.exposure_digest,
            rank,
            &candidate.plan_digest,
            &pairing_config_digest,
            "physics",
            repeat_index,
        )
        .map_err(anyhow::Error::msg)?;
        let ordinary_resolved_input_digest = source_transition_resolved_input_digest(
            &ordinary_identity.run_id,
            &scenario,
            &candidate.plan,
            &preparation.controller,
        );
        let physics_scenario = with_physics_rate_evidence_overlay(&scenario);
        let physics_resolved_input_digest = source_transition_resolved_input_digest(
            &physics_identity.run_id,
            &physics_scenario,
            &candidate.plan,
            &preparation.controller,
        );
        if ordinary_resolved_input_digest == preparation.base_resolved_input_digest
            || physics_resolved_input_digest == preparation.base_resolved_input_digest
            || ordinary_resolved_input_digest == physics_resolved_input_digest
        {
            return Ok(invalid_candidate(
                rank,
                candidate.plan_digest.clone(),
                "invalid/replay/provenance_reuse",
                repeats,
            ));
        }

        let repeat_dir = output_dir
            .join("candidate")
            .join(format!("rank_{rank:03}"))
            .join(format!("repeat_{repeat_index:02}"));
        fs::create_dir_all(&repeat_dir).with_context(|| {
            format!(
                "failed to create candidate replay repeat directory {}",
                repeat_dir.display()
            )
        })?;
        write_json_round_trip(
            &repeat_dir.join("ordinary_identity.json"),
            &ordinary_identity,
            |value| value.validate().map_err(anyhow::Error::msg),
        )?;
        write_json_round_trip(
            &repeat_dir.join("physics_identity.json"),
            &physics_identity,
            |value| value.validate().map_err(anyhow::Error::msg),
        )?;

        let context = match RunContext::from_scenario(&scenario) {
            Ok(context) => context,
            Err(_) => {
                return Ok(invalid_candidate(
                    rank,
                    candidate.plan_digest.clone(),
                    "invalid/replay/scenario_resolution",
                    repeats,
                ));
            }
        };
        let (ordinary, physics) =
            match run_source_transition_cadence_pair(&context, &preparation.controller) {
                Ok(pair) => pair,
                Err(_) => {
                    return Ok(invalid_candidate(
                        rank,
                        candidate.plan_digest.clone(),
                        "invalid/replay/capture_failed",
                        repeats,
                    ));
                }
            };
        let parity = compare_source_transition_cadence_parity(
            &scenario,
            &ordinary,
            &physics_scenario,
            &physics,
        );
        write_json_round_trip(&repeat_dir.join("parity.json"), &parity, |_| Ok(()))?;
        if !parity.passed {
            return Ok(invalid_candidate(
                rank,
                candidate.plan_digest.clone(),
                "invalid/replay/parity_mismatch",
                repeats,
            ));
        }

        let artifact_identity = format!(
            "{}:{}",
            physics_identity.run_id, physics_identity.identity_digest
        );
        let mut provenance = source_transition_provenance_for_route_plan(
            &physics_scenario,
            &candidate.plan,
            &physics.run,
            &physics.controller_updates,
            artifact_identity,
        )
        .map_err(anyhow::Error::msg)?;
        provenance.resolved_input_digest = physics_resolved_input_digest.clone();
        let source = assemble_source_transition_evidence_from_controlled_artifacts(
            &physics_scenario,
            &candidate.plan,
            &physics,
            provenance.clone(),
        );
        let route = assemble_route_execution_evidence_from_controlled_artifacts(
            &physics_scenario,
            &candidate.plan,
            &physics,
            provenance,
        );
        if source.provenance.resolved_input_digest != physics_resolved_input_digest
            || route.provenance.resolved_input_digest != physics_resolved_input_digest
            || route.source_transition != source
        {
            return Ok(invalid_candidate(
                rank,
                candidate.plan_digest.clone(),
                "invalid/replay/provenance_mismatch",
                repeats,
            ));
        }
        if let Err(error) = validate_persisted_source_transition_evidence(&source)
            .map_err(anyhow::Error::msg)
            .and_then(|_| {
                validate_persisted_route_execution_evidence(&route).map_err(anyhow::Error::msg)
            })
        {
            let reason = if source.status == SourceTransitionEvidenceStatus::Invalid
                || route.status == RouteExecutionEvidenceStatus::Invalid
            {
                "invalid/evidence/malformed"
            } else {
                let _ = error;
                "invalid/replay/digest_inconsistency"
            };
            return Ok(invalid_candidate(
                rank,
                candidate.plan_digest.clone(),
                reason,
                repeats,
            ));
        }

        write_json_round_trip(&repeat_dir.join("source_evidence.json"), &source, |value| {
            validate_persisted_source_transition_evidence(value).map_err(anyhow::Error::msg)
        })?;
        write_json_round_trip(&repeat_dir.join("route_evidence.json"), &route, |value| {
            validate_persisted_route_execution_evidence(value).map_err(anyhow::Error::msg)
        })?;

        let (decision, reason) = classify_candidate_evidence(&source, &route);
        let repeat = CandidateReplayRepeatV1 {
            repeat_index,
            ordinary_identity,
            physics_identity,
            ordinary_resolved_input_digest,
            physics_resolved_input_digest,
            source_provenance_resolved_input_digest: source
                .provenance
                .resolved_input_digest
                .clone(),
            route_provenance_resolved_input_digest: route.provenance.resolved_input_digest.clone(),
            parity,
            source_evidence_digest: source.evidence_digest.clone(),
            source_physical_digest: source.physical_digest.clone(),
            route_evidence_digest: route.evidence_digest.clone(),
            route_physical_digest: route.physical_digest.clone(),
            normalized_source_digest: normalized_source_evidence_digest(&source)
                .map_err(anyhow::Error::msg)?,
            normalized_route_digest: normalized_route_evidence_digest(&route)
                .map_err(anyhow::Error::msg)?,
            decision,
            reason,
        };
        repeats.push(repeat);
    }

    let first = repeats
        .first()
        .ok_or_else(|| anyhow!("candidate replay produced no repeats"))?;
    let deterministic = repeats.iter().skip(1).all(|repeat| {
        repeat.normalized_source_digest == first.normalized_source_digest
            && repeat.normalized_route_digest == first.normalized_route_digest
            && repeat.source_physical_digest == first.source_physical_digest
            && repeat.route_physical_digest == first.route_physical_digest
            && repeat.decision == first.decision
            && repeat.reason == first.reason
    });
    let all_repeats_have_decisions = repeats
        .iter()
        .all(|repeat| repeat.decision != CandidateReplayDecisionV1::Invalid);
    let (decision, reason) = if deterministic {
        (first.decision, first.reason.clone())
    } else if all_repeats_have_decisions {
        (
            CandidateReplayDecisionV1::Invalid,
            CANDIDATE_REPLAY_REASON_INVALID_NONDETERMINISTIC.to_owned(),
        )
    } else {
        (
            CandidateReplayDecisionV1::Invalid,
            "invalid/evidence/malformed".to_owned(),
        )
    };
    let result = CandidateReplayCandidateV1 {
        rank,
        plan_digest: candidate.plan_digest.clone(),
        repeats,
        decision,
        reason,
    };
    result.validate().map_err(anyhow::Error::msg)?;
    Ok(result)
}

fn scoped_unknown_candidate(
    rank: usize,
    plan_digest: String,
    reason: &str,
) -> CandidateReplayCandidateV1 {
    CandidateReplayCandidateV1 {
        rank,
        plan_digest,
        repeats: Vec::new(),
        decision: CandidateReplayDecisionV1::Unknown,
        reason: reason.to_owned(),
    }
}

fn invalid_candidate(
    rank: usize,
    plan_digest: String,
    reason: &str,
    repeats: Vec<CandidateReplayRepeatV1>,
) -> CandidateReplayCandidateV1 {
    CandidateReplayCandidateV1 {
        rank,
        plan_digest,
        repeats,
        decision: CandidateReplayDecisionV1::Invalid,
        reason: reason.to_owned(),
    }
}

fn normalized_source_evidence_digest(
    evidence: &SourceTransitionEvidence,
) -> Result<String, String> {
    let mut normalized = evidence.clone();
    normalized.evidence_digest.clear();
    normalized.provenance.resolved_input_digest.clear();
    normalized.provenance.artifact_identity.clear();
    canonical_digest(&normalized)
}

fn normalized_route_evidence_digest(evidence: &RouteExecutionEvidence) -> Result<String, String> {
    let mut normalized = evidence.clone();
    normalized.evidence_digest.clear();
    normalized.provenance.resolved_input_digest.clear();
    normalized.provenance.source_evidence_digest.clear();
    normalized.provenance.artifact_identity.clear();
    normalized.source_transition = {
        let mut source = normalized.source_transition;
        source.evidence_digest.clear();
        source.provenance.resolved_input_digest.clear();
        source.provenance.artifact_identity.clear();
        source
    };
    canonical_digest(&normalized)
}

fn classify_candidate_evidence(
    source: &SourceTransitionEvidence,
    route: &RouteExecutionEvidence,
) -> (CandidateReplayDecisionV1, String) {
    let waypoint_contract_passes = route
        .waypoints
        .iter()
        .map(|waypoint| waypoint.first_contract_pass.is_some())
        .collect::<Vec<_>>();
    let waypoint_resolutions = route
        .waypoints
        .iter()
        .map(|waypoint| {
            waypoint
                .resolution
                .as_ref()
                .map(|resolution| resolution.kind)
        })
        .collect::<Vec<_>>();
    classify_candidate_evidence_parts(
        source.status,
        route.status.clone(),
        &waypoint_contract_passes,
        &waypoint_resolutions,
        route
            .terminal
            .as_ref()
            .map(|terminal| terminal.reason.as_str()),
    )
}

fn classify_candidate_evidence_parts(
    source_status: SourceTransitionEvidenceStatus,
    route_status: RouteExecutionEvidenceStatus,
    waypoint_contract_passes: &[bool],
    waypoint_resolutions: &[Option<RouteExecutionResolutionKind>],
    terminal_reason: Option<&str>,
) -> (CandidateReplayDecisionV1, String) {
    if source_status == SourceTransitionEvidenceStatus::Invalid
        || route_status == RouteExecutionEvidenceStatus::Invalid
    {
        return (
            CandidateReplayDecisionV1::Invalid,
            "invalid/evidence/malformed".to_owned(),
        );
    }
    if terminal_reason == Some("crash") && route_status != RouteExecutionEvidenceStatus::Complete {
        return (
            CandidateReplayDecisionV1::Unsupported,
            CANDIDATE_REPLAY_REASON_UNSUPPORTED_CRASH.to_owned(),
        );
    }
    if source_status != SourceTransitionEvidenceStatus::Complete {
        return (
            CandidateReplayDecisionV1::Unknown,
            "unknown/coverage/source_censored".to_owned(),
        );
    }
    match route_status {
        RouteExecutionEvidenceStatus::Complete => {
            if waypoint_contract_passes.len() != waypoint_resolutions.len() {
                return (
                    CandidateReplayDecisionV1::Invalid,
                    "invalid/evidence/waypoint_count_mismatch".to_owned(),
                );
            }
            let mut saw_deadline = false;
            for (has_contract_pass, resolution) in
                waypoint_contract_passes.iter().zip(waypoint_resolutions)
            {
                if *has_contract_pass {
                    continue;
                } else if matches!(
                    resolution,
                    Some(
                        RouteExecutionResolutionKind::InitialDeadline
                            | RouteExecutionResolutionKind::Deadline
                    )
                ) {
                    saw_deadline = true;
                } else {
                    return (
                        CandidateReplayDecisionV1::Invalid,
                        "invalid/evidence/missing_resolution".to_owned(),
                    );
                }
            }
            if saw_deadline {
                (
                    CandidateReplayDecisionV1::Unsupported,
                    CANDIDATE_REPLAY_REASON_UNSUPPORTED_WAYPOINT_DEADLINE.to_owned(),
                )
            } else {
                (
                    CandidateReplayDecisionV1::Supported,
                    CANDIDATE_REPLAY_REASON_SUPPORTED.to_owned(),
                )
            }
        }
        RouteExecutionEvidenceStatus::CensoredBeforeWaypoint { .. } => (
            CandidateReplayDecisionV1::Unknown,
            "unknown/coverage/route_censored".to_owned(),
        ),
        RouteExecutionEvidenceStatus::Invalid => (
            CandidateReplayDecisionV1::Invalid,
            "invalid/evidence/malformed".to_owned(),
        ),
    }
}

fn write_json_round_trip<T, F>(path: &Path, value: &T, validate: F) -> Result<T>
where
    T: Serialize + DeserializeOwned + PartialEq,
    F: Fn(&T) -> Result<()>,
{
    let bytes = serde_json::to_vec_pretty(value)
        .with_context(|| format!("failed to serialize input artifact {}", path.display()))?;
    let round_trip: T = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to round-trip input artifact {}", path.display()))?;
    if &round_trip != value {
        let left = serde_json::to_value(value)?;
        let right = serde_json::to_value(&round_trip)?;
        bail!(
            "input artifact changes on JSON round-trip at {} ({})",
            path.display(),
            first_json_difference(&left, &right, "$"),
        );
    }
    validate(&round_trip).map_err(|error| {
        anyhow!(
            "round-tripped input artifact {} failed validation: {error}",
            path.display()
        )
    })?;
    fs::write(path, bytes)
        .with_context(|| format!("failed to write input artifact {}", path.display()))?;
    Ok(round_trip)
}

fn first_json_difference(
    left: &serde_json::Value,
    right: &serde_json::Value,
    path: &str,
) -> String {
    if left == right {
        return "no difference".to_owned();
    }
    match (left, right) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            for key in left.keys().chain(right.keys()) {
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => {
                        let difference =
                            first_json_difference(left, right, &format!("{path}.{key}"));
                        if difference != "no difference" {
                            return difference;
                        }
                    }
                    _ => return format!("{path}.{key}"),
                }
            }
            path.to_owned()
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                let difference = first_json_difference(left, right, &format!("{path}[{index}]"));
                if difference != "no difference" {
                    return difference;
                }
            }
            format!("{path}.length")
        }
        _ => format!("{path}: {left} != {right}"),
    }
}

/// Apply the locked fail-closed precedence to a selected result and any
/// alternatives actually reached within the evaluation budget.
pub fn diagnose_candidate_selection(
    selected: CandidateReplayDecisionV1,
    alternatives: &[CandidateReplayDecisionV1],
    exposure_complete: bool,
    exposure_path_truncated: bool,
    exposure_retention_truncated: bool,
    exposed_candidate_count: usize,
) -> CandidateReplayCaseDiagnosisV1 {
    match selected {
        CandidateReplayDecisionV1::Invalid => CandidateReplayCaseDiagnosisV1::Invalid,
        CandidateReplayDecisionV1::Supported => {
            CandidateReplayCaseDiagnosisV1::SelectedPairCompatible
        }
        CandidateReplayDecisionV1::Unknown => {
            CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic
        }
        CandidateReplayDecisionV1::Unsupported => {
            for alternative in alternatives {
                match alternative {
                    CandidateReplayDecisionV1::Invalid => {
                        return CandidateReplayCaseDiagnosisV1::Invalid;
                    }
                    CandidateReplayDecisionV1::Supported => {
                        return CandidateReplayCaseDiagnosisV1::ExecutorSelectionGapWitnessed;
                    }
                    CandidateReplayDecisionV1::Unknown | CandidateReplayDecisionV1::Unsupported => {
                    }
                }
            }
            if alternatives
                .iter()
                .all(|decision| decision.is_unsupported())
                && exposure_complete
                && !exposure_path_truncated
                && !exposure_retention_truncated
                && exposed_candidate_count <= CANDIDATE_REPLAY_MAX_CANDIDATES
                && alternatives.len() + 1 == exposed_candidate_count
            {
                CandidateReplayCaseDiagnosisV1::NoExposedPairCompatible
            } else {
                CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic
            }
        }
    }
}

fn validate_candidate_reason(
    decision: CandidateReplayDecisionV1,
    reason: &str,
) -> Result<(), String> {
    let valid = match decision {
        CandidateReplayDecisionV1::Supported => reason == CANDIDATE_REPLAY_REASON_SUPPORTED,
        CandidateReplayDecisionV1::Unsupported => {
            matches!(
                reason,
                CANDIDATE_REPLAY_REASON_UNSUPPORTED_DEADLINE
                    | CANDIDATE_REPLAY_REASON_UNSUPPORTED_CRASH
            )
        }
        CandidateReplayDecisionV1::Unknown => {
            reason == CANDIDATE_REPLAY_REASON_UNKNOWN_DIRECT
                || reason.starts_with(CANDIDATE_REPLAY_REASON_UNKNOWN_MISSION)
                || reason.starts_with("unknown/coverage/")
        }
        CandidateReplayDecisionV1::Invalid => reason.starts_with("invalid/"),
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "reason '{reason}' does not match candidate decision"
        ))
    }
}

fn decision_counts(candidates: &[CandidateReplayCandidateV1]) -> [usize; 4] {
    candidates.iter().fold([0; 4], |mut counts, candidate| {
        match candidate.decision {
            CandidateReplayDecisionV1::Supported => counts[0] += 1,
            CandidateReplayDecisionV1::Unsupported => counts[1] += 1,
            CandidateReplayDecisionV1::Unknown => counts[2] += 1,
            CandidateReplayDecisionV1::Invalid => counts[3] += 1,
        }
        counts
    })
}

fn validate_case_diagnosis(case: &CandidateReplayCaseV1) -> Result<(), String> {
    let alternatives = case
        .candidates
        .iter()
        .skip(1)
        .map(|candidate| candidate.decision)
        .collect::<Vec<_>>();
    let expected = diagnose_candidate_selection(
        case.selected_decision,
        &alternatives,
        case.exposure_complete,
        case.exposure_path_truncated,
        case.exposure_retention_truncated,
        case.exposed_candidate_count,
    );
    if case.diagnosis != expected {
        return Err("candidate replay case diagnosis violates precedence".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_for(
        rank: usize,
        repeat: usize,
        lane: &str,
        plan_digest: &str,
    ) -> CandidateRunIdentityV1 {
        let pairing_config_digest = CandidateReplayConfigurationV1::default().digest().unwrap();
        CandidateRunIdentityV1::new(
            "row",
            "base-resolved",
            "exposure",
            rank,
            plan_digest,
            &pairing_config_digest,
            lane,
            repeat,
        )
        .unwrap()
    }

    fn identity(repeat: usize, lane: &str) -> CandidateRunIdentityV1 {
        identity_for(0, repeat, lane, "plan-0")
    }

    fn passing_parity() -> SourceTransitionCadenceParity {
        SourceTransitionCadenceParity {
            passed: true,
            ordinary_sample_hz: Some(10),
            physics_sample_hz: Some(120),
            normalized_inputs_equal: true,
            actions_equal: true,
            controller_updates_equal: true,
            events_equal: true,
            terminal_step_reason_equal: true,
            outcomes_equal: true,
            terminal_state_equal: true,
            shared_cadence_samples_equal: true,
            physics_samples_contiguous: true,
            ordinary_shared_sample_steps: Vec::new(),
            mismatch_reasons: Vec::new(),
        }
    }

    fn failed_parity() -> SourceTransitionCadenceParity {
        SourceTransitionCadenceParity {
            mismatch_reasons: vec!["actions_differ".to_owned()],
            ..SourceTransitionCadenceParity::default()
        }
    }

    fn repeat(
        rank: usize,
        plan_digest: &str,
        repeat_index: usize,
        decision: CandidateReplayDecisionV1,
        reason: &str,
        parity: SourceTransitionCadenceParity,
    ) -> CandidateReplayRepeatV1 {
        let ordinary_resolved_input_digest = format!("ordinary-candidate-resolved-{repeat_index}");
        let physics_resolved_input_digest = format!("physics-candidate-resolved-{repeat_index}");
        CandidateReplayRepeatV1 {
            repeat_index,
            ordinary_identity: identity_for(rank, repeat_index, "ordinary", plan_digest),
            physics_identity: identity_for(rank, repeat_index, "physics", plan_digest),
            ordinary_resolved_input_digest,
            physics_resolved_input_digest: physics_resolved_input_digest.clone(),
            source_provenance_resolved_input_digest: physics_resolved_input_digest.clone(),
            route_provenance_resolved_input_digest: physics_resolved_input_digest,
            parity,
            source_evidence_digest: format!("source-evidence-{repeat_index}"),
            source_physical_digest: "source-physical".to_owned(),
            route_evidence_digest: format!("route-evidence-{repeat_index}"),
            route_physical_digest: "route-physical".to_owned(),
            normalized_source_digest: "normalized-source".to_owned(),
            normalized_route_digest: "normalized-route".to_owned(),
            decision,
            reason: reason.to_owned(),
        }
    }

    fn make_candidate(
        rank: usize,
        decision: CandidateReplayDecisionV1,
    ) -> CandidateReplayCandidateV1 {
        let plan_digest = format!("plan-{rank}");
        let reason = match decision {
            CandidateReplayDecisionV1::Supported => CANDIDATE_REPLAY_REASON_SUPPORTED,
            CandidateReplayDecisionV1::Unsupported => CANDIDATE_REPLAY_REASON_UNSUPPORTED_DEADLINE,
            CandidateReplayDecisionV1::Unknown => CANDIDATE_REPLAY_REASON_UNKNOWN_MISSION,
            CandidateReplayDecisionV1::Invalid => "invalid/replay/malformed_evidence",
        };
        let repeats = if decision == CandidateReplayDecisionV1::Invalid {
            vec![repeat(
                rank,
                &plan_digest,
                0,
                decision,
                reason,
                failed_parity(),
            )]
        } else {
            (0..CANDIDATE_REPLAY_REPEAT_COUNT)
                .map(|repeat_index| {
                    repeat(
                        rank,
                        &plan_digest,
                        repeat_index,
                        decision,
                        reason,
                        passing_parity(),
                    )
                })
                .collect()
        };
        CandidateReplayCandidateV1 {
            rank,
            plan_digest,
            repeats,
            decision,
            reason: reason.to_owned(),
        }
    }

    fn case_with_candidates(
        candidates: Vec<CandidateReplayCandidateV1>,
        exposed_candidate_count: usize,
        exposure_complete: bool,
        exposure_path_truncated: bool,
        exposure_retention_truncated: bool,
    ) -> CandidateReplayCaseV1 {
        let selected_decision = candidates
            .first()
            .map_or(CandidateReplayDecisionV1::Unknown, |candidate| {
                candidate.decision
            });
        let counts = decision_counts(&candidates);
        let config_digest = CandidateReplayConfigurationV1::default().digest().unwrap();
        CandidateReplayCaseV1 {
            schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
            schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
            row_id: "row".to_owned(),
            corpus: ProgressIntervalDevelopmentCorpusV1::Baseline,
            base_input_digest: "input".to_owned(),
            base_resolved_input_digest: "base-resolved".to_owned(),
            exposure_digest: "exposure".to_owned(),
            selected_plan_digest: candidates.first().map_or_else(
                || "plan-0".to_owned(),
                |candidate| candidate.plan_digest.clone(),
            ),
            pairing_config_digest: config_digest,
            exposed_candidate_count,
            exposure_complete,
            exposure_path_truncated,
            exposure_retention_truncated,
            evaluation_budget: CANDIDATE_REPLAY_MAX_CANDIDATES,
            evaluated_candidate_count: candidates.len(),
            supported_candidate_count: counts[0],
            unsupported_candidate_count: counts[1],
            unknown_candidate_count: counts[2],
            invalid_candidate_count: counts[3],
            selected_decision,
            diagnosis: diagnose_candidate_selection(
                selected_decision,
                &candidates
                    .iter()
                    .skip(1)
                    .map(|candidate| candidate.decision)
                    .collect::<Vec<_>>(),
                exposure_complete,
                exposure_path_truncated,
                exposure_retention_truncated,
                exposed_candidate_count,
            ),
            candidates,
            case_digest: String::new(),
        }
    }

    fn source_evidence_for_normalization() -> SourceTransitionEvidence {
        SourceTransitionEvidence {
            schema_version: 1,
            extractor_version: "source_transition_d0a_v1".to_owned(),
            evidence_digest: "source-evidence".to_owned(),
            physical_digest: "source-physical".to_owned(),
            status: SourceTransitionEvidenceStatus::Invalid,
            invalid_reason: Some(crate::SourceTransitionInvalidReason::MissingSamples),
            provenance: Default::default(),
            cadence: crate::SourceTransitionCadence::physics_rate(120),
            source_transition_start_m: 0.0,
            source_transition_end_m: 1.0,
            initial_anchor: None,
            contact_exit: None,
            tracking_entry: None,
            boundary_centerline_references: Vec::new(),
            first_outbound_reference: None,
            samples: Vec::new(),
            pad_departure_extrema: None,
            acquisition_extrema: None,
            boundary_crossings: Vec::new(),
            backtracking_events: Vec::new(),
            reentry_events: Vec::new(),
            terminal: None,
            audit: Default::default(),
        }
    }

    fn route_evidence_for_normalization(
        source_transition: SourceTransitionEvidence,
    ) -> RouteExecutionEvidence {
        RouteExecutionEvidence {
            schema_version: 1,
            extractor_version: "route_execution_d0b_v1".to_owned(),
            evidence_digest: "route-evidence".to_owned(),
            physical_digest: "route-physical".to_owned(),
            status: RouteExecutionEvidenceStatus::Invalid,
            invalid_reason: Some(crate::RouteExecutionInvalidReason::MissingSamples),
            provenance: Default::default(),
            cadence: crate::SourceTransitionCadence::physics_rate(120),
            source_transition,
            selected_centerline_m: Vec::new(),
            samples: Vec::new(),
            legs: Vec::new(),
            waypoints: Vec::new(),
            backtracking_events: Vec::new(),
            reentry_events: Vec::new(),
            terminal: None,
            audit: Default::default(),
        }
    }

    #[test]
    fn candidate_identity_binds_lane_repeat_and_provenance() {
        let ordinary = identity(0, "ordinary");
        let physics = identity(0, "physics");
        let repeat = identity(1, "ordinary");
        assert_ne!(ordinary.identity_digest, physics.identity_digest);
        assert_ne!(ordinary.identity_digest, repeat.identity_digest);
        ordinary.validate().unwrap();
        physics.validate().unwrap();
        repeat.validate().unwrap();
    }

    #[test]
    fn case_digest_and_count_validation_is_sealed() {
        let configuration = CandidateReplayConfigurationV1::default();
        configuration.validate().unwrap();
        let mut case = CandidateReplayCaseV1 {
            schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
            schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
            row_id: "row".to_owned(),
            corpus: ProgressIntervalDevelopmentCorpusV1::Baseline,
            base_input_digest: "input".to_owned(),
            base_resolved_input_digest: "resolved".to_owned(),
            exposure_digest: "exposure".to_owned(),
            selected_plan_digest: "plan".to_owned(),
            pairing_config_digest: configuration.digest().unwrap(),
            exposed_candidate_count: 1,
            exposure_complete: false,
            exposure_path_truncated: true,
            exposure_retention_truncated: false,
            evaluation_budget: CANDIDATE_REPLAY_MAX_CANDIDATES,
            evaluated_candidate_count: 0,
            supported_candidate_count: 0,
            unsupported_candidate_count: 0,
            unknown_candidate_count: 0,
            invalid_candidate_count: 0,
            selected_decision: CandidateReplayDecisionV1::Unknown,
            diagnosis: CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic,
            candidates: Vec::new(),
            case_digest: String::new(),
        };
        case.seal().unwrap();
        case.case_digest.push('x');
        assert!(case.validate().is_err());
    }

    #[test]
    fn selected_result_precedence_stops_before_alternatives() {
        assert_eq!(
            diagnose_candidate_selection(
                CandidateReplayDecisionV1::Supported,
                &[CandidateReplayDecisionV1::Invalid],
                true,
                false,
                false,
                2,
            ),
            CandidateReplayCaseDiagnosisV1::SelectedPairCompatible
        );
        assert_eq!(
            diagnose_candidate_selection(
                CandidateReplayDecisionV1::Unknown,
                &[CandidateReplayDecisionV1::Supported],
                true,
                false,
                false,
                2,
            ),
            CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic
        );
    }

    #[test]
    fn unsupported_result_precedence_and_negative_gate_are_locked() {
        assert_eq!(
            diagnose_candidate_selection(
                CandidateReplayDecisionV1::Unsupported,
                &[CandidateReplayDecisionV1::Invalid],
                true,
                false,
                false,
                2,
            ),
            CandidateReplayCaseDiagnosisV1::Invalid
        );
        assert_eq!(
            diagnose_candidate_selection(
                CandidateReplayDecisionV1::Unsupported,
                &[CandidateReplayDecisionV1::Supported],
                true,
                false,
                false,
                2,
            ),
            CandidateReplayCaseDiagnosisV1::ExecutorSelectionGapWitnessed
        );
        assert_eq!(
            diagnose_candidate_selection(
                CandidateReplayDecisionV1::Unsupported,
                &[CandidateReplayDecisionV1::Unsupported],
                true,
                false,
                false,
                2,
            ),
            CandidateReplayCaseDiagnosisV1::NoExposedPairCompatible
        );
        assert_eq!(
            diagnose_candidate_selection(
                CandidateReplayDecisionV1::Unsupported,
                &[CandidateReplayDecisionV1::Unsupported],
                true,
                true,
                false,
                2,
            ),
            CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic
        );
    }

    #[test]
    fn candidate_rejects_repeat_evidence_or_decision_mismatch() {
        let mut candidate = make_candidate(0, CandidateReplayDecisionV1::Supported);
        candidate.repeats[1].normalized_source_digest = "changed".to_owned();
        assert!(candidate.validate().is_err());

        let mut candidate = make_candidate(0, CandidateReplayDecisionV1::Supported);
        candidate.repeats[1].normalized_route_digest = "changed".to_owned();
        assert!(candidate.validate().is_err());

        let mut candidate = make_candidate(0, CandidateReplayDecisionV1::Supported);
        candidate.repeats[1].source_physical_digest = "changed".to_owned();
        assert!(candidate.validate().is_err());

        let mut candidate = make_candidate(0, CandidateReplayDecisionV1::Supported);
        candidate.repeats[1].route_physical_digest = "changed".to_owned();
        assert!(candidate.validate().is_err());

        let mut candidate = make_candidate(0, CandidateReplayDecisionV1::Supported);
        candidate.repeats[1].reason = "supported/changed".to_owned();
        assert!(candidate.validate().is_err());

        let mut candidate = make_candidate(0, CandidateReplayDecisionV1::Supported);
        candidate.repeats[0].physics_resolved_input_digest =
            candidate.repeats[0].ordinary_resolved_input_digest.clone();
        assert!(candidate.validate().is_err());
    }

    #[test]
    fn normalized_repeat_digests_retain_execution_evidence_identity() {
        let source = source_evidence_for_normalization();
        let source_digest = normalized_source_evidence_digest(&source).unwrap();
        let mut source_identity = source.clone();
        source_identity.provenance.resolved_input_digest = "new-run".to_owned();
        source_identity.provenance.artifact_identity = "new-artifact".to_owned();
        source_identity.evidence_digest = "resealed".to_owned();
        assert_eq!(
            normalized_source_evidence_digest(&source_identity).unwrap(),
            source_digest
        );

        let mut source_bundle = source.clone();
        source_bundle.provenance.raw_bundle_digest = "different-bundle".to_owned();
        assert_ne!(
            normalized_source_evidence_digest(&source_bundle).unwrap(),
            source_digest
        );
        let mut source_audit = source.clone();
        source_audit.audit.controller_updates_digest = "different-audit".to_owned();
        assert_ne!(
            normalized_source_evidence_digest(&source_audit).unwrap(),
            source_digest
        );

        let route = route_evidence_for_normalization(source.clone());
        let route_digest = normalized_route_evidence_digest(&route).unwrap();
        let mut route_identity = route.clone();
        route_identity.provenance.resolved_input_digest = "new-run".to_owned();
        route_identity.provenance.artifact_identity = "new-artifact".to_owned();
        route_identity.provenance.source_evidence_digest = "new-source".to_owned();
        route_identity
            .source_transition
            .provenance
            .resolved_input_digest = "new-run".to_owned();
        route_identity
            .source_transition
            .provenance
            .artifact_identity = "new-artifact".to_owned();
        assert_eq!(
            normalized_route_evidence_digest(&route_identity).unwrap(),
            route_digest
        );

        let mut route_bundle = route.clone();
        route_bundle.provenance.raw_bundle_digest = "different-bundle".to_owned();
        assert_ne!(
            normalized_route_evidence_digest(&route_bundle).unwrap(),
            route_digest
        );
        let mut nested_source_bundle = route;
        nested_source_bundle
            .source_transition
            .provenance
            .raw_bundle_digest = "different-bundle".to_owned();
        assert_ne!(
            normalized_route_evidence_digest(&nested_source_bundle).unwrap(),
            route_digest
        );
    }

    #[test]
    fn nondeterministic_invalid_preserves_two_valid_repeat_observations() {
        let plan_digest = "plan-0";
        let mut candidate = CandidateReplayCandidateV1 {
            rank: 0,
            plan_digest: plan_digest.to_owned(),
            repeats: vec![
                repeat(
                    0,
                    plan_digest,
                    0,
                    CandidateReplayDecisionV1::Supported,
                    CANDIDATE_REPLAY_REASON_SUPPORTED,
                    passing_parity(),
                ),
                repeat(
                    0,
                    plan_digest,
                    1,
                    CandidateReplayDecisionV1::Unsupported,
                    CANDIDATE_REPLAY_REASON_UNSUPPORTED_DEADLINE,
                    passing_parity(),
                ),
            ],
            decision: CandidateReplayDecisionV1::Invalid,
            reason: CANDIDATE_REPLAY_REASON_INVALID_NONDETERMINISTIC.to_owned(),
        };
        candidate.validate().unwrap();

        candidate.repeats[1].decision = CandidateReplayDecisionV1::Supported;
        candidate.repeats[1].reason = CANDIDATE_REPLAY_REASON_SUPPORTED.to_owned();
        assert!(candidate.validate().is_err());
    }

    #[test]
    fn case_rejects_reused_base_resolved_provenance() {
        let mut candidate = make_candidate(0, CandidateReplayDecisionV1::Supported);
        candidate.repeats[0].ordinary_resolved_input_digest = "base-resolved".to_owned();
        let case = case_with_candidates(vec![candidate], 1, true, false, false);
        assert!(case.validate().is_err());
    }

    #[test]
    fn invalid_candidate_may_stop_with_one_partial_repeat() {
        let mut case = case_with_candidates(
            vec![make_candidate(0, CandidateReplayDecisionV1::Invalid)],
            1,
            true,
            false,
            false,
        );
        case.seal().unwrap();
        assert_eq!(case.invalid_candidate_count, 1);
        assert_eq!(case.diagnosis, CandidateReplayCaseDiagnosisV1::Invalid);
    }

    #[test]
    fn alternatives_stop_after_supported_or_invalid_but_unknown_may_continue() {
        let stopped = case_with_candidates(
            vec![
                make_candidate(0, CandidateReplayDecisionV1::Unsupported),
                make_candidate(1, CandidateReplayDecisionV1::Supported),
                make_candidate(2, CandidateReplayDecisionV1::Unsupported),
            ],
            3,
            true,
            false,
            false,
        );
        assert!(stopped.validate().is_err());

        let stopped = case_with_candidates(
            vec![
                make_candidate(0, CandidateReplayDecisionV1::Unsupported),
                make_candidate(1, CandidateReplayDecisionV1::Invalid),
                make_candidate(2, CandidateReplayDecisionV1::Unsupported),
            ],
            3,
            true,
            false,
            false,
        );
        assert!(stopped.validate().is_err());

        let mut valid = case_with_candidates(
            vec![
                make_candidate(0, CandidateReplayDecisionV1::Unsupported),
                make_candidate(1, CandidateReplayDecisionV1::Unknown),
                make_candidate(2, CandidateReplayDecisionV1::Unsupported),
            ],
            3,
            true,
            false,
            false,
        );
        valid.seal().unwrap();
        assert_eq!(
            valid.diagnosis,
            CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic
        );
    }

    #[test]
    fn ordered_evaluation_applies_selected_and_alternative_stop_rules() {
        let mut calls = Vec::new();
        let candidates = evaluate_candidates_in_order(8, |rank| {
            calls.push(rank);
            Ok(make_candidate(rank, CandidateReplayDecisionV1::Supported))
        })
        .unwrap();
        assert_eq!(calls, vec![0]);
        assert_eq!(candidates.len(), 1);

        calls.clear();
        let candidates = evaluate_candidates_in_order(8, |rank| {
            calls.push(rank);
            Ok(make_candidate(rank, CandidateReplayDecisionV1::Unknown))
        })
        .unwrap();
        assert_eq!(calls, vec![0]);
        assert_eq!(candidates.len(), 1);

        calls.clear();
        let candidates = evaluate_candidates_in_order(8, |rank| {
            calls.push(rank);
            let decision = match rank {
                0 => CandidateReplayDecisionV1::Unsupported,
                1 => CandidateReplayDecisionV1::Unknown,
                2 => CandidateReplayDecisionV1::Supported,
                _ => CandidateReplayDecisionV1::Unsupported,
            };
            Ok(make_candidate(rank, decision))
        })
        .unwrap();
        assert_eq!(calls, vec![0, 1, 2]);
        assert_eq!(candidates.len(), 3);

        calls.clear();
        let candidates = evaluate_candidates_in_order(8, |rank| {
            calls.push(rank);
            let decision = if rank == 0 || rank == 1 {
                CandidateReplayDecisionV1::Unsupported
            } else {
                CandidateReplayDecisionV1::Invalid
            };
            Ok(make_candidate(rank, decision))
        })
        .unwrap();
        assert_eq!(calls, vec![0, 1, 2]);
        assert_eq!(candidates.len(), 3);

        calls.clear();
        let candidates = evaluate_candidates_in_order(10, |rank| {
            calls.push(rank);
            Ok(make_candidate(
                rank,
                if rank == 0 {
                    CandidateReplayDecisionV1::Unsupported
                } else {
                    CandidateReplayDecisionV1::Unknown
                },
            ))
        })
        .unwrap();
        assert_eq!(
            calls,
            (0..CANDIDATE_REPLAY_MAX_CANDIDATES).collect::<Vec<_>>()
        );
        assert_eq!(candidates.len(), CANDIDATE_REPLAY_MAX_CANDIDATES);
    }

    #[test]
    fn ordered_evaluation_propagates_executor_errors() {
        let mut calls = Vec::new();
        let error = evaluate_candidates_in_order(8, |rank| {
            calls.push(rank);
            if rank == 0 {
                Err(anyhow!("strict artifact write failed"))
            } else {
                Ok(make_candidate(rank, CandidateReplayDecisionV1::Unsupported))
            }
        })
        .expect_err("execution errors must not be relabeled as scientific invalid results");
        assert_eq!(calls, vec![0]);
        assert!(error.to_string().contains("strict artifact write failed"));
    }

    #[test]
    fn summary_rejects_tampered_evaluated_candidate_total() {
        let config_digest = CandidateReplayConfigurationV1::default().digest().unwrap();
        let mut summary = CandidateReplaySummaryV1 {
            schema_id: CANDIDATE_REPLAY_SCHEMA_ID.to_owned(),
            schema_version: CANDIDATE_REPLAY_SCHEMA_VERSION,
            source_d0_input_digest: "d0-input".to_owned(),
            pairing_config_digest: config_digest,
            case_count: 2,
            baseline_count: 1,
            diagnostic_count: 1,
            complete_case_count: 2,
            evaluated_candidate_count: 2,
            diagnosis_counts: BTreeMap::from([(
                CandidateReplayCaseDiagnosisV1::SelectedPairCompatible
                    .as_str()
                    .to_owned(),
                2,
            )]),
            candidate_decision_counts: BTreeMap::from([(
                CandidateReplayDecisionV1::Supported.as_str().to_owned(),
                2,
            )]),
            case_digests: vec!["case-a".to_owned(), "case-b".to_owned()],
            summary_digest: String::new(),
        };
        summary.seal().unwrap();
        summary.evaluated_candidate_count = 3;
        assert!(summary.seal().is_err());
    }

    #[test]
    fn bulk_summary_aggregates_corpus_case_and_candidate_counts() {
        let mut baseline = case_with_candidates(
            vec![make_candidate(0, CandidateReplayDecisionV1::Supported)],
            1,
            true,
            false,
            false,
        );
        baseline.seal().unwrap();
        let mut diagnostic = case_with_candidates(
            vec![
                make_candidate(0, CandidateReplayDecisionV1::Unsupported),
                make_candidate(1, CandidateReplayDecisionV1::Unknown),
            ],
            3,
            true,
            false,
            false,
        );
        diagnostic.corpus = ProgressIntervalDevelopmentCorpusV1::Diagnostic;
        diagnostic.seal().unwrap();

        let summary = build_candidate_replay_summary("d0-input", &[baseline, diagnostic]).unwrap();
        summary.validate().unwrap();
        assert_eq!(summary.case_count, 2);
        assert_eq!(summary.baseline_count, 1);
        assert_eq!(summary.diagnostic_count, 1);
        assert_eq!(summary.complete_case_count, 1);
        assert_eq!(summary.evaluated_candidate_count, 3);
        assert_eq!(
            summary.diagnosis_counts
                [CandidateReplayCaseDiagnosisV1::SelectedPairCompatible.as_str()],
            1
        );
        assert_eq!(
            summary.diagnosis_counts
                [CandidateReplayCaseDiagnosisV1::UnknownIncompleteCandidateDiagnostic.as_str()],
            1
        );
        assert_eq!(
            summary.candidate_decision_counts[CandidateReplayDecisionV1::Supported.as_str()],
            1
        );
        assert_eq!(
            summary.candidate_decision_counts[CandidateReplayDecisionV1::Unsupported.as_str()],
            1
        );
        assert_eq!(
            summary.candidate_decision_counts[CandidateReplayDecisionV1::Unknown.as_str()],
            1
        );
        assert!(
            summary
                .case_digests
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
    }

    #[test]
    fn neutral_mapping_accepts_complete_contract_passes() {
        let (decision, reason) = classify_candidate_evidence_parts(
            SourceTransitionEvidenceStatus::Complete,
            RouteExecutionEvidenceStatus::Complete,
            &[true, true],
            &[
                Some(RouteExecutionResolutionKind::ContractPass),
                Some(RouteExecutionResolutionKind::ContractPass),
            ],
            Some("checkpoint_satisfied"),
        );
        assert_eq!(decision, CandidateReplayDecisionV1::Supported);
        assert_eq!(reason, CANDIDATE_REPLAY_REASON_SUPPORTED);
    }

    #[test]
    fn neutral_mapping_marks_deadline_without_pass_unsupported() {
        let (decision, reason) = classify_candidate_evidence_parts(
            SourceTransitionEvidenceStatus::Complete,
            RouteExecutionEvidenceStatus::Complete,
            &[true, false],
            &[
                Some(RouteExecutionResolutionKind::ContractPass),
                Some(RouteExecutionResolutionKind::Deadline),
            ],
            Some("checkpoint_failed"),
        );
        assert_eq!(decision, CandidateReplayDecisionV1::Unsupported);
        assert_eq!(
            reason,
            CANDIDATE_REPLAY_REASON_UNSUPPORTED_WAYPOINT_DEADLINE
        );
    }

    #[test]
    fn neutral_mapping_marks_pre_completion_crash_unsupported() {
        let (decision, reason) = classify_candidate_evidence_parts(
            SourceTransitionEvidenceStatus::Complete,
            RouteExecutionEvidenceStatus::CensoredBeforeWaypoint { waypoint_index: 0 },
            &[],
            &[],
            Some("crash"),
        );
        assert_eq!(decision, CandidateReplayDecisionV1::Unsupported);
        assert_eq!(reason, CANDIDATE_REPLAY_REASON_UNSUPPORTED_CRASH);
    }

    #[test]
    fn neutral_mapping_marks_censored_pre_completion_crash_unsupported() {
        let (decision, reason) = classify_candidate_evidence_parts(
            SourceTransitionEvidenceStatus::CensoredBeforeTrackingEntry,
            RouteExecutionEvidenceStatus::CensoredBeforeWaypoint { waypoint_index: 0 },
            &[],
            &[],
            Some("crash"),
        );
        assert_eq!(decision, CandidateReplayDecisionV1::Unsupported);
        assert_eq!(reason, CANDIDATE_REPLAY_REASON_UNSUPPORTED_CRASH);
    }

    #[test]
    fn neutral_mapping_accepts_initial_deadline_with_contract_pass() {
        let (decision, reason) = classify_candidate_evidence_parts(
            SourceTransitionEvidenceStatus::Complete,
            RouteExecutionEvidenceStatus::Complete,
            &[true],
            &[Some(RouteExecutionResolutionKind::InitialDeadline)],
            Some("crash"),
        );
        assert_eq!(decision, CandidateReplayDecisionV1::Supported);
        assert_eq!(reason, CANDIDATE_REPLAY_REASON_SUPPORTED);
    }

    #[test]
    fn neutral_mapping_keeps_nondecisive_censor_unknown() {
        let (decision, reason) = classify_candidate_evidence_parts(
            SourceTransitionEvidenceStatus::CensoredBeforeTrackingEntry,
            RouteExecutionEvidenceStatus::CensoredBeforeWaypoint { waypoint_index: 0 },
            &[],
            &[],
            Some("max_time_reached"),
        );
        assert_eq!(decision, CandidateReplayDecisionV1::Unknown);
        assert_eq!(reason, "unknown/coverage/source_censored");
    }

    #[test]
    fn neutral_mapping_marks_invalid_evidence_invalid() {
        let (decision, reason) = classify_candidate_evidence_parts(
            SourceTransitionEvidenceStatus::Invalid,
            RouteExecutionEvidenceStatus::Invalid,
            &[],
            &[],
            None,
        );
        assert_eq!(decision, CandidateReplayDecisionV1::Invalid);
        assert_eq!(reason, "invalid/evidence/malformed");
    }

    #[test]
    fn committed_bulk_input_order_is_baseline_then_diagnostic() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval has a repository parent")
            .to_path_buf();
        let manifest_path =
            repo_root.join("fixtures/manifests/source_transition_d0a_development.json");
        let inputs =
            crate::resolve_source_transition_development_inputs(&manifest_path, &repo_root)
                .expect("committed D0 development inputs should resolve");
        let resolved = candidate_replay_inputs_in_manifest_order(&inputs)
            .expect("committed bulk inputs should have route plans");
        assert_eq!(
            resolved.len(),
            inputs.manifest.baseline_expected_case_count
                + inputs.manifest.diagnostic_expected_case_count
        );
        assert!(
            resolved[..inputs.baseline_runs.len()]
                .iter()
                .all(|input| input.corpus == ProgressIntervalDevelopmentCorpusV1::Baseline)
        );
        assert!(
            resolved[inputs.baseline_runs.len()..]
                .iter()
                .all(|input| input.corpus == ProgressIntervalDevelopmentCorpusV1::Diagnostic)
        );
        assert_eq!(
            resolved
                .iter()
                .map(|input| input.row_id.as_str())
                .collect::<Vec<_>>(),
            inputs
                .baseline_runs
                .iter()
                .map(|run| run.descriptor.run_id.as_str())
                .chain(
                    inputs
                        .diagnostic_cases
                        .iter()
                        .map(|case| case.run_id.as_str())
                )
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn committed_case_preparation_is_input_only_and_byte_stable() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval has a repository parent")
            .to_path_buf();
        let manifest_path =
            repo_root.join("fixtures/manifests/source_transition_d0a_development.json");
        let inputs =
            crate::resolve_source_transition_development_inputs(&manifest_path, &repo_root)
                .expect("committed D0 development inputs should resolve");
        let case_id = inputs
            .baseline_runs
            .first()
            .expect("committed baseline should not be empty")
            .descriptor
            .run_id
            .clone();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let output_a = std::env::temp_dir().join(format!("pd_eval_r1_prepare_a_{unique}"));
        let output_b = std::env::temp_dir().join(format!("pd_eval_r1_prepare_b_{unique}"));
        let preparation_a =
            prepare_candidate_replay_case(&manifest_path, &repo_root, &output_a, &case_id)
                .expect("first input-only preparation should succeed");
        let preparation_b =
            prepare_candidate_replay_case(&manifest_path, &repo_root, &output_b, &case_id)
                .expect("second input-only preparation should succeed");
        assert_eq!(preparation_a, preparation_b);
        assert!(!preparation_a.exposure.candidates.is_empty());
        assert_eq!(
            preparation_a.exposure.candidates.first().unwrap().plan,
            preparation_a.selected_plan
        );
        assert!(
            preparation_a
                .exposure
                .candidates
                .windows(2)
                .all(|pair| pair[0].rank < pair[1].rank)
        );
        let plan_digests = preparation_a
            .exposure
            .candidates
            .iter()
            .map(|candidate| candidate.plan_digest.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(plan_digests.len(), preparation_a.exposure.candidates.len());
        for name in ["preparation.json", "exposure.json"] {
            assert_eq!(
                std::fs::read(output_a.join(name)).expect("first artifact should be readable"),
                std::fs::read(output_b.join(name)).expect("second artifact should be readable")
            );
        }
    }

    #[test]
    fn preparation_rejects_resealed_modified_nonzero_exposure_candidate() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval has a repository parent")
            .to_path_buf();
        let manifest_path =
            repo_root.join("fixtures/manifests/source_transition_d0a_development.json");
        let inputs =
            crate::resolve_source_transition_development_inputs(&manifest_path, &repo_root)
                .expect("committed D0 development inputs should resolve");
        let selected = candidate_replay_inputs_in_manifest_order(&inputs)
            .expect("committed bulk inputs should resolve")
            .into_iter()
            .find(|input| {
                let request = planning_request(&input.scenario, &input.selected_plan);
                pd_plan::expose_candidates(&request)
                    .map(|exposure| {
                        exposure.complete
                            && !exposure.retention_truncated
                            && exposure.candidates.len() >= 2
                    })
                    .unwrap_or(false)
            })
            .expect("committed corpus should contain a complete multi-candidate exposure");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("pd_eval_r1_exposure_tamper_{unique}"));
        let preparation =
            prepare_candidate_replay_case_from_selected(selected, &output_dir, "tamper_probe")
                .expect("multi-candidate preparation should succeed");
        assert!(preparation.exposure.candidates.len() >= 2);

        // Omit one nonzero candidate and adjust every structural count before
        // resealing both envelopes.  Structural exposure validation accepts
        // this self-consistent counterfeit; only recomputing the exact current
        // planner exposure can reject it.
        let mut tampered = preparation.clone();
        tampered.exposure.candidates.pop();
        tampered.exposure.accepted_candidate_count -= 1;
        tampered.exposure.retained_candidate_count -= 1;
        tampered.exposure.exposure_digest = pd_plan::candidate_exposure_digest(&tampered.exposure);
        tampered
            .exposure
            .validate()
            .expect("counterfeit exposure should remain structurally valid");
        tampered.preparation_digest.clear();
        tampered.preparation_digest = canonical_digest(&tampered).unwrap();
        let error = tampered
            .validate()
            .expect_err("resealed nonzero candidate tamper must be rejected");
        assert_eq!(
            error,
            "candidate exposure is not the exact current planner exposure"
        );
    }

    #[test]
    fn persisted_candidate_bundle_rejects_resealed_evidence_and_parity_tampering() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval has a repository parent")
            .to_path_buf();
        let manifest_path =
            repo_root.join("fixtures/manifests/source_transition_d0a_development.json");
        let inputs =
            crate::resolve_source_transition_development_inputs(&manifest_path, &repo_root)
                .expect("committed D0 development inputs should resolve");
        let case_id = inputs
            .baseline_runs
            .first()
            .expect("committed baseline should not be empty")
            .descriptor
            .run_id
            .clone();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("pd_eval_r1_bundle_tamper_{unique}"));
        run_candidate_replay_case(&manifest_path, &repo_root, &output_dir, &case_id)
            .expect("committed case should produce a validated bundle");

        let case_dir = output_dir.join("case").join(&case_id);
        let preparation: CandidateReplayPreparationV1 = read_json_validate(
            &case_dir.join("preparation.json"),
            |value: &CandidateReplayPreparationV1| value.validate().map_err(anyhow::Error::msg),
        )
        .expect("persisted preparation should validate");
        let persisted_case: CandidateReplayCaseV1 = read_json_validate(
            &case_dir.join("case.json"),
            |value: &CandidateReplayCaseV1| value.validate().map_err(anyhow::Error::msg),
        )
        .expect("persisted case should validate");
        let repeat_dir = case_dir.join("candidate/rank_000/repeat_00");
        let source_path = repeat_dir.join("source_evidence.json");
        let source_original = std::fs::read(&source_path).expect("source evidence should exist");
        let mut source: SourceTransitionEvidence =
            serde_json::from_slice(&source_original).expect("source evidence should be typed");
        source.provenance.artifact_identity.push_str(":tampered");
        source.evidence_digest = source_transition_evidence_digest(&source);
        validate_persisted_source_transition_evidence(&source)
            .expect("resealed source evidence should remain independently valid");
        std::fs::write(
            &source_path,
            serde_json::to_vec_pretty(&source).expect("tampered source should serialize"),
        )
        .expect("tampered source should be writable");
        let error = validate_candidate_replay_case_bundle(&case_dir, &preparation, &persisted_case)
            .expect_err("resealed source evidence tamper must be rejected");
        assert!(
            error.to_string().contains("evidence"),
            "unexpected error: {error}"
        );
        std::fs::write(&source_path, source_original).expect("source evidence should be restored");

        let parity_path = repeat_dir.join("parity.json");
        let parity_original = std::fs::read(&parity_path).expect("parity should exist");
        let mut parity: SourceTransitionCadenceParity =
            serde_json::from_slice(&parity_original).expect("parity should be typed");
        parity.ordinary_sample_hz = parity.ordinary_sample_hz.map(|rate| rate + 1);
        validate_repeat_parity(
            persisted_case.candidates[0].decision,
            &parity,
            persisted_case.candidates[0].rank,
        )
        .expect("tampered parity should remain independently valid");
        std::fs::write(
            &parity_path,
            serde_json::to_vec_pretty(&parity).expect("tampered parity should serialize"),
        )
        .expect("tampered parity should be writable");
        let error = validate_candidate_replay_case_bundle(&case_dir, &preparation, &persisted_case)
            .expect_err("persisted parity tamper must be rejected");
        assert!(
            error.to_string().contains("parity"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn top_level_candidate_replay_rejects_nonempty_output_roots() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval has a repository parent")
            .to_path_buf();
        let manifest_path =
            repo_root.join("fixtures/manifests/source_transition_d0a_development.json");
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let single_output = std::env::temp_dir().join(format!("pd_eval_r1_stale_single_{unique}"));
        std::fs::create_dir_all(&single_output).expect("stale single output should be created");
        std::fs::write(single_output.join("stale.json"), b"stale").expect("stale marker");
        let error =
            run_candidate_replay_case(&manifest_path, &repo_root, &single_output, "stale_probe")
                .expect_err("single-case runner must reject stale output");
        assert!(
            error.to_string().contains("must be empty"),
            "unexpected error: {error}"
        );

        let bulk_output = std::env::temp_dir().join(format!("pd_eval_r1_stale_bulk_{unique}"));
        std::fs::create_dir_all(&bulk_output).expect("stale bulk output should be created");
        std::fs::write(bulk_output.join("stale.json"), b"stale").expect("stale marker");
        let error = run_candidate_replay_development(&manifest_path, &repo_root, &bulk_output)
            .expect_err("bulk runner must reject stale output");
        assert!(
            error.to_string().contains("must be empty"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn committed_selected_candidate_executes_with_typed_evidence_artifacts() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval has a repository parent")
            .to_path_buf();
        let manifest_path =
            repo_root.join("fixtures/manifests/source_transition_d0a_development.json");
        let inputs =
            crate::resolve_source_transition_development_inputs(&manifest_path, &repo_root)
                .expect("committed D0 development inputs should resolve");
        let case_id = inputs
            .baseline_runs
            .first()
            .expect("committed baseline should not be empty")
            .descriptor
            .run_id
            .clone();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let output_dir = std::env::temp_dir().join(format!("pd_eval_r1_execute_{unique}"));
        let preparation = prepare_candidate_replay_case(
            &manifest_path,
            &repo_root,
            &output_dir.join("preparation"),
            &case_id,
        )
        .expect("committed input-only preparation should succeed");
        let candidate =
            execute_candidate_replay_candidate(&preparation, 0, &output_dir.join("execution"))
                .expect("selected candidate execution should return a sealed result");
        candidate
            .validate()
            .expect("selected candidate should validate");
        assert_eq!(candidate.rank, 0);
        assert_eq!(candidate.repeats.len(), CANDIDATE_REPLAY_REPEAT_COUNT);
        assert!(candidate.repeats.iter().all(|repeat| repeat.parity.passed));
        for repeat_index in 0..CANDIDATE_REPLAY_REPEAT_COUNT {
            let repeat_dir = output_dir
                .join("execution")
                .join("candidate")
                .join("rank_000")
                .join(format!("repeat_{repeat_index:02}"));
            for name in [
                "ordinary_identity.json",
                "physics_identity.json",
                "parity.json",
                "source_evidence.json",
                "route_evidence.json",
            ] {
                assert!(repeat_dir.join(name).is_file(), "missing {name}");
            }
        }
    }
}
