//! W4 already-seen comparison between the input-only bounded physical witness
//! lane and the separately sealed R1 frozen-executor lane.
//!
//! The physical runner in this module has no R1 result path.  The comparison
//! runner loads and exactly validates the complete physical root before it
//! opens the R1 root, preserving the pre-run sealing boundary.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result, anyhow, bail};
use rayon::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    BoundedTrajectoryArtifactStatusV1, BoundedTrajectoryDecisionV1,
    BoundedTrajectoryProposalConfigurationV1, BoundedTrajectoryWitnessConfigurationV1,
    BoundedTrajectoryWitnessPredictionV1, CandidateReplayCaseDiagnosisV1, CandidateReplayCaseV1,
    CandidateReplayDecisionV1, CandidateReplayPreparationV1, CandidateReplayRootV1,
    FINITE_TEMPLATE_MAX_SEARCH_LIMIT, ProgressIntervalDevelopmentCorpusV1, RouteCapabilityInputV1,
    build_candidate_replay_development_preparations, canonical_digest,
    finite_template_proposal_configuration_v1, load_candidate_replay_root,
    run_finite_template_proposal_spike, validate_bounded_trajectory_reason_v1,
};

pub const PHYSICAL_WITNESS_DEVELOPMENT_SCHEMA_ID: &str =
    "bounded_trajectory_physical_development_v1";
pub const PHYSICAL_EXECUTOR_COMPARISON_SCHEMA_ID: &str = "physical_executor_comparison_v1";
pub const PHYSICAL_EXECUTOR_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalExecutorComparisonStatusV1 {
    Complete,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalExecutorInterpretationV1 {
    PhysicalAndExecutorSupported,
    PhysicalSupportedExecutorUnsupported,
    PhysicalSupportedExecutorUnknown,
    PhysicalUnknownExecutorSupported,
    PhysicalUnknownExecutorUnsupported,
    PhysicalAndExecutorUnknown,
    InvalidSource,
}

impl PhysicalExecutorInterpretationV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PhysicalAndExecutorSupported => "physical_and_executor_supported",
            Self::PhysicalSupportedExecutorUnsupported => "physical_supported_executor_unsupported",
            Self::PhysicalSupportedExecutorUnknown => "physical_supported_executor_unknown",
            Self::PhysicalUnknownExecutorSupported => "physical_unknown_executor_supported",
            Self::PhysicalUnknownExecutorUnsupported => "physical_unknown_executor_unsupported",
            Self::PhysicalAndExecutorUnknown => "physical_and_executor_unknown",
            Self::InvalidSource => "invalid_source",
        }
    }
}

pub fn physical_executor_interpretation_v1(
    physical_status: BoundedTrajectoryArtifactStatusV1,
    physical_decision: Option<BoundedTrajectoryDecisionV1>,
    executor_decision: CandidateReplayDecisionV1,
) -> (
    PhysicalExecutorComparisonStatusV1,
    PhysicalExecutorInterpretationV1,
) {
    if physical_status == BoundedTrajectoryArtifactStatusV1::Invalid
        || executor_decision == CandidateReplayDecisionV1::Invalid
    {
        return (
            PhysicalExecutorComparisonStatusV1::Invalid,
            PhysicalExecutorInterpretationV1::InvalidSource,
        );
    }
    let interpretation = match (physical_decision, executor_decision) {
        (Some(BoundedTrajectoryDecisionV1::Supported), CandidateReplayDecisionV1::Supported) => {
            PhysicalExecutorInterpretationV1::PhysicalAndExecutorSupported
        }
        (Some(BoundedTrajectoryDecisionV1::Supported), CandidateReplayDecisionV1::Unsupported) => {
            PhysicalExecutorInterpretationV1::PhysicalSupportedExecutorUnsupported
        }
        (Some(BoundedTrajectoryDecisionV1::Supported), CandidateReplayDecisionV1::Unknown) => {
            PhysicalExecutorInterpretationV1::PhysicalSupportedExecutorUnknown
        }
        (Some(BoundedTrajectoryDecisionV1::Unknown), CandidateReplayDecisionV1::Supported) => {
            PhysicalExecutorInterpretationV1::PhysicalUnknownExecutorSupported
        }
        (Some(BoundedTrajectoryDecisionV1::Unknown), CandidateReplayDecisionV1::Unsupported) => {
            PhysicalExecutorInterpretationV1::PhysicalUnknownExecutorUnsupported
        }
        (Some(BoundedTrajectoryDecisionV1::Unknown), CandidateReplayDecisionV1::Unknown) => {
            PhysicalExecutorInterpretationV1::PhysicalAndExecutorUnknown
        }
        _ => PhysicalExecutorInterpretationV1::InvalidSource,
    };
    let status = if interpretation == PhysicalExecutorInterpretationV1::InvalidSource {
        PhysicalExecutorComparisonStatusV1::Invalid
    } else {
        PhysicalExecutorComparisonStatusV1::Complete
    };
    (status, interpretation)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicalWitnessDevelopmentCaseV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub row_id: String,
    pub corpus: ProgressIntervalDevelopmentCorpusV1,
    pub candidate_rank: usize,
    pub preparation_digest: String,
    pub exposure_digest: String,
    pub base_resolved_input_digest: String,
    pub route_plan_digest: String,
    pub physical_input_digest: String,
    pub witness_configuration_digest: String,
    pub proposal_configuration: BoundedTrajectoryProposalConfigurationV1,
    pub prediction: BoundedTrajectoryWitnessPredictionV1,
    pub proposal_result_digest: String,
    pub case_digest: String,
}

impl PhysicalWitnessDevelopmentCaseV1 {
    fn validate_without_digest(&self) -> Result<(), String> {
        if self.schema_id != PHYSICAL_WITNESS_DEVELOPMENT_SCHEMA_ID
            || self.schema_version != PHYSICAL_EXECUTOR_SCHEMA_VERSION
        {
            return Err("physical witness development case schema mismatch".to_owned());
        }
        if self.candidate_rank != 0 {
            return Err(
                "physical witness development case must bind candidate rank zero".to_owned(),
            );
        }
        for (name, value) in [
            (
                "source_d0_input_digest",
                self.source_d0_input_digest.as_str(),
            ),
            ("row_id", self.row_id.as_str()),
            ("preparation_digest", self.preparation_digest.as_str()),
            ("exposure_digest", self.exposure_digest.as_str()),
            (
                "base_resolved_input_digest",
                self.base_resolved_input_digest.as_str(),
            ),
            ("route_plan_digest", self.route_plan_digest.as_str()),
            ("physical_input_digest", self.physical_input_digest.as_str()),
            (
                "witness_configuration_digest",
                self.witness_configuration_digest.as_str(),
            ),
            (
                "proposal_result_digest",
                self.proposal_result_digest.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                return Err(format!(
                    "physical witness development case has empty {name}"
                ));
            }
        }
        self.proposal_configuration.validate()?;
        let expected_proposal =
            finite_template_proposal_configuration_v1(self.proposal_configuration.search_limit)?;
        if self.proposal_configuration != expected_proposal {
            return Err("physical witness development proposal configuration is not W3".to_owned());
        }
        self.prediction.validate()?;
        if self.prediction.status != BoundedTrajectoryArtifactStatusV1::Complete
            || self.prediction.decision.is_none()
            || self.prediction.input_digest != self.physical_input_digest
            || self.prediction.configuration_digest != self.witness_configuration_digest
            || self.prediction.proposal_configuration_digest.as_deref()
                != Some(&self.proposal_configuration.configuration_digest)
        {
            return Err("physical witness development prediction join mismatch".to_owned());
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_without_digest()?;
        let mut material = self.clone();
        material.case_digest.clear();
        if self.case_digest.is_empty() || self.case_digest != canonical_digest(&material)? {
            return Err("physical witness development case digest mismatch".to_owned());
        }
        Ok(())
    }

    fn seal(&mut self) -> Result<(), String> {
        self.case_digest.clear();
        self.case_digest = canonical_digest(self)?;
        self.validate()
    }

    pub fn validate_against_preparation_exact(
        &self,
        preparation: &CandidateReplayPreparationV1,
    ) -> Result<(), String> {
        self.validate()?;
        preparation.validate()?;
        if self.source_d0_input_digest != preparation.source_d0_input_digest
            || self.row_id != preparation.row_id
            || self.corpus != preparation.corpus
            || self.preparation_digest != preparation.preparation_digest
            || self.exposure_digest != preparation.exposure.exposure_digest
            || self.base_resolved_input_digest != preparation.base_resolved_input_digest
            || self.route_plan_digest != preparation.selected_plan.plan_digest
        {
            return Err("physical witness development preparation join mismatch".to_owned());
        }
        let selected =
            preparation.exposure.candidates.first().ok_or_else(|| {
                "physical witness development exposure has no rank zero".to_owned()
            })?;
        if selected.plan_digest != self.route_plan_digest
            || selected.plan != preparation.selected_plan
        {
            return Err("physical witness development selected candidate mismatch".to_owned());
        }
        let recomputed = RouteCapabilityInputV1::from_request_and_plan(
            &preparation.request,
            &preparation.selected_plan,
        )?;
        if recomputed != preparation.capability_input
            || recomputed.physical_digest()? != self.physical_input_digest
        {
            return Err("physical witness development input recomputation mismatch".to_owned());
        }
        let witness_configuration = BoundedTrajectoryWitnessConfigurationV1::v1().seal()?;
        if witness_configuration.configuration_digest != self.witness_configuration_digest {
            return Err("physical witness development witness configuration mismatch".to_owned());
        }
        let result = run_finite_template_proposal_spike(
            &recomputed,
            &witness_configuration,
            &self.proposal_configuration,
        )?;
        if result.prediction != self.prediction
            || result.result_digest != self.proposal_result_digest
        {
            return Err("physical witness development exact proposal replay mismatch".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicalWitnessDevelopmentSummaryV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub witness_configuration_digest: String,
    pub proposal_configuration_digest: String,
    pub search_limit: u64,
    pub case_count: usize,
    pub baseline_count: usize,
    pub diagnostic_count: usize,
    pub supported_count: usize,
    pub unknown_count: usize,
    pub case_digests: Vec<String>,
    pub summary_digest: String,
}

impl PhysicalWitnessDevelopmentSummaryV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != PHYSICAL_WITNESS_DEVELOPMENT_SCHEMA_ID
            || self.schema_version != PHYSICAL_EXECUTOR_SCHEMA_VERSION
        {
            return Err("physical witness development summary schema mismatch".to_owned());
        }
        if self.source_d0_input_digest.trim().is_empty()
            || self.witness_configuration_digest.trim().is_empty()
            || self.proposal_configuration_digest.trim().is_empty()
            || self.case_count == 0
            || self.case_count != self.case_digests.len()
            || self.baseline_count.checked_add(self.diagnostic_count) != Some(self.case_count)
            || self.supported_count.checked_add(self.unknown_count) != Some(self.case_count)
            || self
                .case_digests
                .iter()
                .any(|value| value.trim().is_empty())
            || self.case_digests.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("physical witness development summary fields are inconsistent".to_owned());
        }
        if self.search_limit != FINITE_TEMPLATE_MAX_SEARCH_LIMIT {
            return Err("W4 physical summary must use the complete W3 catalog".to_owned());
        }
        let proposal = finite_template_proposal_configuration_v1(self.search_limit)?;
        if proposal.configuration_digest != self.proposal_configuration_digest {
            return Err(
                "physical witness development summary proposal configuration mismatch".to_owned(),
            );
        }
        let witness = BoundedTrajectoryWitnessConfigurationV1::v1().seal()?;
        if witness.configuration_digest != self.witness_configuration_digest {
            return Err(
                "physical witness development summary witness configuration mismatch".to_owned(),
            );
        }
        let mut material = self.clone();
        material.summary_digest.clear();
        if self.summary_digest.is_empty() || self.summary_digest != canonical_digest(&material)? {
            return Err("physical witness development summary digest mismatch".to_owned());
        }
        Ok(())
    }

    fn seal(&mut self) -> Result<(), String> {
        self.summary_digest.clear();
        self.summary_digest = canonical_digest(self)?;
        self.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicalExecutorComparisonV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub row_id: String,
    pub corpus: ProgressIntervalDevelopmentCorpusV1,
    pub candidate_rank: usize,
    pub preparation_digest: String,
    pub exposure_digest: String,
    pub base_resolved_input_digest: String,
    pub route_plan_digest: String,
    pub physical_input_digest: String,
    pub witness_configuration_digest: String,
    pub proposal_configuration_digest: String,
    pub physical_source_digest: String,
    pub executor_source_digest: String,
    pub pairing_config_digest: String,
    pub physical_status: BoundedTrajectoryArtifactStatusV1,
    pub physical_decision: Option<BoundedTrajectoryDecisionV1>,
    pub physical_reason: Option<String>,
    pub executor_decision: CandidateReplayDecisionV1,
    pub executor_reason: String,
    pub executor_case_diagnosis: CandidateReplayCaseDiagnosisV1,
    pub status: PhysicalExecutorComparisonStatusV1,
    pub interpretation: PhysicalExecutorInterpretationV1,
    pub comparison_digest: String,
}

impl PhysicalExecutorComparisonV1 {
    fn validate_without_digest(&self) -> Result<(), String> {
        if self.schema_id != PHYSICAL_EXECUTOR_COMPARISON_SCHEMA_ID
            || self.schema_version != PHYSICAL_EXECUTOR_SCHEMA_VERSION
        {
            return Err("physical/executor comparison schema mismatch".to_owned());
        }
        if self.candidate_rank != 0 {
            return Err("physical/executor comparison must bind candidate rank zero".to_owned());
        }
        for (name, value) in [
            (
                "source_d0_input_digest",
                self.source_d0_input_digest.as_str(),
            ),
            ("row_id", self.row_id.as_str()),
            ("preparation_digest", self.preparation_digest.as_str()),
            ("exposure_digest", self.exposure_digest.as_str()),
            (
                "base_resolved_input_digest",
                self.base_resolved_input_digest.as_str(),
            ),
            ("route_plan_digest", self.route_plan_digest.as_str()),
            ("physical_input_digest", self.physical_input_digest.as_str()),
            (
                "witness_configuration_digest",
                self.witness_configuration_digest.as_str(),
            ),
            (
                "proposal_configuration_digest",
                self.proposal_configuration_digest.as_str(),
            ),
            (
                "physical_source_digest",
                self.physical_source_digest.as_str(),
            ),
            (
                "executor_source_digest",
                self.executor_source_digest.as_str(),
            ),
            ("pairing_config_digest", self.pairing_config_digest.as_str()),
            ("executor_reason", self.executor_reason.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("physical/executor comparison has empty {name}"));
            }
        }
        let (status, interpretation) = physical_executor_interpretation_v1(
            self.physical_status,
            self.physical_decision,
            self.executor_decision,
        );
        if status != self.status || interpretation != self.interpretation {
            return Err("physical/executor comparison interpretation mismatch".to_owned());
        }
        match self.physical_status {
            BoundedTrajectoryArtifactStatusV1::Complete => {
                if self.physical_decision.is_none()
                    || self
                        .physical_reason
                        .as_deref()
                        .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err("complete physical axis is missing decision/reason".to_owned());
                }
                validate_bounded_trajectory_reason_v1(
                    self.physical_reason
                        .as_deref()
                        .expect("complete physical reason checked above"),
                )?;
            }
            BoundedTrajectoryArtifactStatusV1::Invalid => {
                if self.physical_decision.is_some()
                    || self
                        .physical_reason
                        .as_deref()
                        .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(
                        "invalid physical axis must omit its decision and retain a reason"
                            .to_owned(),
                    );
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_without_digest()?;
        let mut material = self.clone();
        material.comparison_digest.clear();
        if self.comparison_digest.is_empty()
            || self.comparison_digest != canonical_digest(&material)?
        {
            return Err("physical/executor comparison digest mismatch".to_owned());
        }
        Ok(())
    }

    fn seal(&mut self) -> Result<(), String> {
        self.comparison_digest.clear();
        self.comparison_digest = canonical_digest(self)?;
        self.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicalExecutorComparisonSummaryV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub physical_summary_digest: String,
    pub executor_summary_digest: String,
    pub witness_configuration_digest: String,
    pub proposal_configuration_digest: String,
    pub pairing_config_digest: String,
    pub case_count: usize,
    pub complete_count: usize,
    pub invalid_count: usize,
    pub interpretation_counts: BTreeMap<String, usize>,
    pub selection_gap_row_ids: Vec<String>,
    pub comparison_digests: Vec<String>,
    pub summary_digest: String,
}

impl PhysicalExecutorComparisonSummaryV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != PHYSICAL_EXECUTOR_COMPARISON_SCHEMA_ID
            || self.schema_version != PHYSICAL_EXECUTOR_SCHEMA_VERSION
        {
            return Err("physical/executor comparison summary schema mismatch".to_owned());
        }
        if self.source_d0_input_digest.trim().is_empty()
            || self.physical_summary_digest.trim().is_empty()
            || self.executor_summary_digest.trim().is_empty()
            || self.witness_configuration_digest.trim().is_empty()
            || self.proposal_configuration_digest.trim().is_empty()
            || self.pairing_config_digest.trim().is_empty()
            || self.case_count == 0
            || self.complete_count.checked_add(self.invalid_count) != Some(self.case_count)
            || self.comparison_digests.len() != self.case_count
            || self
                .comparison_digests
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self
                .selection_gap_row_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err("physical/executor comparison summary fields are inconsistent".to_owned());
        }
        let known = all_interpretations()
            .into_iter()
            .map(PhysicalExecutorInterpretationV1::as_str)
            .collect::<BTreeSet<_>>();
        let actual = self
            .interpretation_counts
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if actual != known
            || self.interpretation_counts.values().sum::<usize>() != self.case_count
            || self
                .comparison_digests
                .iter()
                .any(|digest| digest.trim().is_empty())
            || self
                .selection_gap_row_ids
                .iter()
                .any(|row_id| row_id.trim().is_empty())
        {
            return Err("physical/executor interpretation counts are inconsistent".to_owned());
        }
        let mut material = self.clone();
        material.summary_digest.clear();
        if self.summary_digest.is_empty() || self.summary_digest != canonical_digest(&material)? {
            return Err("physical/executor comparison summary digest mismatch".to_owned());
        }
        Ok(())
    }

    fn seal(&mut self) -> Result<(), String> {
        self.summary_digest.clear();
        self.summary_digest = canonical_digest(self)?;
        self.validate()
    }
}

fn all_interpretations() -> [PhysicalExecutorInterpretationV1; 7] {
    [
        PhysicalExecutorInterpretationV1::PhysicalAndExecutorSupported,
        PhysicalExecutorInterpretationV1::PhysicalSupportedExecutorUnsupported,
        PhysicalExecutorInterpretationV1::PhysicalSupportedExecutorUnknown,
        PhysicalExecutorInterpretationV1::PhysicalUnknownExecutorSupported,
        PhysicalExecutorInterpretationV1::PhysicalUnknownExecutorUnsupported,
        PhysicalExecutorInterpretationV1::PhysicalAndExecutorUnknown,
        PhysicalExecutorInterpretationV1::InvalidSource,
    ]
}

#[derive(Clone)]
struct PhysicalWitnessRootV1 {
    summary: PhysicalWitnessDevelopmentSummaryV1,
    cases: Vec<(
        CandidateReplayPreparationV1,
        PhysicalWitnessDevelopmentCaseV1,
    )>,
}

fn require_empty_output_dir(output_dir: &Path, context: &str) -> Result<()> {
    match fs::read_dir(output_dir) {
        Ok(mut entries) => {
            if entries.next().transpose()?.is_some() {
                bail!(
                    "{context} output directory must be empty: {}",
                    output_dir.display()
                );
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to inspect {context} output directory {}",
                output_dir.display()
            )
        }),
    }
}

fn validate_row_id(row_id: &str) -> Result<()> {
    if row_id.trim().is_empty()
        || row_id == "."
        || row_id == ".."
        || row_id.contains('/')
        || row_id.contains('\\')
    {
        bail!("W4 row ID must be one nonempty output-directory component");
    }
    Ok(())
}

fn directory_entries(path: &Path) -> Result<BTreeSet<String>> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("failed to read W4 artifact directory {}", path.display()))?;
    entries
        .map(|entry| {
            entry
                .with_context(|| format!("failed to inspect {}", path.display()))?
                .file_name()
                .into_string()
                .map_err(|_| anyhow!("W4 artifact directory contains a non-UTF-8 entry"))
        })
        .collect()
}

fn require_directory_entries(
    path: &Path,
    expected: &BTreeSet<String>,
    context: &str,
) -> Result<()> {
    if directory_entries(path)? != *expected {
        bail!("{context} directory entries differ at {}", path.display());
    }
    Ok(())
}

fn write_json_round_trip<T, F>(path: &Path, value: &T, validate: F) -> Result<T>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> Result<()>,
{
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, &bytes)
        .with_context(|| format!("failed to write W4 artifact {}", path.display()))?;
    read_json_validate(path, validate)
}

fn read_json_validate<T, F>(path: &Path, validate: F) -> Result<T>
where
    T: Serialize + DeserializeOwned,
    F: Fn(&T) -> Result<()>,
{
    let bytes =
        fs::read(path).with_context(|| format!("failed to read W4 artifact {}", path.display()))?;
    let value: T = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to deserialize W4 artifact {}", path.display()))?;
    if serde_json::to_vec_pretty(&value)? != bytes {
        bail!(
            "W4 artifact is not canonical typed JSON: {}",
            path.display()
        );
    }
    validate(&value)?;
    Ok(value)
}

fn build_physical_case(
    preparation: &CandidateReplayPreparationV1,
    witness_configuration: &BoundedTrajectoryWitnessConfigurationV1,
    proposal_configuration: &BoundedTrajectoryProposalConfigurationV1,
) -> Result<PhysicalWitnessDevelopmentCaseV1> {
    preparation.validate().map_err(anyhow::Error::msg)?;
    let result = run_finite_template_proposal_spike(
        &preparation.capability_input,
        witness_configuration,
        proposal_configuration,
    )
    .map_err(anyhow::Error::msg)?;
    let mut case = PhysicalWitnessDevelopmentCaseV1 {
        schema_id: PHYSICAL_WITNESS_DEVELOPMENT_SCHEMA_ID.to_owned(),
        schema_version: PHYSICAL_EXECUTOR_SCHEMA_VERSION,
        source_d0_input_digest: preparation.source_d0_input_digest.clone(),
        row_id: preparation.row_id.clone(),
        corpus: preparation.corpus,
        candidate_rank: 0,
        preparation_digest: preparation.preparation_digest.clone(),
        exposure_digest: preparation.exposure.exposure_digest.clone(),
        base_resolved_input_digest: preparation.base_resolved_input_digest.clone(),
        route_plan_digest: preparation.selected_plan.plan_digest.clone(),
        physical_input_digest: preparation
            .capability_input
            .physical_digest()
            .map_err(anyhow::Error::msg)?,
        witness_configuration_digest: witness_configuration.configuration_digest.clone(),
        proposal_configuration: proposal_configuration.clone(),
        prediction: result.prediction,
        proposal_result_digest: result.result_digest,
        case_digest: String::new(),
    };
    case.seal().map_err(anyhow::Error::msg)?;
    Ok(case)
}

fn build_physical_summary(
    cases: &[PhysicalWitnessDevelopmentCaseV1],
) -> Result<PhysicalWitnessDevelopmentSummaryV1> {
    let first = cases
        .first()
        .ok_or_else(|| anyhow!("W4 physical development run produced no cases"))?;
    let mut baseline_count = 0_usize;
    let mut diagnostic_count = 0_usize;
    let mut supported_count = 0_usize;
    let mut unknown_count = 0_usize;
    let mut case_digests = Vec::with_capacity(cases.len());
    let mut row_ids = BTreeSet::new();
    for case in cases {
        case.validate().map_err(anyhow::Error::msg)?;
        if case.source_d0_input_digest != first.source_d0_input_digest
            || case.witness_configuration_digest != first.witness_configuration_digest
            || case.proposal_configuration != first.proposal_configuration
            || !row_ids.insert(case.row_id.clone())
        {
            bail!("W4 physical development cases have mixed identities");
        }
        match case.corpus {
            ProgressIntervalDevelopmentCorpusV1::Baseline => baseline_count += 1,
            ProgressIntervalDevelopmentCorpusV1::Diagnostic => diagnostic_count += 1,
        }
        match case.prediction.decision {
            Some(BoundedTrajectoryDecisionV1::Supported) => supported_count += 1,
            Some(BoundedTrajectoryDecisionV1::Unknown) => unknown_count += 1,
            None => bail!("W4 physical development case has no decision"),
        }
        case_digests.push(case.case_digest.clone());
    }
    case_digests.sort();
    if case_digests.windows(2).any(|pair| pair[0] == pair[1]) {
        bail!("W4 physical development cases have duplicate digests");
    }
    let mut summary = PhysicalWitnessDevelopmentSummaryV1 {
        schema_id: PHYSICAL_WITNESS_DEVELOPMENT_SCHEMA_ID.to_owned(),
        schema_version: PHYSICAL_EXECUTOR_SCHEMA_VERSION,
        source_d0_input_digest: first.source_d0_input_digest.clone(),
        witness_configuration_digest: first.witness_configuration_digest.clone(),
        proposal_configuration_digest: first.proposal_configuration.configuration_digest.clone(),
        search_limit: first.proposal_configuration.search_limit,
        case_count: cases.len(),
        baseline_count,
        diagnostic_count,
        supported_count,
        unknown_count,
        case_digests,
        summary_digest: String::new(),
    };
    summary.seal().map_err(anyhow::Error::msg)?;
    Ok(summary)
}

/// Generate and seal the W4 physical lane from development inputs alone.
///
/// This API deliberately has no R1 artifact argument and always uses the
/// complete, already-versioned W3 catalog.
pub fn run_physical_witness_development(
    manifest_path: &Path,
    repo_root: &Path,
    output_dir: &Path,
) -> Result<PhysicalWitnessDevelopmentSummaryV1> {
    require_empty_output_dir(output_dir, "W4 physical")?;
    let preparations = build_candidate_replay_development_preparations(manifest_path, repo_root)?;
    let witness_configuration = BoundedTrajectoryWitnessConfigurationV1::v1()
        .seal()
        .map_err(anyhow::Error::msg)?;
    let proposal_configuration =
        finite_template_proposal_configuration_v1(FINITE_TEMPLATE_MAX_SEARCH_LIMIT)
            .map_err(anyhow::Error::msg)?;

    fs::create_dir_all(output_dir.join("case")).with_context(|| {
        format!(
            "failed to create W4 physical output directory {}",
            output_dir.display()
        )
    })?;
    // Each W3 search remains serial and deterministic. Development rows are
    // independent, so the batch may evaluate them in parallel; indexed
    // collection retains manifest order and summaries sort proof identities.
    let cases = preparations
        .into_par_iter()
        .map(|preparation| -> Result<PhysicalWitnessDevelopmentCaseV1> {
            validate_row_id(&preparation.row_id)?;
            let case_dir = output_dir.join("case").join(&preparation.row_id);
            fs::create_dir_all(&case_dir).with_context(|| {
                format!(
                    "failed to create W4 physical case directory {}",
                    case_dir.display()
                )
            })?;
            let persisted_preparation: CandidateReplayPreparationV1 = write_json_round_trip(
                &case_dir.join("preparation.json"),
                &preparation,
                |value: &CandidateReplayPreparationV1| value.validate().map_err(anyhow::Error::msg),
            )?;
            let case = build_physical_case(
                &persisted_preparation,
                &witness_configuration,
                &proposal_configuration,
            )?;
            let persisted_case: PhysicalWitnessDevelopmentCaseV1 = write_json_round_trip(
                &case_dir.join("case.json"),
                &case,
                |value: &PhysicalWitnessDevelopmentCaseV1| {
                    value.validate().map_err(anyhow::Error::msg)
                },
            )?;
            if persisted_case.preparation_digest != persisted_preparation.preparation_digest {
                bail!("W4 physical persisted case differs from its input preparation");
            }
            Ok(persisted_case)
        })
        .collect::<Result<Vec<_>>>()?;
    let summary = build_physical_summary(&cases)?;
    write_json_round_trip(
        &output_dir.join("summary.json"),
        &summary,
        |value: &PhysicalWitnessDevelopmentSummaryV1| value.validate().map_err(anyhow::Error::msg),
    )
}

fn load_physical_witness_root(root: &Path) -> Result<PhysicalWitnessRootV1> {
    require_directory_entries(
        root,
        &BTreeSet::from(["case".to_owned(), "summary.json".to_owned()]),
        "W4 physical root",
    )?;
    let summary: PhysicalWitnessDevelopmentSummaryV1 = read_json_validate(
        &root.join("summary.json"),
        |value: &PhysicalWitnessDevelopmentSummaryV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    let case_root = root.join("case");
    let case_names = directory_entries(&case_root)?;
    if case_names.len() != summary.case_count {
        bail!("W4 physical root case count differs from summary");
    }
    let case_names = case_names.into_iter().collect::<Vec<_>>();
    let cases = case_names
        .into_par_iter()
        .map(
            |row_id| -> Result<(
                CandidateReplayPreparationV1,
                PhysicalWitnessDevelopmentCaseV1,
            )> {
                validate_row_id(&row_id)?;
                let case_dir = case_root.join(&row_id);
                require_directory_entries(
                    &case_dir,
                    &BTreeSet::from(["case.json".to_owned(), "preparation.json".to_owned()]),
                    "W4 physical case",
                )?;
                let preparation: CandidateReplayPreparationV1 = read_json_validate(
                    &case_dir.join("preparation.json"),
                    |value: &CandidateReplayPreparationV1| {
                        value.validate().map_err(anyhow::Error::msg)
                    },
                )?;
                let case: PhysicalWitnessDevelopmentCaseV1 = read_json_validate(
                    &case_dir.join("case.json"),
                    |value: &PhysicalWitnessDevelopmentCaseV1| {
                        value.validate().map_err(anyhow::Error::msg)
                    },
                )?;
                if preparation.row_id != row_id || case.row_id != row_id {
                    bail!("W4 physical case directory does not match row identity");
                }
                case.validate_against_preparation_exact(&preparation)
                    .map_err(anyhow::Error::msg)?;
                Ok((preparation, case))
            },
        )
        .collect::<Result<Vec<_>>>()?;
    let loaded_cases = cases
        .iter()
        .map(|(_, case)| case.clone())
        .collect::<Vec<_>>();
    if build_physical_summary(&loaded_cases)? != summary {
        bail!("W4 physical summary differs from exact case artifacts");
    }
    Ok(PhysicalWitnessRootV1 { summary, cases })
}

/// Exactly reload a compact physical root, including deterministic W3 replay.
pub fn validate_physical_witness_development_root(
    root: &Path,
) -> Result<PhysicalWitnessDevelopmentSummaryV1> {
    Ok(load_physical_witness_root(root)?.summary)
}

fn build_comparison_from_validated_sources(
    physical_preparation: &CandidateReplayPreparationV1,
    physical_case: &PhysicalWitnessDevelopmentCaseV1,
    executor_preparation: &CandidateReplayPreparationV1,
    executor_case: &CandidateReplayCaseV1,
) -> Result<PhysicalExecutorComparisonV1> {
    physical_case.validate().map_err(anyhow::Error::msg)?;
    physical_preparation
        .validate()
        .map_err(anyhow::Error::msg)?;
    executor_preparation
        .validate()
        .map_err(anyhow::Error::msg)?;
    executor_case.validate().map_err(anyhow::Error::msg)?;
    if executor_preparation != physical_preparation {
        bail!("W4 physical and executor preparations are not the exact same input boundary");
    }
    if executor_case.row_id != executor_preparation.row_id
        || executor_case.corpus != executor_preparation.corpus
        || executor_case.base_input_digest != executor_preparation.base_input_digest
        || executor_case.base_resolved_input_digest
            != executor_preparation.base_resolved_input_digest
        || executor_case.exposure_digest != executor_preparation.exposure.exposure_digest
        || executor_case.selected_plan_digest != executor_preparation.selected_plan.plan_digest
        || executor_case.exposed_candidate_count != executor_preparation.exposure.candidates.len()
    {
        bail!("W4 executor case does not match its preparation");
    }
    let selected = executor_case
        .candidates
        .first()
        .ok_or_else(|| anyhow!("W4 executor case has no selected candidate"))?;
    if selected.rank != 0
        || selected.plan_digest != physical_case.route_plan_digest
        || executor_case.selected_plan_digest != physical_case.route_plan_digest
        || executor_case.selected_decision != selected.decision
    {
        bail!("W4 executor axis is not the exact selected rank-zero route");
    }
    let physical_status = physical_case.prediction.status;
    let physical_decision = physical_case.prediction.decision;
    let (status, interpretation) =
        physical_executor_interpretation_v1(physical_status, physical_decision, selected.decision);
    let mut comparison = PhysicalExecutorComparisonV1 {
        schema_id: PHYSICAL_EXECUTOR_COMPARISON_SCHEMA_ID.to_owned(),
        schema_version: PHYSICAL_EXECUTOR_SCHEMA_VERSION,
        source_d0_input_digest: physical_case.source_d0_input_digest.clone(),
        row_id: physical_case.row_id.clone(),
        corpus: physical_case.corpus,
        candidate_rank: 0,
        preparation_digest: physical_case.preparation_digest.clone(),
        exposure_digest: physical_case.exposure_digest.clone(),
        base_resolved_input_digest: physical_case.base_resolved_input_digest.clone(),
        route_plan_digest: physical_case.route_plan_digest.clone(),
        physical_input_digest: physical_case.physical_input_digest.clone(),
        witness_configuration_digest: physical_case.witness_configuration_digest.clone(),
        proposal_configuration_digest: physical_case
            .proposal_configuration
            .configuration_digest
            .clone(),
        physical_source_digest: physical_case.case_digest.clone(),
        executor_source_digest: executor_case.case_digest.clone(),
        pairing_config_digest: executor_case.pairing_config_digest.clone(),
        physical_status,
        physical_decision,
        physical_reason: physical_case.prediction.first_reason.clone(),
        executor_decision: selected.decision,
        executor_reason: selected.reason.clone(),
        executor_case_diagnosis: executor_case.diagnosis,
        status,
        interpretation,
        comparison_digest: String::new(),
    };
    comparison.seal().map_err(anyhow::Error::msg)?;
    Ok(comparison)
}

impl PhysicalExecutorComparisonV1 {
    pub fn validate_against_sources_exact(
        &self,
        physical_preparation: &CandidateReplayPreparationV1,
        physical_case: &PhysicalWitnessDevelopmentCaseV1,
        executor_preparation: &CandidateReplayPreparationV1,
        executor_case: &CandidateReplayCaseV1,
    ) -> Result<(), String> {
        self.validate()?;
        physical_case.validate_against_preparation_exact(physical_preparation)?;
        let expected = build_comparison_from_validated_sources(
            physical_preparation,
            physical_case,
            executor_preparation,
            executor_case,
        )
        .map_err(|error| error.to_string())?;
        if self != &expected {
            return Err("physical/executor comparison differs from exact source join".to_owned());
        }
        Ok(())
    }
}

fn build_comparison_summary(
    physical_summary: &PhysicalWitnessDevelopmentSummaryV1,
    executor_root: &CandidateReplayRootV1,
    comparisons: &[PhysicalExecutorComparisonV1],
) -> Result<PhysicalExecutorComparisonSummaryV1> {
    if comparisons.len() != physical_summary.case_count
        || comparisons.len() != executor_root.summary.case_count
    {
        bail!("W4 comparison case count differs from a source summary");
    }
    let mut interpretation_counts = BTreeMap::new();
    for interpretation in all_interpretations() {
        interpretation_counts.insert(interpretation.as_str().to_owned(), 0_usize);
    }
    let mut complete_count = 0_usize;
    let mut invalid_count = 0_usize;
    let mut selection_gap_row_ids = Vec::new();
    let mut comparison_digests = Vec::with_capacity(comparisons.len());
    let mut row_ids = BTreeSet::new();
    for comparison in comparisons {
        comparison.validate().map_err(anyhow::Error::msg)?;
        if comparison.source_d0_input_digest != physical_summary.source_d0_input_digest
            || comparison.witness_configuration_digest
                != physical_summary.witness_configuration_digest
            || comparison.proposal_configuration_digest
                != physical_summary.proposal_configuration_digest
            || comparison.pairing_config_digest != executor_root.summary.pairing_config_digest
            || !row_ids.insert(comparison.row_id.clone())
        {
            bail!("W4 comparisons have mixed or duplicate identities");
        }
        match comparison.status {
            PhysicalExecutorComparisonStatusV1::Complete => complete_count += 1,
            PhysicalExecutorComparisonStatusV1::Invalid => invalid_count += 1,
        }
        *interpretation_counts
            .get_mut(comparison.interpretation.as_str())
            .expect("all W4 interpretations were initialized") += 1;
        if comparison.executor_case_diagnosis
            == CandidateReplayCaseDiagnosisV1::ExecutorSelectionGapWitnessed
        {
            selection_gap_row_ids.push(comparison.row_id.clone());
        }
        comparison_digests.push(comparison.comparison_digest.clone());
    }
    selection_gap_row_ids.sort();
    comparison_digests.sort();
    if comparison_digests.windows(2).any(|pair| pair[0] == pair[1]) {
        bail!("W4 comparison cases have duplicate digests");
    }
    let mut summary = PhysicalExecutorComparisonSummaryV1 {
        schema_id: PHYSICAL_EXECUTOR_COMPARISON_SCHEMA_ID.to_owned(),
        schema_version: PHYSICAL_EXECUTOR_SCHEMA_VERSION,
        source_d0_input_digest: physical_summary.source_d0_input_digest.clone(),
        physical_summary_digest: physical_summary.summary_digest.clone(),
        executor_summary_digest: executor_root.summary.summary_digest.clone(),
        witness_configuration_digest: physical_summary.witness_configuration_digest.clone(),
        proposal_configuration_digest: physical_summary.proposal_configuration_digest.clone(),
        pairing_config_digest: executor_root.summary.pairing_config_digest.clone(),
        case_count: comparisons.len(),
        complete_count,
        invalid_count,
        interpretation_counts,
        selection_gap_row_ids,
        comparison_digests,
        summary_digest: String::new(),
    };
    summary.seal().map_err(anyhow::Error::msg)?;
    Ok(summary)
}

/// Join one sealed W4 physical root to one separately sealed R1 root.
///
/// The physical root is loaded and exactly replayed before this function opens
/// the R1 path.  Every comparison uses candidate rank zero on both axes.
pub fn run_physical_executor_comparison(
    physical_root: &Path,
    executor_root: &Path,
    output_dir: &Path,
) -> Result<PhysicalExecutorComparisonSummaryV1> {
    require_empty_output_dir(output_dir, "W4 comparison")?;

    // Preserve the outcome-isolation order: this exact replay completes before
    // the executor root is opened.
    let physical = load_physical_witness_root(physical_root)?;
    let executor = load_candidate_replay_root(executor_root)?;
    if physical.summary.source_d0_input_digest != executor.summary.source_d0_input_digest
        || physical.summary.case_count != executor.summary.case_count
    {
        bail!("W4 physical and executor source summaries do not match");
    }

    let mut executor_cases = executor
        .cases
        .iter()
        .map(|(preparation, case)| (case.row_id.clone(), (preparation.clone(), case.clone())))
        .collect::<BTreeMap<_, _>>();
    if executor_cases.len() != executor.cases.len() {
        bail!("W4 executor root contains duplicate row identities");
    }

    fs::create_dir_all(output_dir.join("case")).with_context(|| {
        format!(
            "failed to create W4 comparison output directory {}",
            output_dir.display()
        )
    })?;
    let mut comparisons = Vec::with_capacity(physical.cases.len());
    for (physical_preparation, physical_case) in &physical.cases {
        let (executor_preparation, executor_case) = executor_cases
            .remove(&physical_case.row_id)
            .ok_or_else(|| anyhow!("W4 executor root is missing row {}", physical_case.row_id))?;
        let comparison = build_comparison_from_validated_sources(
            physical_preparation,
            physical_case,
            &executor_preparation,
            &executor_case,
        )?;
        let case_dir = output_dir.join("case").join(&comparison.row_id);
        fs::create_dir_all(&case_dir).with_context(|| {
            format!(
                "failed to create W4 comparison case directory {}",
                case_dir.display()
            )
        })?;
        let persisted: PhysicalExecutorComparisonV1 = write_json_round_trip(
            &case_dir.join("comparison.json"),
            &comparison,
            |value: &PhysicalExecutorComparisonV1| value.validate().map_err(anyhow::Error::msg),
        )?;
        if persisted != comparison {
            bail!("W4 persisted comparison differs from its validated source join");
        }
        comparisons.push(persisted);
    }
    if !executor_cases.is_empty() {
        bail!("W4 executor root contains rows absent from the physical root");
    }
    let summary = build_comparison_summary(&physical.summary, &executor, &comparisons)?;
    write_json_round_trip(
        &output_dir.join("summary.json"),
        &summary,
        |value: &PhysicalExecutorComparisonSummaryV1| value.validate().map_err(anyhow::Error::msg),
    )
}

/// Reload a W4 comparison root against both exact source roots.
///
/// The compact physical root is completely replayed before the R1 root or the
/// comparison artifacts are opened.
pub fn validate_physical_executor_comparison_root(
    comparison_root: &Path,
    physical_root: &Path,
    executor_root: &Path,
) -> Result<PhysicalExecutorComparisonSummaryV1> {
    let physical = load_physical_witness_root(physical_root)?;
    let executor = load_candidate_replay_root(executor_root)?;
    require_directory_entries(
        comparison_root,
        &BTreeSet::from(["case".to_owned(), "summary.json".to_owned()]),
        "W4 comparison root",
    )?;
    let summary: PhysicalExecutorComparisonSummaryV1 = read_json_validate(
        &comparison_root.join("summary.json"),
        |value: &PhysicalExecutorComparisonSummaryV1| value.validate().map_err(anyhow::Error::msg),
    )?;
    if summary.physical_summary_digest != physical.summary.summary_digest
        || summary.executor_summary_digest != executor.summary.summary_digest
        || summary.source_d0_input_digest != physical.summary.source_d0_input_digest
        || summary.source_d0_input_digest != executor.summary.source_d0_input_digest
    {
        bail!("W4 comparison summary does not bind the supplied source roots");
    }

    let physical_cases = physical
        .cases
        .iter()
        .map(|(preparation, case)| (case.row_id.clone(), (preparation, case)))
        .collect::<BTreeMap<_, _>>();
    let executor_cases = executor
        .cases
        .iter()
        .map(|(preparation, case)| (case.row_id.clone(), (preparation, case)))
        .collect::<BTreeMap<_, _>>();
    if physical_cases.len() != physical.cases.len() || executor_cases.len() != executor.cases.len()
    {
        bail!("W4 source roots contain duplicate row identities");
    }

    let comparison_case_root = comparison_root.join("case");
    let row_ids = directory_entries(&comparison_case_root)?;
    if row_ids.len() != summary.case_count {
        bail!("W4 comparison root case count differs from summary");
    }
    let mut comparisons = Vec::with_capacity(row_ids.len());
    for row_id in row_ids {
        validate_row_id(&row_id)?;
        let case_dir = comparison_case_root.join(&row_id);
        require_directory_entries(
            &case_dir,
            &BTreeSet::from(["comparison.json".to_owned()]),
            "W4 comparison case",
        )?;
        let comparison: PhysicalExecutorComparisonV1 = read_json_validate(
            &case_dir.join("comparison.json"),
            |value: &PhysicalExecutorComparisonV1| value.validate().map_err(anyhow::Error::msg),
        )?;
        if comparison.row_id != row_id {
            bail!("W4 comparison case directory does not match row identity");
        }
        let (physical_preparation, physical_case) = physical_cases
            .get(&row_id)
            .ok_or_else(|| anyhow!("W4 physical root is missing comparison row {row_id}"))?;
        let (executor_preparation, executor_case) = executor_cases
            .get(&row_id)
            .ok_or_else(|| anyhow!("W4 executor root is missing comparison row {row_id}"))?;
        let expected = build_comparison_from_validated_sources(
            physical_preparation,
            physical_case,
            executor_preparation,
            executor_case,
        )?;
        if comparison != expected {
            bail!("W4 comparison row {row_id} differs from its exact source join");
        }
        comparisons.push(comparison);
    }
    if comparisons.len() != physical_cases.len() || comparisons.len() != executor_cases.len() {
        bail!("W4 comparison root does not cover both source row sets exactly");
    }
    let rebuilt = build_comparison_summary(&physical.summary, &executor, &comparisons)?;
    if rebuilt != summary {
        bail!("W4 comparison summary differs from validated comparison cases");
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("pd_w4_{label}_{nonce}"))
    }

    fn comparison(
        physical: BoundedTrajectoryDecisionV1,
        executor: CandidateReplayDecisionV1,
    ) -> PhysicalExecutorComparisonV1 {
        let (status, interpretation) = physical_executor_interpretation_v1(
            BoundedTrajectoryArtifactStatusV1::Complete,
            Some(physical),
            executor,
        );
        let mut value = PhysicalExecutorComparisonV1 {
            schema_id: PHYSICAL_EXECUTOR_COMPARISON_SCHEMA_ID.to_owned(),
            schema_version: PHYSICAL_EXECUTOR_SCHEMA_VERSION,
            source_d0_input_digest: "source".to_owned(),
            row_id: "row".to_owned(),
            corpus: ProgressIntervalDevelopmentCorpusV1::Baseline,
            candidate_rank: 0,
            preparation_digest: "preparation".to_owned(),
            exposure_digest: "exposure".to_owned(),
            base_resolved_input_digest: "resolved".to_owned(),
            route_plan_digest: "plan".to_owned(),
            physical_input_digest: "input".to_owned(),
            witness_configuration_digest: "witness-config".to_owned(),
            proposal_configuration_digest: "proposal-config".to_owned(),
            physical_source_digest: "physical-source".to_owned(),
            executor_source_digest: "executor-source".to_owned(),
            pairing_config_digest: "pairing".to_owned(),
            physical_status: BoundedTrajectoryArtifactStatusV1::Complete,
            physical_decision: Some(physical),
            physical_reason: Some(match physical {
                BoundedTrajectoryDecisionV1::Supported => {
                    "supported/exact_witness_verified".to_owned()
                }
                BoundedTrajectoryDecisionV1::Unknown => {
                    "unknown/coverage/bounded_search_exhausted".to_owned()
                }
            }),
            executor_decision: executor,
            executor_reason: match executor {
                CandidateReplayDecisionV1::Supported => "supported/contract_complete",
                CandidateReplayDecisionV1::Unsupported => {
                    "unsupported/containment/waypoint_deadline"
                }
                CandidateReplayDecisionV1::Unknown => "unknown/scope/direct_route",
                CandidateReplayDecisionV1::Invalid => "invalid/replay/nondeterministic",
            }
            .to_owned(),
            executor_case_diagnosis: CandidateReplayCaseDiagnosisV1::SelectedPairCompatible,
            status,
            interpretation,
            comparison_digest: String::new(),
        };
        value.seal().expect("test comparison should seal");
        value
    }

    #[test]
    fn interpretation_matrix_keeps_physical_and_executor_axes_separate() {
        let cells = [
            (
                BoundedTrajectoryDecisionV1::Supported,
                CandidateReplayDecisionV1::Supported,
                PhysicalExecutorInterpretationV1::PhysicalAndExecutorSupported,
            ),
            (
                BoundedTrajectoryDecisionV1::Supported,
                CandidateReplayDecisionV1::Unsupported,
                PhysicalExecutorInterpretationV1::PhysicalSupportedExecutorUnsupported,
            ),
            (
                BoundedTrajectoryDecisionV1::Supported,
                CandidateReplayDecisionV1::Unknown,
                PhysicalExecutorInterpretationV1::PhysicalSupportedExecutorUnknown,
            ),
            (
                BoundedTrajectoryDecisionV1::Unknown,
                CandidateReplayDecisionV1::Supported,
                PhysicalExecutorInterpretationV1::PhysicalUnknownExecutorSupported,
            ),
            (
                BoundedTrajectoryDecisionV1::Unknown,
                CandidateReplayDecisionV1::Unsupported,
                PhysicalExecutorInterpretationV1::PhysicalUnknownExecutorUnsupported,
            ),
            (
                BoundedTrajectoryDecisionV1::Unknown,
                CandidateReplayDecisionV1::Unknown,
                PhysicalExecutorInterpretationV1::PhysicalAndExecutorUnknown,
            ),
        ];
        for (physical, executor, expected) in cells {
            let (status, interpretation) = physical_executor_interpretation_v1(
                BoundedTrajectoryArtifactStatusV1::Complete,
                Some(physical),
                executor,
            );
            assert_eq!(status, PhysicalExecutorComparisonStatusV1::Complete);
            assert_eq!(interpretation, expected);
            comparison(physical, executor)
                .validate()
                .expect("mapped comparison should validate");
        }
    }

    #[test]
    fn invalid_axis_invalidates_only_the_join() {
        for (physical_status, physical_decision, executor_decision) in [
            (
                BoundedTrajectoryArtifactStatusV1::Invalid,
                None,
                CandidateReplayDecisionV1::Supported,
            ),
            (
                BoundedTrajectoryArtifactStatusV1::Complete,
                Some(BoundedTrajectoryDecisionV1::Supported),
                CandidateReplayDecisionV1::Invalid,
            ),
        ] {
            assert_eq!(
                physical_executor_interpretation_v1(
                    physical_status,
                    physical_decision,
                    executor_decision,
                ),
                (
                    PhysicalExecutorComparisonStatusV1::Invalid,
                    PhysicalExecutorInterpretationV1::InvalidSource,
                )
            );
        }
    }

    #[test]
    fn comparison_digest_covers_exact_join_fields() {
        let original = comparison(
            BoundedTrajectoryDecisionV1::Supported,
            CandidateReplayDecisionV1::Unsupported,
        );
        for mutate in [
            |value: &mut PhysicalExecutorComparisonV1| value.candidate_rank = 1,
            |value: &mut PhysicalExecutorComparisonV1| {
                value.route_plan_digest = "alternative".to_owned()
            },
            |value: &mut PhysicalExecutorComparisonV1| {
                value.physical_input_digest = "other-input".to_owned()
            },
            |value: &mut PhysicalExecutorComparisonV1| {
                value.executor_source_digest = "other-source".to_owned()
            },
        ] {
            let mut tampered = original.clone();
            mutate(&mut tampered);
            assert!(tampered.validate().is_err());
        }
    }

    #[test]
    fn physical_runner_rejects_nonempty_output_before_resolving_inputs() {
        let output_dir = temp_dir("nonempty");
        fs::create_dir_all(&output_dir).expect("temp output should be created");
        fs::write(output_dir.join("owned"), b"occupied").expect("marker should be written");
        let error = run_physical_witness_development(
            Path::new("missing-manifest.json"),
            Path::new("missing-root"),
            &output_dir,
        )
        .expect_err("nonempty root must fail before manifest resolution");
        assert!(error.to_string().contains("must be empty"));
        fs::remove_dir_all(output_dir).expect("temp output should be removable");
    }

    #[test]
    fn comparison_runner_rejects_nonempty_output_before_opening_sources() {
        let output_dir = temp_dir("comparison_nonempty");
        fs::create_dir_all(&output_dir).expect("temp output should be created");
        fs::write(output_dir.join("owned"), b"occupied").expect("marker should be written");
        let error = run_physical_executor_comparison(
            Path::new("missing-physical-root"),
            Path::new("missing-executor-root"),
            &output_dir,
        )
        .expect_err("nonempty root must fail before source loading");
        assert!(error.to_string().contains("must be empty"));
        fs::remove_dir_all(output_dir).expect("temp output should be removable");
    }

    #[test]
    fn physical_summary_rejects_a_reduced_w3_catalog() {
        let witness = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("witness configuration should seal");
        let proposal =
            finite_template_proposal_configuration_v1(1).expect("small proposal should seal");
        let summary = PhysicalWitnessDevelopmentSummaryV1 {
            schema_id: PHYSICAL_WITNESS_DEVELOPMENT_SCHEMA_ID.to_owned(),
            schema_version: PHYSICAL_EXECUTOR_SCHEMA_VERSION,
            source_d0_input_digest: "source".to_owned(),
            witness_configuration_digest: witness.configuration_digest,
            proposal_configuration_digest: proposal.configuration_digest,
            search_limit: 1,
            case_count: 1,
            baseline_count: 1,
            diagnostic_count: 0,
            supported_count: 0,
            unknown_count: 1,
            case_digests: vec!["case".to_owned()],
            summary_digest: "digest".to_owned(),
        };

        assert_eq!(
            summary.validate().expect_err("reduced catalog must fail"),
            "W4 physical summary must use the complete W3 catalog"
        );
    }

    #[test]
    fn compact_physical_case_replays_exactly_and_rejects_resealed_plan_tamper() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("pd-eval should have a repository parent")
            .to_path_buf();
        let manifest = repo_root.join("fixtures/manifests/source_transition_d0a_development.json");
        let preparations = build_candidate_replay_development_preparations(&manifest, &repo_root)
            .expect("development preparations should resolve");
        let preparation = preparations
            .first()
            .expect("development corpus should be nonempty");
        let witness = BoundedTrajectoryWitnessConfigurationV1::v1()
            .seal()
            .expect("witness configuration should seal");
        let proposal =
            finite_template_proposal_configuration_v1(1).expect("small test proposal should seal");
        let case = build_physical_case(preparation, &witness, &proposal)
            .expect("compact physical case should build");
        case.validate_against_preparation_exact(preparation)
            .expect("compact case should reproduce the exact W3 result");

        let mut tampered = case.clone();
        tampered.route_plan_digest = "alternative-plan".to_owned();
        tampered
            .seal()
            .expect("self-consistent tampered case should reseal");
        assert!(
            tampered
                .validate_against_preparation_exact(preparation)
                .is_err()
        );
    }
}
