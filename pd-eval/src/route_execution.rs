//! Neutral, post-run route-wide execution evidence (D0b).
//!
//! The route extractor intentionally has a smaller input boundary than the
//! post-run assembler.  [`RouteExecutionKernelInput`] contains only resolved
//! physical/route context, the complete physics-rate sample stream, and its
//! cadence.  Terminal metadata is admitted only by the assembler for sound
//! prefix classification; controller updates are retained only in the audit
//! sidecar.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use pd_control::{
    ControlledRunArtifacts, ControllerMarker, ControllerUpdateRecord, TelemetryValue,
};
use pd_core::{
    NormalizedRouteGeometry, RoutePlan, RoutePlanningRequest, SafetyProfile, SampleRecord,
    ScenarioSpec, TerrainDefinition, TransferWaypointSpec, Vec2, VehicleInitialState, VehicleSpec,
    WaypointHandoffAssessment, WaypointHandoffKinematics, build_endpoint_profile,
};
use serde::{Deserialize, Serialize};

use super::review::neutral_waypoint_sample_stats;
use super::source_transition::{
    SourceTransitionAuditInput, SourceTransitionCadence, SourceTransitionClearance,
    SourceTransitionCrossingDirection, SourceTransitionEvidence, SourceTransitionEvidenceStatus,
    SourceTransitionKernelInput, SourceTransitionProvenance, SourceTransitionRawState,
    SourceTransitionSampleIdentity, SourceTransitionTerminal,
    assemble_source_transition_evidence_from_artifacts, digest_serialized,
    extract_source_transition_kernel, point_clearance_record, raw_state,
    source_transition_provenance_for_route_plan, world_to_normalized,
};

/// Version of the serialized D0b route-evidence schema.
pub const ROUTE_EXECUTION_SCHEMA_VERSION: u32 = 1;
/// Version of the deterministic D0b extractor implementation.
pub const ROUTE_EXECUTION_EXTRACTOR_VERSION: &str = "route_execution_d0b_v1";
const ROUTE_EXECUTION_TOLERANCE_M: f64 = 1.0e-9;

/// Neutral route-evidence status.  This is not a controller or mission
/// outcome label.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RouteExecutionEvidenceStatus {
    #[default]
    Invalid,
    Complete,
    CensoredBeforeWaypoint {
        waypoint_index: usize,
    },
}

impl RouteExecutionEvidenceStatus {
    pub fn code(&self) -> String {
        match self {
            Self::Invalid => "invalid".to_owned(),
            Self::Complete => "complete".to_owned(),
            Self::CensoredBeforeWaypoint { waypoint_index } => {
                format!("censored_before_waypoint_{waypoint_index}")
            }
        }
    }
}

/// Stable malformed/unverifiable-input reasons for the route extractor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RouteExecutionInvalidReason {
    DirectRouteOutOfScope,
    MissingSamples,
    MissingPhysicsRate,
    WrongCadence {
        sample_hz: u32,
        physics_hz: u32,
    },
    InvalidGeometry {
        message: String,
    },
    MissingTerminalCensor {
        waypoint_index: usize,
    },
    InvalidTerminalReason,
    TerminalStepMismatch {
        terminal_step: u64,
        last_sample_step: u64,
    },
    SourceEvidenceInvalid {
        reason: String,
    },
    SampleClearance {
        sample_index: usize,
        message: String,
    },
    Kinematics {
        waypoint_index: usize,
        sample_index: usize,
    },
}

impl std::fmt::Display for RouteExecutionInvalidReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DirectRouteOutOfScope => formatter.write_str("direct_route_out_of_scope"),
            Self::MissingSamples => formatter.write_str("missing_samples"),
            Self::MissingPhysicsRate => formatter.write_str("missing_physics_rate"),
            Self::WrongCadence {
                sample_hz,
                physics_hz,
            } => {
                write!(formatter, "wrong_cadence:{sample_hz}!={physics_hz}")
            }
            Self::InvalidGeometry { message } => write!(formatter, "invalid_geometry:{message}"),
            Self::MissingTerminalCensor { waypoint_index } => {
                write!(
                    formatter,
                    "missing_terminal_censor:waypoint_{waypoint_index}"
                )
            }
            Self::InvalidTerminalReason => formatter.write_str("invalid_terminal_reason"),
            Self::TerminalStepMismatch {
                terminal_step,
                last_sample_step,
            } => write!(
                formatter,
                "terminal_step_mismatch:{terminal_step}!={last_sample_step}"
            ),
            Self::SourceEvidenceInvalid { reason } => write!(formatter, "source_evidence:{reason}"),
            Self::SampleClearance {
                sample_index,
                message,
            } => {
                write!(formatter, "sample_clearance:{sample_index}:{message}")
            }
            Self::Kinematics {
                waypoint_index,
                sample_index,
            } => {
                write!(formatter, "kinematics:{waypoint_index}:{sample_index}")
            }
        }
    }
}

/// Region classification for a mapped physical sample.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteExecutionSampleRegion {
    BeforeSource,
    Centerline,
    AfterScope,
}

/// A route path-relative decomposition in the normalized source-relative
/// frame.  `cross_track_m` is signed; raw progress and region are retained so
/// backtracking and re-entry cannot be hidden by projection.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionPathMetrics {
    pub normalized_position_m: Vec2,
    pub normalized_route_progress: f64,
    pub along_route_m: f64,
    pub along_track_m: f64,
    pub cross_track_m: f64,
    pub velocity_along_route_mps: f64,
    pub velocity_cross_route_mps: f64,
    pub segment_index: Option<usize>,
    pub route_leg_index: Option<usize>,
    pub region: RouteExecutionSampleRegion,
}

/// Every retained physics-rate observation, including its exact clearances.
/// This physical record contains no controller phase, marker, identity, or
/// outcome-derived field; those remain audit-only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionSample {
    pub sample_index: usize,
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub source_progress_m: f64,
    pub state: SourceTransitionRawState,
    pub path: RouteExecutionPathMetrics,
    pub clearance: SourceTransitionClearance,
}

/// Scalar minimum/maximum pair used by route-leg extrema.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionExtremaValue {
    pub min: f64,
    pub max: f64,
}

impl RouteExecutionExtremaValue {
    fn from_values(values: impl Iterator<Item = f64>) -> Option<Self> {
        let mut values = values.peekable();
        let first = values.peek().copied()?;
        let mut min = first;
        let mut max = first;
        for value in values {
            min = min.min(value);
            max = max.max(value);
        }
        Some(Self { min, max })
    }
}

/// Extrema over the samples assigned to one leg range.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionExtrema {
    pub first_sample_index: usize,
    pub last_sample_index: usize,
    pub source_progress_m: RouteExecutionExtremaValue,
    pub normalized_route_progress: RouteExecutionExtremaValue,
    pub along_route_m: RouteExecutionExtremaValue,
    pub along_track_m: RouteExecutionExtremaValue,
    pub cross_track_m: RouteExecutionExtremaValue,
    pub velocity_along_route_mps: RouteExecutionExtremaValue,
    pub velocity_cross_route_mps: RouteExecutionExtremaValue,
    pub speed_mps: RouteExecutionExtremaValue,
    pub vertical_speed_mps: RouteExecutionExtremaValue,
    pub attitude_rad: RouteExecutionExtremaValue,
    pub angular_rate_radps: RouteExecutionExtremaValue,
    pub mass_kg: RouteExecutionExtremaValue,
    pub fuel_kg: RouteExecutionExtremaValue,
    pub observed_hull_clearance_m: RouteExecutionExtremaValue,
    pub observed_touchdown_clearance_m: RouteExecutionExtremaValue,
    pub contact_clearance_m: RouteExecutionExtremaValue,
    pub resolved_clearance_m: RouteExecutionExtremaValue,
    pub full_clearance_m: RouteExecutionExtremaValue,
}

/// A contiguous run of samples assigned to one route leg.  Multiple ranges
/// are retained when the vehicle backtracks across a segment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionSampleRange {
    pub first_sample_index: usize,
    pub last_sample_index: usize,
    pub sample_indices: Vec<usize>,
}

/// Stable identity for the boundaries of a route leg.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionBoundaryIdentity {
    pub kind: String,
    pub waypoint_index: Option<usize>,
    pub waypoint_id: Option<String>,
    pub sample_index: Option<usize>,
    pub physics_step: Option<u64>,
    pub sim_time_s: Option<f64>,
}

/// The immutable contract available at a leg's outbound handoff boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionAdmissibleInput {
    pub waypoint_index: usize,
    pub waypoint_id: String,
    pub contract: TransferWaypointSpec,
}

/// Per-leg mapped samples, ranges, extrema, and declared boundary identities.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionLegEvidence {
    pub leg_index: usize,
    pub start_boundary: RouteExecutionBoundaryIdentity,
    pub end_boundary: RouteExecutionBoundaryIdentity,
    pub admissible_input: Option<RouteExecutionAdmissibleInput>,
    pub sample_ranges: Vec<RouteExecutionSampleRange>,
    pub extrema: Option<RouteExecutionExtrema>,
}

/// Whether a route waypoint opportunity resolved by a neutral contract pass
/// or by reaching its deadline without a pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteExecutionResolutionKind {
    ContractPass,
    Deadline,
    InitialDeadline,
}

/// First/last raw values for a directed adjacent-pair crossing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionBracket {
    pub before: SourceTransitionRawState,
    pub after: SourceTransitionRawState,
    pub before_value: f64,
    pub after_value: f64,
    pub boundary_value: f64,
    pub interpolation_fraction: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteExecutionCrossingKind {
    CaptureWindow,
    DeadlinePlane,
}

/// A later forward/reverse crossing retained for audit and re-entry analysis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionCrossing {
    pub kind: RouteExecutionCrossingKind,
    pub direction: SourceTransitionCrossingDirection,
    pub before_sample_index: usize,
    pub after_sample_index: usize,
    pub before_value: f64,
    pub after_value: f64,
    pub boundary_value: f64,
}

/// A serializable snapshot of the canonical core handoff assessment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionAssessment {
    pub triggered: bool,
    pub capture_window_open: bool,
    pub deadline_reached: bool,
    pub spatial_pass: bool,
    pub envelope_pass: bool,
    pub contract_pass: bool,
    pub contract_pass_in_window: bool,
    pub violations: Vec<String>,
}

/// Full neutral kinematics and assessment for one opportunity sample.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionOpportunitySample {
    pub sample: RouteExecutionSample,
    pub kinematics: WaypointHandoffKinematicsRecord,
    pub assessment: RouteExecutionAssessment,
    pub window_seen: bool,
}

/// Serializable form of [`WaypointHandoffKinematics`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointHandoffKinematicsRecord {
    pub distance_m: f64,
    pub cross_track_m: f64,
    pub plane_progress_m: f64,
    pub outbound_heading_error_rad: f64,
    pub outbound_progress_mps: f64,
    pub outbound_cross_speed_mps: f64,
    pub speed_mps: f64,
    pub vertical_speed_mps: f64,
}

impl From<WaypointHandoffKinematics> for WaypointHandoffKinematicsRecord {
    fn from(value: WaypointHandoffKinematics) -> Self {
        Self {
            distance_m: value.distance_m,
            cross_track_m: value.cross_track_m,
            plane_progress_m: value.plane_progress_m,
            outbound_heading_error_rad: value.outbound_heading_error_rad,
            outbound_progress_mps: value.outbound_progress_mps,
            outbound_cross_speed_mps: value.outbound_cross_speed_mps,
            speed_mps: value.speed_mps,
            vertical_speed_mps: value.vertical_speed_mps,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionResolution {
    pub kind: RouteExecutionResolutionKind,
    pub sample_index: usize,
    pub physics_step: u64,
    pub sim_time_s: f64,
}

/// One waypoint's immutable contract and complete neutral opportunity series.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionWaypointEvidence {
    pub waypoint_index: usize,
    pub waypoint_id: String,
    pub contract: TransferWaypointSpec,
    pub capture_entry: Option<RouteExecutionBracket>,
    pub initially_inside: bool,
    pub deadline: Option<RouteExecutionBracket>,
    pub initially_at_deadline: bool,
    pub opportunity: Vec<RouteExecutionOpportunitySample>,
    pub first_contract_pass: Option<RouteExecutionOpportunitySample>,
    pub resolution: Option<RouteExecutionResolution>,
    pub crossings: Vec<RouteExecutionCrossing>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteExecutionPathEventKind {
    Backtracking,
    Reentry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionPathEvent {
    pub kind: RouteExecutionPathEventKind,
    pub before_sample_index: usize,
    pub after_sample_index: usize,
    pub before_progress_m: f64,
    pub after_progress_m: f64,
}

/// Audit fields intentionally excluded from the physical digest.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionAudit {
    pub sample_ordering: Vec<SourceTransitionSampleIdentity>,
    pub controller_handoff_markers: Vec<RouteExecutionControllerMarkerAudit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionControllerMarkerAudit {
    pub update_index: u64,
    pub physics_step: u64,
    pub sim_time_s: f64,
    pub phase: Option<String>,
    pub marker: ControllerMarker,
}

/// Route-specific identities kept separate from source, controller, outcome,
/// capability, and prediction identities.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionProvenance {
    pub resolved_input_digest: String,
    pub request_digest: String,
    pub policy_digest: String,
    pub route_plan_digest: String,
    pub source_target_geometry_digest: String,
    pub raw_bundle_digest: String,
    pub samples_digest: String,
    pub action_log_digest: String,
    pub source_evidence_digest: String,
    pub artifact_identity: String,
}

/// Serialized neutral D0b evidence.  Physical/kernel fields contain no
/// controller identity, phase, marker, seed, outcome, route label, or
/// prediction/capability digest.  The separate audit sidecar may carry the
/// controller's handoff marker and its phase for forensic comparison only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionEvidence {
    pub schema_version: u32,
    pub extractor_version: String,
    pub evidence_digest: String,
    pub physical_digest: String,
    pub status: RouteExecutionEvidenceStatus,
    pub invalid_reason: Option<RouteExecutionInvalidReason>,
    pub provenance: RouteExecutionProvenance,
    pub cadence: SourceTransitionCadence,
    pub source_transition: SourceTransitionEvidence,
    /// Exact normalized selected centerline used by the route kernel.
    pub selected_centerline_m: Vec<Vec2>,
    pub samples: Vec<RouteExecutionSample>,
    pub legs: Vec<RouteExecutionLegEvidence>,
    pub waypoints: Vec<RouteExecutionWaypointEvidence>,
    pub backtracking_events: Vec<RouteExecutionPathEvent>,
    pub reentry_events: Vec<RouteExecutionPathEvent>,
    pub terminal: Option<SourceTransitionTerminal>,
    pub audit: RouteExecutionAudit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionAuditSidecar {
    pub schema_version: u32,
    pub extractor_version: String,
    pub evidence_digest: String,
    pub status: RouteExecutionEvidenceStatus,
    pub provenance: RouteExecutionProvenance,
    pub cadence: SourceTransitionCadence,
    pub source_evidence_digest: String,
    pub terminal: Option<SourceTransitionTerminal>,
    pub audit: RouteExecutionAudit,
}

/// Pure route kernel context.  No terminal metadata, labels, controller
/// telemetry, outcome, scenario seed, or controller identity is admitted.
#[derive(Clone, Copy, Debug)]
pub struct RouteExecutionKernelInput<'a> {
    pub terrain: &'a TerrainDefinition,
    pub source_pad_center_x_m: f64,
    pub source_pad_surface_y_m: f64,
    pub target_pad_center_x_m: f64,
    pub target_pad_surface_y_m: f64,
    pub vehicle: &'a VehicleSpec,
    pub initial_state: &'a VehicleInitialState,
    pub geometry: &'a NormalizedRouteGeometry,
    pub profile: &'a SafetyProfile,
    pub selected_centerline_m: &'a [Vec2],
    pub waypoints: &'a [TransferWaypointSpec],
    pub samples: &'a [SampleRecord],
    pub cadence: SourceTransitionCadence,
}

#[derive(Clone, Debug)]
struct RouteGeometry {
    centerline: Vec<Vec2>,
    segment_leg_indices: Vec<Option<usize>>,
    cumulative_segment_lengths: Vec<f64>,
    normalized_span_m: f64,
    total_length_m: f64,
}

#[derive(Clone, Debug)]
pub struct RouteExecutionKernelOutput {
    pub samples: Vec<RouteExecutionSample>,
    pub legs: Vec<RouteExecutionLegEvidence>,
    pub waypoints: Vec<RouteExecutionWaypointEvidence>,
    pub backtracking_events: Vec<RouteExecutionPathEvent>,
    pub reentry_events: Vec<RouteExecutionPathEvent>,
    pub physical_digest: String,
    pub retained_end_index: usize,
}

/// Extract route evidence from full physics-rate samples.  Status and terminal
/// censoring are deliberately left to the post-run assembler.
pub fn extract_route_execution_kernel(
    input: &RouteExecutionKernelInput<'_>,
) -> Result<RouteExecutionKernelOutput, RouteExecutionInvalidReason> {
    if input.waypoints.is_empty() {
        return Err(RouteExecutionInvalidReason::DirectRouteOutOfScope);
    }
    if input.samples.is_empty() {
        return Err(RouteExecutionInvalidReason::MissingSamples);
    }
    let Some(sample_hz) = input.cadence.sample_hz else {
        return Err(RouteExecutionInvalidReason::MissingPhysicsRate);
    };
    if sample_hz != input.cadence.physics_hz {
        return Err(RouteExecutionInvalidReason::WrongCadence {
            sample_hz,
            physics_hz: input.cadence.physics_hz,
        });
    }
    for waypoint in input.waypoints {
        waypoint
            .validate()
            .map_err(|message| RouteExecutionInvalidReason::InvalidGeometry {
                message: format!("waypoint '{}': {message}", waypoint.id),
            })?;
    }
    let source_input = SourceTransitionKernelInput {
        terrain: input.terrain,
        source_pad_center_x_m: input.source_pad_center_x_m,
        vehicle: input.vehicle,
        initial_state: input.initial_state,
        geometry: input.geometry,
        profile: input.profile,
        selected_centerline_m: input.selected_centerline_m,
        waypoints: input.waypoints,
        samples: input.samples,
        cadence: input.cadence,
        audit: SourceTransitionAuditInput::default(),
    };
    let source_kernel = extract_source_transition_kernel(&source_input).map_err(|reason| {
        RouteExecutionInvalidReason::SourceEvidenceInvalid {
            reason: reason.to_string(),
        }
    })?;
    let tracking_after_index = source_kernel
        .tracking_entry
        .as_ref()
        .map(|tracking| tracking.after.sample_index);
    let geometry = resolve_route_geometry(input, tracking_after_index)?;
    let all_samples = input
        .samples
        .iter()
        .enumerate()
        .map(|(sample_index, sample)| {
            route_execution_sample(input, &geometry, sample_index, sample)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut waypoint_evidence = Vec::with_capacity(input.waypoints.len());
    let mut search_starts = Vec::with_capacity(input.waypoints.len());
    let mut next_search_start = tracking_after_index.unwrap_or(0);
    let mut retained_end_index = all_samples.len().saturating_sub(1);
    if tracking_after_index.is_none() {
        waypoint_evidence.extend(input.waypoints.iter().enumerate().map(
            |(waypoint_index, waypoint)| unresolved_waypoint_evidence(waypoint_index, waypoint),
        ));
    }
    for (waypoint_index, waypoint) in input.waypoints.iter().enumerate() {
        if tracking_after_index.is_none() {
            break;
        }
        search_starts.push(next_search_start);
        let next_target = if let Some(next) = input.waypoints.get(waypoint_index + 1) {
            next.position_m
        } else {
            Vec2::new(input.target_pad_center_x_m, input.target_pad_surface_y_m)
        };
        let anchor = if waypoint_index == 0 {
            Vec2::new(input.source_pad_center_x_m, input.source_pad_surface_y_m)
        } else {
            input.waypoints[waypoint_index - 1].position_m
        };
        let (evidence, resolution) = extract_waypoint_opportunity(
            &all_samples,
            waypoint_index,
            waypoint,
            anchor,
            next_target,
            next_search_start,
        )?;
        waypoint_evidence.push(evidence);
        if let Some(resolution) = resolution {
            next_search_start = resolution;
            retained_end_index = resolution;
        } else {
            retained_end_index = all_samples.len().saturating_sub(1);
            break;
        }
    }
    let retained_end_index = retained_end_index.min(all_samples.len().saturating_sub(1));
    let all_samples_for_crossings = all_samples.clone();
    let samples = all_samples
        .iter()
        .take(retained_end_index + 1)
        .cloned()
        .collect::<Vec<_>>();
    for waypoint_index in 0..waypoint_evidence.len() {
        let Some(&search_start) = search_starts.get(waypoint_index) else {
            continue;
        };
        let crossing_end = waypoint_evidence
            .get(waypoint_index + 1)
            .and_then(|next| next.resolution.as_ref())
            .map_or(retained_end_index, |resolution| resolution.sample_index)
            .min(retained_end_index);
        let anchor = if waypoint_index == 0 {
            Vec2::new(input.source_pad_center_x_m, input.source_pad_surface_y_m)
        } else {
            input.waypoints[waypoint_index - 1].position_m
        };
        let next_target = input
            .waypoints
            .get(waypoint_index + 1)
            .map(|next| next.position_m)
            .unwrap_or(Vec2::new(
                input.target_pad_center_x_m,
                input.target_pad_surface_y_m,
            ));
        let crossings = route_waypoint_crossings(
            &all_samples_for_crossings,
            search_start,
            crossing_end,
            &input.waypoints[waypoint_index],
            anchor,
            next_target,
        );
        waypoint_evidence[waypoint_index].crossings = crossings;
    }
    let legs = build_leg_evidence(input, &samples, tracking_after_index, &waypoint_evidence);
    let (backtracking_events, reentry_events) = path_events(&samples);
    let physical_digest = digest_serialized(&(
        input.selected_centerline_m,
        &samples,
        &legs,
        &waypoint_evidence,
        &backtracking_events,
        &reentry_events,
    ));
    Ok(RouteExecutionKernelOutput {
        samples,
        legs,
        waypoints: waypoint_evidence,
        backtracking_events,
        reentry_events,
        physical_digest,
        retained_end_index,
    })
}

/// Assemble route evidence with nested D0a source evidence.  Terminal metadata
/// is used only for censor classification and audit.
pub fn assemble_route_execution_evidence(
    input: &RouteExecutionKernelInput<'_>,
    source_evidence: SourceTransitionEvidence,
    mut provenance: RouteExecutionProvenance,
    terminal: Option<SourceTransitionTerminal>,
    controller_updates: &[ControllerUpdateRecord],
) -> RouteExecutionEvidence {
    provenance.source_evidence_digest = source_evidence.evidence_digest.clone();
    if source_evidence.status == SourceTransitionEvidenceStatus::Invalid {
        let reason = source_invalid_route_reason(&source_evidence).unwrap_or(
            RouteExecutionInvalidReason::SourceEvidenceInvalid {
                reason: "nested source evidence is invalid".to_owned(),
            },
        );
        return invalid_route_evidence(
            input.cadence,
            input.samples,
            source_evidence,
            provenance,
            terminal,
            controller_updates,
            reason,
        );
    }
    let kernel = match extract_route_execution_kernel(input) {
        Ok(kernel) => kernel,
        Err(reason) => {
            return invalid_route_evidence(
                input.cadence,
                input.samples,
                source_evidence,
                provenance,
                terminal,
                controller_updates,
                reason,
            );
        }
    };
    let (status, invalid_reason) = classify_route_status(input, &kernel, terminal.as_ref());
    let evidence = RouteExecutionEvidence {
        schema_version: ROUTE_EXECUTION_SCHEMA_VERSION,
        extractor_version: ROUTE_EXECUTION_EXTRACTOR_VERSION.to_owned(),
        evidence_digest: String::new(),
        physical_digest: String::new(),
        status,
        invalid_reason,
        provenance,
        cadence: input.cadence,
        source_transition: source_evidence,
        selected_centerline_m: input.selected_centerline_m.to_vec(),
        samples: kernel.samples,
        legs: kernel.legs,
        waypoints: kernel.waypoints,
        backtracking_events: kernel.backtracking_events,
        reentry_events: kernel.reentry_events,
        terminal,
        audit: route_execution_audit(input.samples, controller_updates),
    };
    seal_route_execution_evidence(evidence)
}

/// Convenience wrapper for a controller-produced full physics-rate bundle.
pub fn assemble_route_execution_evidence_from_controlled_artifacts(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &ControlledRunArtifacts,
    provenance: SourceTransitionProvenance,
) -> RouteExecutionEvidence {
    let source_evidence = assemble_source_transition_evidence_from_artifacts(
        scenario,
        route_plan,
        &artifacts.run,
        &artifacts.controller_updates,
        provenance.clone(),
    );
    let terminal =
        super::source_transition::source_transition_terminal_from_manifest(&artifacts.run.manifest);
    let context = match route_execution_context(scenario, route_plan, &provenance) {
        Ok(context) => context,
        Err(reason) => {
            let reason = source_invalid_route_reason(&source_evidence).unwrap_or(reason);
            return invalid_route_evidence_from_context(
                RouteExecutionInvalidContext {
                    scenario,
                    route_plan,
                    source_evidence,
                    source_provenance: provenance,
                    terminal,
                    samples: &artifacts.run.samples,
                    controller_updates: &artifacts.controller_updates,
                },
                reason,
            );
        }
    };
    let input = context.kernel_input(scenario, route_plan, &artifacts.run.samples);
    let route_provenance = context.provenance.clone();
    assemble_route_execution_evidence(
        &input,
        source_evidence,
        route_provenance,
        terminal,
        &artifacts.controller_updates,
    )
}

/// Build a D0b route artifact directly from ordinary run artifacts.
pub fn assemble_route_execution_evidence_from_artifacts(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &pd_core::RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
    provenance: SourceTransitionProvenance,
) -> RouteExecutionEvidence {
    let source_evidence = assemble_source_transition_evidence_from_artifacts(
        scenario,
        route_plan,
        artifacts,
        controller_updates,
        provenance.clone(),
    );
    let terminal =
        super::source_transition::source_transition_terminal_from_manifest(&artifacts.manifest);
    let context = match route_execution_context(scenario, route_plan, &provenance) {
        Ok(context) => context,
        Err(reason) => {
            let reason = source_invalid_route_reason(&source_evidence).unwrap_or(reason);
            return invalid_route_evidence_from_context(
                RouteExecutionInvalidContext {
                    scenario,
                    route_plan,
                    source_evidence,
                    source_provenance: provenance,
                    terminal,
                    samples: &artifacts.samples,
                    controller_updates,
                },
                reason,
            );
        }
    };
    let input = context.kernel_input(scenario, route_plan, &artifacts.samples);
    let route_provenance = context.provenance.clone();
    assemble_route_execution_evidence(
        &input,
        source_evidence,
        route_provenance,
        terminal,
        controller_updates,
    )
}

struct RouteExecutionResolvedContext {
    source_pad_center_x_m: f64,
    source_pad_surface_y_m: f64,
    target_pad_center_x_m: f64,
    target_pad_surface_y_m: f64,
    profile: SafetyProfile,
    selected_centerline: Vec<Vec2>,
    provenance: RouteExecutionProvenance,
}

impl RouteExecutionResolvedContext {
    fn kernel_input<'a>(
        &'a self,
        scenario: &'a ScenarioSpec,
        route_plan: &'a RoutePlan,
        samples: &'a [SampleRecord],
    ) -> RouteExecutionKernelInput<'a> {
        RouteExecutionKernelInput {
            terrain: &scenario.world.terrain,
            source_pad_center_x_m: self.source_pad_center_x_m,
            source_pad_surface_y_m: self.source_pad_surface_y_m,
            target_pad_center_x_m: self.target_pad_center_x_m,
            target_pad_surface_y_m: self.target_pad_surface_y_m,
            vehicle: &scenario.vehicle,
            initial_state: &scenario.initial_state,
            geometry: &route_plan.normalized_geometry,
            profile: &self.profile,
            selected_centerline_m: &self.selected_centerline,
            waypoints: &route_plan.route.waypoints,
            samples,
            cadence: SourceTransitionCadence {
                sample_hz: scenario.sim.sample_hz,
                physics_hz: scenario.sim.physics_hz,
            },
        }
    }
}

fn route_execution_context(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    source_provenance: &SourceTransitionProvenance,
) -> Result<RouteExecutionResolvedContext, RouteExecutionInvalidReason> {
    let route = &route_plan.route;
    if route.waypoints.is_empty() {
        return Err(RouteExecutionInvalidReason::DirectRouteOutOfScope);
    }
    if scenario.mission.transfer_route.as_ref() != Some(route) {
        return Err(RouteExecutionInvalidReason::InvalidGeometry {
            message: "scenario transfer route does not match route plan".to_owned(),
        });
    }
    let source_pad = scenario
        .world
        .landing_pad(&route.source_pad_id)
        .ok_or_else(|| RouteExecutionInvalidReason::InvalidGeometry {
            message: "route source pad is not present in scenario world".to_owned(),
        })?;
    let target_pad = scenario
        .world
        .landing_pad(&route.target_pad_id)
        .ok_or_else(|| RouteExecutionInvalidReason::InvalidGeometry {
            message: "route target pad is not present in scenario world".to_owned(),
        })?;
    let request = RoutePlanningRequest {
        world: scenario.world.clone(),
        vehicle: scenario.vehicle.clone(),
        initial_state: scenario.initial_state.clone(),
        source_pad_id: source_pad.id.clone(),
        target_pad_id: target_pad.id.clone(),
        policy: route_plan.policy.clone(),
    };
    let (profile, _) = build_endpoint_profile(
        &request,
        route_plan.normalized_geometry.direct_horizontal_span_m,
    )
    .map_err(|error| RouteExecutionInvalidReason::InvalidGeometry {
        message: error.to_string(),
    })?;
    let selected_centerline = route_plan
        .diagnostics
        .selected_centerline_m
        .iter()
        .copied()
        .map(|point| {
            world_to_normalized(
                point,
                source_pad.center_x_m,
                route_plan.normalized_geometry.horizontal_sign,
            )
        })
        .collect::<Vec<_>>();
    let provenance = RouteExecutionProvenance {
        resolved_input_digest: source_provenance.resolved_input_digest.clone(),
        request_digest: source_provenance.request_digest.clone(),
        policy_digest: source_provenance.policy_digest.clone(),
        route_plan_digest: source_provenance.route_plan_digest.clone(),
        source_target_geometry_digest: source_provenance.source_target_geometry_digest.clone(),
        raw_bundle_digest: source_provenance.raw_bundle_digest.clone(),
        samples_digest: source_provenance.samples_digest.clone(),
        action_log_digest: source_provenance.action_log_digest.clone(),
        source_evidence_digest: String::new(),
        artifact_identity: source_provenance.artifact_identity.clone(),
    };
    Ok(RouteExecutionResolvedContext {
        source_pad_center_x_m: source_pad.center_x_m,
        source_pad_surface_y_m: source_pad.surface_y_m,
        target_pad_center_x_m: target_pad.center_x_m,
        target_pad_surface_y_m: target_pad.surface_y_m,
        profile,
        selected_centerline,
        provenance,
    })
}

fn resolve_route_geometry(
    input: &RouteExecutionKernelInput<'_>,
    tracking_after_index: Option<usize>,
) -> Result<RouteGeometry, RouteExecutionInvalidReason> {
    if input.geometry.horizontal_sign != -1 && input.geometry.horizontal_sign != 1 {
        return Err(RouteExecutionInvalidReason::InvalidGeometry {
            message: "horizontal sign must be +/-1".to_owned(),
        });
    }
    let centerline = input.selected_centerline_m.to_vec();
    if centerline.len() < 2
        || centerline
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        || centerline.windows(2).any(|pair| pair[1].x <= pair[0].x)
    {
        return Err(RouteExecutionInvalidReason::InvalidGeometry {
            message: "selected centerline must be finite and strictly ordered".to_owned(),
        });
    }
    if centerline[0].x.abs() > ROUTE_EXECUTION_TOLERANCE_M
        || (centerline.last().map_or(0.0, |point| point.x)
            - input.geometry.direct_horizontal_span_m)
            .abs()
            > ROUTE_EXECUTION_TOLERANCE_M
    {
        return Err(RouteExecutionInvalidReason::InvalidGeometry {
            message: "selected centerline must cover normalized route scope".to_owned(),
        });
    }
    let tracking_segment_index =
        centerline_segment_index(input.profile.source_transition_end_m, &centerline).ok_or_else(
            || RouteExecutionInvalidReason::InvalidGeometry {
                message: "tracking entry is outside selected centerline".to_owned(),
            },
        )?;
    if tracking_after_index.is_some_and(|index| index >= input.samples.len()) {
        return Err(RouteExecutionInvalidReason::InvalidGeometry {
            message: "tracking entry sample index is outside raw samples".to_owned(),
        });
    }
    let mut waypoint_centerline_indices = Vec::with_capacity(input.waypoints.len());
    let mut previous_x = input.profile.source_transition_end_m;
    for waypoint in input.waypoints {
        let normalized = world_to_normalized(
            waypoint.position_m,
            input.source_pad_center_x_m,
            input.geometry.horizontal_sign,
        );
        if !normalized.x.is_finite()
            || normalized.x <= previous_x + ROUTE_EXECUTION_TOLERANCE_M
            || normalized.x > input.geometry.direct_horizontal_span_m
        {
            return Err(RouteExecutionInvalidReason::InvalidGeometry {
                message: format!(
                    "waypoint '{}' is not strictly after its route anchor",
                    waypoint.id
                ),
            });
        }
        let index = centerline
            .iter()
            .position(|point| (*point - normalized).length() <= ROUTE_EXECUTION_TOLERANCE_M)
            .ok_or_else(|| RouteExecutionInvalidReason::InvalidGeometry {
                message: format!(
                    "waypoint '{}' is not a selected centerline point",
                    waypoint.id
                ),
            })?;
        if index <= tracking_segment_index {
            return Err(RouteExecutionInvalidReason::InvalidGeometry {
                message: format!("waypoint '{}' is not after tracking entry", waypoint.id),
            });
        }
        waypoint_centerline_indices.push(index);
        previous_x = normalized.x;
    }
    let mut segment_leg_indices = vec![None; centerline.len() - 1];
    for (segment_index, slot) in segment_leg_indices
        .iter_mut()
        .enumerate()
        .skip(tracking_segment_index)
    {
        let leg_index = input
            .waypoints
            .iter()
            .enumerate()
            .find(|(index, _)| segment_index < waypoint_centerline_indices[*index])
            .map(|(index, _)| index)
            .unwrap_or(input.waypoints.len());
        *slot = Some(leg_index);
    }
    let mut cumulative_segment_lengths = vec![0.0; centerline.len()];
    for (index, pair) in centerline.windows(2).enumerate() {
        let length = (pair[1] - pair[0]).length();
        if !length.is_finite() || length <= ROUTE_EXECUTION_TOLERANCE_M {
            return Err(RouteExecutionInvalidReason::InvalidGeometry {
                message: format!("centerline segment {index} has zero length"),
            });
        }
        cumulative_segment_lengths[index + 1] = cumulative_segment_lengths[index] + length;
    }
    let total_length_m = cumulative_segment_lengths.last().copied().unwrap_or(0.0);
    Ok(RouteGeometry {
        centerline,
        segment_leg_indices,
        cumulative_segment_lengths,
        normalized_span_m: input.geometry.direct_horizontal_span_m,
        total_length_m,
    })
}

fn route_execution_sample(
    input: &RouteExecutionKernelInput<'_>,
    geometry: &RouteGeometry,
    sample_index: usize,
    sample: &SampleRecord,
) -> Result<RouteExecutionSample, RouteExecutionInvalidReason> {
    let state = raw_state(sample_index, sample);
    let source_progress_m = f64::from(input.geometry.horizontal_sign)
        * (state.position_m.x - input.source_pad_center_x_m);
    if !source_progress_m.is_finite() {
        return Err(RouteExecutionInvalidReason::InvalidGeometry {
            message: format!("sample {sample_index} source progress is non-finite"),
        });
    }
    let path = route_path_metrics(
        input,
        geometry,
        state.position_m,
        state.velocity_mps,
        source_progress_m,
    );
    let clearance_for = |envelope: pd_core::CorridorEnvelope| {
        input
            .terrain
            .exact_point_clearance(state.position_m, envelope)
            .map_err(|error| RouteExecutionInvalidReason::SampleClearance {
                sample_index,
                message: error.to_string(),
            })
    };
    let contact = clearance_for(input.profile.contact_envelope)?;
    let resolved = clearance_for(input.profile.envelope_at(source_progress_m))?;
    let full = clearance_for(input.profile.full_envelope)?;
    let result = RouteExecutionSample {
        sample_index,
        physics_step: sample.physics_step,
        sim_time_s: sample.sim_time_s,
        source_progress_m,
        state,
        path,
        clearance: SourceTransitionClearance {
            observed_hull_clearance_m: sample.observation.min_hull_clearance_m,
            observed_touchdown_clearance_m: Some(sample.observation.touchdown_clearance_m),
            contact_envelope: point_clearance_record(input.profile.contact_envelope, contact),
            resolved_envelope: point_clearance_record(
                input.profile.envelope_at(source_progress_m),
                resolved,
            ),
            full_envelope: point_clearance_record(input.profile.full_envelope, full),
        },
    };
    if !route_execution_sample_finite(&result) {
        return Err(RouteExecutionInvalidReason::InvalidGeometry {
            message: format!("sample {sample_index} derived route values are non-finite"),
        });
    }
    Ok(result)
}

fn unresolved_waypoint_evidence(
    waypoint_index: usize,
    waypoint: &TransferWaypointSpec,
) -> RouteExecutionWaypointEvidence {
    RouteExecutionWaypointEvidence {
        waypoint_index,
        waypoint_id: waypoint.id.clone(),
        contract: waypoint.clone(),
        capture_entry: None,
        initially_inside: false,
        deadline: None,
        initially_at_deadline: false,
        opportunity: Vec::new(),
        first_contract_pass: None,
        resolution: None,
        crossings: Vec::new(),
    }
}

fn route_path_metrics(
    input: &RouteExecutionKernelInput<'_>,
    geometry: &RouteGeometry,
    world_position: Vec2,
    world_velocity: Vec2,
    source_progress_m: f64,
) -> RouteExecutionPathMetrics {
    let normalized_position_m = Vec2::new(source_progress_m, world_position.y);
    let normalized_velocity = Vec2::new(
        world_velocity.x * f64::from(input.geometry.horizontal_sign),
        world_velocity.y,
    );
    let (region, segment_index) = if source_progress_m < geometry.centerline[0].x {
        (RouteExecutionSampleRegion::BeforeSource, None)
    } else if source_progress_m
        > geometry
            .centerline
            .last()
            .map_or(geometry.normalized_span_m, |point| point.x)
    {
        (RouteExecutionSampleRegion::AfterScope, None)
    } else {
        (
            RouteExecutionSampleRegion::Centerline,
            centerline_segment_index(source_progress_m, &geometry.centerline),
        )
    };
    let segment_index_for_projection = segment_index.unwrap_or_else(|| {
        if matches!(region, RouteExecutionSampleRegion::BeforeSource) {
            0
        } else {
            geometry.centerline.len() - 2
        }
    });
    let start = geometry.centerline[segment_index_for_projection];
    let end = geometry.centerline[segment_index_for_projection + 1];
    let tangent_vector = end - start;
    let tangent_length = tangent_vector.length();
    let tangent = tangent_vector * (1.0 / tangent_length);
    let delta = normalized_position_m - start;
    let along_segment = dot(delta, tangent);
    let normal = Vec2::new(-tangent.y, tangent.x);
    let along_route_m =
        geometry.cumulative_segment_lengths[segment_index_for_projection] + along_segment;
    RouteExecutionPathMetrics {
        normalized_position_m,
        normalized_route_progress: along_route_m / geometry.total_length_m,
        along_route_m,
        along_track_m: along_segment,
        cross_track_m: dot(delta, normal),
        velocity_along_route_mps: dot(normalized_velocity, tangent),
        velocity_cross_route_mps: dot(normalized_velocity, normal),
        segment_index,
        route_leg_index: segment_index.and_then(|index| geometry.segment_leg_indices[index]),
        region,
    }
}

fn extract_waypoint_opportunity(
    samples: &[RouteExecutionSample],
    waypoint_index: usize,
    waypoint: &TransferWaypointSpec,
    anchor: Vec2,
    next_target: Vec2,
    search_start: usize,
) -> Result<(RouteExecutionWaypointEvidence, Option<usize>), RouteExecutionInvalidReason> {
    if search_start >= samples.len() {
        return Err(RouteExecutionInvalidReason::Kinematics {
            waypoint_index,
            sample_index: search_start,
        });
    }
    let mut kinematics = Vec::with_capacity(samples.len().saturating_sub(search_start));
    for sample in &samples[search_start..] {
        let stats = neutral_waypoint_sample_stats(
            sample.state.position_m,
            sample.state.velocity_mps,
            anchor,
            waypoint.position_m,
            next_target,
            waypoint.handoff_tangent_unit,
        )
        .ok_or(RouteExecutionInvalidReason::Kinematics {
            waypoint_index,
            sample_index: sample.sample_index,
        })?;
        kinematics.push((sample, stats));
    }
    let initial_stats = kinematics[0].1;
    let initially_inside = initial_stats.distance_m <= waypoint.capture_radius_m;
    let initially_at_deadline = initial_stats.plane_progress_m >= 0.0;
    let mut capture_entry = None;
    let mut deadline = None;
    let mut first_deadline_position = None;
    for pair in kinematics.windows(2) {
        let (before_sample, before_stats) = pair[0];
        let (after_sample, after_stats) = pair[1];
        if capture_entry.is_none()
            && before_stats.distance_m > waypoint.capture_radius_m
            && waypoint.capture_radius_m >= after_stats.distance_m
        {
            capture_entry = Some(RouteExecutionBracket {
                before: before_sample.state,
                after: after_sample.state,
                before_value: before_stats.distance_m,
                after_value: after_stats.distance_m,
                boundary_value: waypoint.capture_radius_m,
                interpolation_fraction: fraction(
                    before_stats.distance_m,
                    after_stats.distance_m,
                    waypoint.capture_radius_m,
                ),
            });
        }
        if !initially_at_deadline
            && deadline.is_none()
            && before_stats.plane_progress_m < 0.0
            && after_stats.plane_progress_m >= 0.0
        {
            deadline = Some(RouteExecutionBracket {
                before: before_sample.state,
                after: after_sample.state,
                before_value: before_stats.plane_progress_m,
                after_value: after_stats.plane_progress_m,
                boundary_value: 0.0,
                interpolation_fraction: fraction(
                    before_stats.plane_progress_m,
                    after_stats.plane_progress_m,
                    0.0,
                ),
            });
            first_deadline_position = Some(after_sample.sample_index);
            break;
        }
    }
    if initially_at_deadline {
        first_deadline_position = Some(kinematics[0].0.sample_index);
    }
    let opportunity_start = if initially_at_deadline || initially_inside {
        Some(kinematics[0].0.sample_index)
    } else if let Some(entry) = capture_entry.as_ref() {
        Some(entry.after.sample_index)
    } else {
        first_deadline_position
    };
    let mut opportunity = Vec::new();
    let mut first_contract_pass = None;
    let mut resolution = None;
    let mut window_seen = false;
    let mut resolution_scan_end = samples
        .last()
        .map_or(search_start, |sample| sample.sample_index);
    if let Some(opportunity_start) = opportunity_start {
        for (sample, stats) in kinematics.iter().copied() {
            if sample.sample_index < opportunity_start {
                continue;
            }
            if stats.distance_m <= waypoint.capture_radius_m {
                window_seen = true;
            }
            let assessment = waypoint.assess_handoff(stats);
            let contract_pass_in_window = assessment.contract_pass_in_window(window_seen);
            let record = RouteExecutionOpportunitySample {
                sample: sample.clone(),
                kinematics: stats.into(),
                assessment: assessment_record(assessment, contract_pass_in_window),
                window_seen,
            };
            opportunity.push(record.clone());
            if initially_at_deadline && sample.sample_index == opportunity_start {
                if contract_pass_in_window {
                    first_contract_pass = Some(record.clone());
                }
                resolution = Some(RouteExecutionResolution {
                    kind: RouteExecutionResolutionKind::InitialDeadline,
                    sample_index: sample.sample_index,
                    physics_step: sample.physics_step,
                    sim_time_s: sample.sim_time_s,
                });
                resolution_scan_end = sample.sample_index;
                break;
            }
            if contract_pass_in_window {
                first_contract_pass = Some(record.clone());
                resolution = Some(RouteExecutionResolution {
                    kind: RouteExecutionResolutionKind::ContractPass,
                    sample_index: sample.sample_index,
                    physics_step: sample.physics_step,
                    sim_time_s: sample.sim_time_s,
                });
                resolution_scan_end = sample.sample_index;
                break;
            }
            if stats.plane_progress_m >= 0.0 {
                resolution = Some(RouteExecutionResolution {
                    kind: if sample.sample_index == opportunity_start && initially_at_deadline {
                        RouteExecutionResolutionKind::InitialDeadline
                    } else {
                        RouteExecutionResolutionKind::Deadline
                    },
                    sample_index: sample.sample_index,
                    physics_step: sample.physics_step,
                    sim_time_s: sample.sim_time_s,
                });
                resolution_scan_end = sample.sample_index;
                break;
            }
        }
    }
    if resolution.is_none() {
        resolution_scan_end = samples
            .last()
            .map_or(search_start, |sample| sample.sample_index);
    }
    let crossings = waypoint_crossings(&kinematics, waypoint.capture_radius_m, resolution_scan_end);
    Ok((
        RouteExecutionWaypointEvidence {
            waypoint_index,
            waypoint_id: waypoint.id.clone(),
            contract: waypoint.clone(),
            capture_entry,
            initially_inside,
            deadline,
            initially_at_deadline,
            opportunity,
            first_contract_pass,
            resolution: resolution.clone(),
            crossings,
        },
        resolution.map(|resolution| resolution.sample_index),
    ))
}

fn waypoint_crossings(
    kinematics: &[(&RouteExecutionSample, WaypointHandoffKinematics)],
    capture_radius_m: f64,
    resolution_end: usize,
) -> Vec<RouteExecutionCrossing> {
    let mut crossings = Vec::new();
    for pair in kinematics.windows(2) {
        let (before_sample, before_stats) = pair[0];
        let (after_sample, after_stats) = pair[1];
        if after_sample.sample_index > resolution_end {
            break;
        }
        let capture_direction = if before_stats.distance_m > capture_radius_m
            && capture_radius_m >= after_stats.distance_m
        {
            Some(SourceTransitionCrossingDirection::Forward)
        } else if before_stats.distance_m <= capture_radius_m
            && after_stats.distance_m > capture_radius_m
        {
            Some(SourceTransitionCrossingDirection::Reverse)
        } else {
            None
        };
        if let Some(direction) = capture_direction {
            crossings.push(RouteExecutionCrossing {
                kind: RouteExecutionCrossingKind::CaptureWindow,
                direction,
                before_sample_index: before_sample.sample_index,
                after_sample_index: after_sample.sample_index,
                before_value: before_stats.distance_m,
                after_value: after_stats.distance_m,
                boundary_value: capture_radius_m,
            });
        }
        let deadline_direction =
            if before_stats.plane_progress_m < 0.0 && after_stats.plane_progress_m >= 0.0 {
                Some(SourceTransitionCrossingDirection::Forward)
            } else if before_stats.plane_progress_m >= 0.0 && after_stats.plane_progress_m < 0.0 {
                Some(SourceTransitionCrossingDirection::Reverse)
            } else {
                None
            };
        if let Some(direction) = deadline_direction {
            crossings.push(RouteExecutionCrossing {
                kind: RouteExecutionCrossingKind::DeadlinePlane,
                direction,
                before_sample_index: before_sample.sample_index,
                after_sample_index: after_sample.sample_index,
                before_value: before_stats.plane_progress_m,
                after_value: after_stats.plane_progress_m,
                boundary_value: 0.0,
            });
        }
    }
    crossings
}

fn route_waypoint_crossings(
    samples: &[RouteExecutionSample],
    search_start: usize,
    resolution_end: usize,
    waypoint: &TransferWaypointSpec,
    anchor: Vec2,
    next_target: Vec2,
) -> Vec<RouteExecutionCrossing> {
    let kinematics = samples
        .iter()
        .filter(|sample| sample.sample_index >= search_start)
        .map(|sample| {
            (
                sample,
                neutral_waypoint_sample_stats(
                    sample.state.position_m,
                    sample.state.velocity_mps,
                    anchor,
                    waypoint.position_m,
                    next_target,
                    waypoint.handoff_tangent_unit,
                ),
            )
        })
        .filter_map(|(sample, stats)| stats.map(|stats| (sample, stats)))
        .collect::<Vec<_>>();
    waypoint_crossings(&kinematics, waypoint.capture_radius_m, resolution_end)
}

fn assessment_record(
    assessment: WaypointHandoffAssessment,
    contract_pass_in_window: bool,
) -> RouteExecutionAssessment {
    RouteExecutionAssessment {
        triggered: assessment.triggered,
        capture_window_open: assessment.capture_window_open,
        deadline_reached: assessment.deadline_reached,
        spatial_pass: assessment.spatial_pass,
        envelope_pass: assessment.envelope_pass,
        contract_pass: assessment.contract_pass(),
        contract_pass_in_window,
        violations: assessment
            .violations
            .into_iter()
            .map(|violation| violation.as_str().to_owned())
            .collect(),
    }
}

fn build_leg_evidence(
    input: &RouteExecutionKernelInput<'_>,
    samples: &[RouteExecutionSample],
    tracking_after_index: Option<usize>,
    waypoint_evidence: &[RouteExecutionWaypointEvidence],
) -> Vec<RouteExecutionLegEvidence> {
    let leg_count = input.waypoints.len() + 1;
    (0..leg_count)
        .map(|leg_index| {
            let indices = samples
                .iter()
                .filter(|sample| sample.path.route_leg_index == Some(leg_index))
                .map(|sample| sample.sample_index)
                .collect::<Vec<_>>();
            let sample_ranges = contiguous_ranges(&indices);
            let extrema = extrema_for_indices(samples, &indices);
            let start_boundary = if leg_index == 0 {
                let sample = tracking_after_index.and_then(|index| samples.get(index));
                boundary_identity(
                    if sample.is_some() {
                        "tracking_entry"
                    } else {
                        "tracking_entry_unreached"
                    },
                    None,
                    None,
                    sample,
                )
            } else {
                let waypoint = &input.waypoints[leg_index - 1];
                let sample = waypoint_evidence
                    .get(leg_index - 1)
                    .and_then(|evidence| evidence.resolution.as_ref())
                    .and_then(|resolution| samples.get(resolution.sample_index));
                boundary_identity(
                    if sample.is_some() {
                        "waypoint_resolution"
                    } else {
                        "waypoint_resolution_unreached"
                    },
                    Some(leg_index - 1),
                    Some(waypoint.id.clone()),
                    sample,
                )
            };
            let end_boundary = if leg_index < input.waypoints.len() {
                let waypoint = &input.waypoints[leg_index];
                let sample = waypoint_evidence
                    .get(leg_index)
                    .and_then(|evidence| evidence.resolution.as_ref())
                    .and_then(|resolution| samples.get(resolution.sample_index));
                boundary_identity(
                    if sample.is_some() {
                        "waypoint_resolution"
                    } else {
                        "waypoint_resolution_unreached"
                    },
                    Some(leg_index),
                    Some(waypoint.id.clone()),
                    sample,
                )
            } else {
                target_scope_boundary(samples, input.geometry.direct_horizontal_span_m)
            };
            RouteExecutionLegEvidence {
                leg_index,
                start_boundary,
                end_boundary,
                admissible_input: input.waypoints.get(leg_index).map(|waypoint| {
                    RouteExecutionAdmissibleInput {
                        waypoint_index: leg_index,
                        waypoint_id: waypoint.id.clone(),
                        contract: waypoint.clone(),
                    }
                }),
                sample_ranges,
                extrema,
            }
        })
        .collect()
}

fn target_scope_boundary(
    samples: &[RouteExecutionSample],
    normalized_span_m: f64,
) -> RouteExecutionBoundaryIdentity {
    let sample = samples
        .iter()
        .find(|sample| sample.source_progress_m >= normalized_span_m - ROUTE_EXECUTION_TOLERANCE_M);
    boundary_identity(
        if sample.is_some() {
            "target_scope"
        } else {
            "target_scope_unreached"
        },
        None,
        None,
        sample,
    )
}

fn boundary_identity(
    kind: &str,
    waypoint_index: Option<usize>,
    waypoint_id: Option<String>,
    sample: Option<&RouteExecutionSample>,
) -> RouteExecutionBoundaryIdentity {
    RouteExecutionBoundaryIdentity {
        kind: kind.to_owned(),
        waypoint_index,
        waypoint_id,
        sample_index: sample.map(|sample| sample.sample_index),
        physics_step: sample.map(|sample| sample.physics_step),
        sim_time_s: sample.map(|sample| sample.sim_time_s),
    }
}

fn contiguous_ranges(indices: &[usize]) -> Vec<RouteExecutionSampleRange> {
    let mut ranges = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for &index in indices {
        if current
            .last()
            .is_some_and(|last| index != last.saturating_add(1))
        {
            ranges.push(RouteExecutionSampleRange {
                first_sample_index: current[0],
                last_sample_index: *current.last().unwrap_or(&current[0]),
                sample_indices: std::mem::take(&mut current),
            });
        }
        current.push(index);
    }
    if !current.is_empty() {
        ranges.push(RouteExecutionSampleRange {
            first_sample_index: current[0],
            last_sample_index: *current.last().unwrap_or(&current[0]),
            sample_indices: current,
        });
    }
    ranges
}

fn extrema_for_indices(
    samples: &[RouteExecutionSample],
    indices: &[usize],
) -> Option<RouteExecutionExtrema> {
    let selected = indices
        .iter()
        .filter_map(|index| samples.get(*index))
        .collect::<Vec<_>>();
    let first = selected.first()?;
    let last = selected.last()?;
    let value = |values: Vec<f64>| {
        RouteExecutionExtremaValue::from_values(values.into_iter())
            .expect("extrema input is non-empty")
    };
    Some(RouteExecutionExtrema {
        first_sample_index: first.sample_index,
        last_sample_index: last.sample_index,
        source_progress_m: value(
            selected
                .iter()
                .map(|sample| sample.source_progress_m)
                .collect(),
        ),
        normalized_route_progress: value(
            selected
                .iter()
                .map(|sample| sample.path.normalized_route_progress)
                .collect(),
        ),
        along_route_m: value(
            selected
                .iter()
                .map(|sample| sample.path.along_route_m)
                .collect(),
        ),
        along_track_m: value(
            selected
                .iter()
                .map(|sample| sample.path.along_track_m)
                .collect(),
        ),
        cross_track_m: value(
            selected
                .iter()
                .map(|sample| sample.path.cross_track_m)
                .collect(),
        ),
        velocity_along_route_mps: value(
            selected
                .iter()
                .map(|sample| sample.path.velocity_along_route_mps)
                .collect(),
        ),
        velocity_cross_route_mps: value(
            selected
                .iter()
                .map(|sample| sample.path.velocity_cross_route_mps)
                .collect(),
        ),
        speed_mps: value(
            selected
                .iter()
                .map(|sample| sample.state.velocity_mps.length())
                .collect(),
        ),
        vertical_speed_mps: value(
            selected
                .iter()
                .map(|sample| sample.state.velocity_mps.y)
                .collect(),
        ),
        attitude_rad: value(
            selected
                .iter()
                .map(|sample| sample.state.attitude_rad)
                .collect(),
        ),
        angular_rate_radps: value(
            selected
                .iter()
                .map(|sample| sample.state.angular_rate_radps)
                .collect(),
        ),
        mass_kg: value(selected.iter().map(|sample| sample.state.mass_kg).collect()),
        fuel_kg: value(selected.iter().map(|sample| sample.state.fuel_kg).collect()),
        observed_hull_clearance_m: value(
            selected
                .iter()
                .map(|sample| sample.clearance.observed_hull_clearance_m)
                .collect(),
        ),
        observed_touchdown_clearance_m: value(
            selected
                .iter()
                .filter_map(|sample| sample.clearance.observed_touchdown_clearance_m)
                .collect(),
        ),
        contact_clearance_m: value(
            selected
                .iter()
                .map(|sample| sample.clearance.contact_envelope.minimum_clearance_m)
                .collect(),
        ),
        resolved_clearance_m: value(
            selected
                .iter()
                .map(|sample| sample.clearance.resolved_envelope.minimum_clearance_m)
                .collect(),
        ),
        full_clearance_m: value(
            selected
                .iter()
                .map(|sample| sample.clearance.full_envelope.minimum_clearance_m)
                .collect(),
        ),
    })
}

fn path_events(
    samples: &[RouteExecutionSample],
) -> (Vec<RouteExecutionPathEvent>, Vec<RouteExecutionPathEvent>) {
    let mut backtracking = Vec::new();
    let mut reentry = Vec::new();
    let Some(first_sample) = samples.first() else {
        return (backtracking, reentry);
    };
    let mut high_water_m = first_sample.path.along_route_m;
    let mut below_high_water = false;
    for pair in samples.windows(2) {
        let before = &pair[0];
        let after = &pair[1];
        if after.path.along_route_m < before.path.along_route_m {
            high_water_m = high_water_m.max(before.path.along_route_m);
            below_high_water = true;
            backtracking.push(RouteExecutionPathEvent {
                kind: RouteExecutionPathEventKind::Backtracking,
                before_sample_index: before.sample_index,
                after_sample_index: after.sample_index,
                before_progress_m: before.path.along_route_m,
                after_progress_m: after.path.along_route_m,
            });
        } else if below_high_water
            && before.path.along_route_m < high_water_m
            && after.path.along_route_m >= high_water_m
        {
            reentry.push(RouteExecutionPathEvent {
                kind: RouteExecutionPathEventKind::Reentry,
                before_sample_index: before.sample_index,
                after_sample_index: after.sample_index,
                before_progress_m: before.path.along_route_m,
                after_progress_m: after.path.along_route_m,
            });
            below_high_water = false;
            high_water_m = after.path.along_route_m;
        } else if !below_high_water {
            high_water_m = high_water_m.max(after.path.along_route_m);
        }
    }
    (backtracking, reentry)
}

fn classify_route_status(
    input: &RouteExecutionKernelInput<'_>,
    kernel: &RouteExecutionKernelOutput,
    terminal: Option<&SourceTransitionTerminal>,
) -> (
    RouteExecutionEvidenceStatus,
    Option<RouteExecutionInvalidReason>,
) {
    if kernel.waypoints.len() == input.waypoints.len()
        && kernel
            .waypoints
            .iter()
            .all(|waypoint| waypoint.resolution.is_some())
    {
        return (RouteExecutionEvidenceStatus::Complete, None);
    }
    let unresolved_index = kernel
        .waypoints
        .iter()
        .position(|waypoint| waypoint.resolution.is_none())
        .unwrap_or(kernel.waypoints.len());
    let Some(terminal) = terminal else {
        return (
            RouteExecutionEvidenceStatus::Invalid,
            Some(RouteExecutionInvalidReason::MissingTerminalCensor {
                waypoint_index: unresolved_index,
            }),
        );
    };
    if !valid_terminal_reason(&terminal.reason) {
        return (
            RouteExecutionEvidenceStatus::Invalid,
            Some(RouteExecutionInvalidReason::InvalidTerminalReason),
        );
    }
    let Some(last_sample) = input.samples.last() else {
        return (
            RouteExecutionEvidenceStatus::Invalid,
            Some(RouteExecutionInvalidReason::MissingSamples),
        );
    };
    if terminal.physics_step != last_sample.physics_step {
        return (
            RouteExecutionEvidenceStatus::Invalid,
            Some(RouteExecutionInvalidReason::TerminalStepMismatch {
                terminal_step: terminal.physics_step,
                last_sample_step: last_sample.physics_step,
            }),
        );
    }
    (
        RouteExecutionEvidenceStatus::CensoredBeforeWaypoint {
            waypoint_index: unresolved_index,
        },
        None,
    )
}

fn valid_terminal_reason(reason: &str) -> bool {
    matches!(
        reason,
        "checkpoint_satisfied"
            | "checkpoint_failed"
            | "touchdown_on_target"
            | "touchdown_off_target"
            | "crash"
            | "max_time_reached"
    )
}

fn source_invalid_route_reason(
    source_evidence: &SourceTransitionEvidence,
) -> Option<RouteExecutionInvalidReason> {
    (source_evidence.status == SourceTransitionEvidenceStatus::Invalid).then(|| {
        RouteExecutionInvalidReason::SourceEvidenceInvalid {
            reason: source_evidence
                .invalid_reason
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "invalid".to_owned()),
        }
    })
}

fn route_execution_audit(
    samples: &[SampleRecord],
    controller_updates: &[ControllerUpdateRecord],
) -> RouteExecutionAudit {
    let sample_ordering = samples
        .iter()
        .enumerate()
        .map(|(sample_index, sample)| SourceTransitionSampleIdentity {
            sample_index,
            physics_step: sample.physics_step,
            sim_time_s: sample.sim_time_s,
        })
        .collect();
    let controller_handoff_markers = controller_updates
        .iter()
        .filter_map(|update| {
            let markers = update
                .frame
                .markers
                .iter()
                .filter(|marker| {
                    marker.id == "waypoint/handoff"
                        || marker
                            .metadata
                            .get("kind")
                            .is_some_and(|value| matches!(value, TelemetryValue::Text(kind) if kind == "waypoint_handoff"))
                })
                .map(|marker| RouteExecutionControllerMarkerAudit {
                    update_index: update.controller_update_index,
                    physics_step: update.physics_step,
                    sim_time_s: update.sim_time_s,
                    phase: update.frame.phase.clone(),
                    marker: marker.clone(),
                })
                .collect::<Vec<_>>();
            (!markers.is_empty()).then_some(markers)
        })
        .flatten()
        .collect();
    RouteExecutionAudit {
        sample_ordering,
        controller_handoff_markers,
    }
}

fn invalid_route_evidence(
    cadence: SourceTransitionCadence,
    samples: &[SampleRecord],
    source_evidence: SourceTransitionEvidence,
    mut provenance: RouteExecutionProvenance,
    terminal: Option<SourceTransitionTerminal>,
    controller_updates: &[ControllerUpdateRecord],
    reason: RouteExecutionInvalidReason,
) -> RouteExecutionEvidence {
    provenance.source_evidence_digest = source_evidence.evidence_digest.clone();
    let evidence = RouteExecutionEvidence {
        schema_version: ROUTE_EXECUTION_SCHEMA_VERSION,
        extractor_version: ROUTE_EXECUTION_EXTRACTOR_VERSION.to_owned(),
        evidence_digest: String::new(),
        physical_digest: String::new(),
        status: RouteExecutionEvidenceStatus::Invalid,
        invalid_reason: Some(reason),
        provenance,
        cadence,
        source_transition: source_evidence,
        selected_centerline_m: Vec::new(),
        samples: Vec::new(),
        legs: Vec::new(),
        waypoints: Vec::new(),
        backtracking_events: Vec::new(),
        reentry_events: Vec::new(),
        terminal,
        audit: route_execution_audit(samples, controller_updates),
    };
    seal_route_execution_evidence(evidence)
}

fn seal_route_execution_evidence(mut evidence: RouteExecutionEvidence) -> RouteExecutionEvidence {
    evidence.evidence_digest.clear();
    evidence.physical_digest.clear();
    // Seal the persisted representation.  Exact float round-tripping should
    // converge immediately; retain a bounded defensive fixed-point check.
    for _ in 0..8 {
        let Ok(bytes) = serde_json::to_vec(&evidence) else {
            break;
        };
        let Ok(next) = serde_json::from_slice::<RouteExecutionEvidence>(&bytes) else {
            break;
        };
        if next == evidence {
            break;
        }
        evidence = next;
    }
    evidence.physical_digest = if evidence.status == RouteExecutionEvidenceStatus::Invalid {
        digest_serialized(&(
            evidence.invalid_reason.clone(),
            evidence.provenance.samples_digest.clone(),
        ))
    } else {
        route_execution_physical_digest(&evidence)
    };
    evidence.evidence_digest = route_execution_evidence_digest(&evidence);
    evidence
}

struct RouteExecutionInvalidContext<'a> {
    scenario: &'a ScenarioSpec,
    route_plan: &'a RoutePlan,
    source_evidence: SourceTransitionEvidence,
    source_provenance: SourceTransitionProvenance,
    terminal: Option<SourceTransitionTerminal>,
    samples: &'a [SampleRecord],
    controller_updates: &'a [ControllerUpdateRecord],
}

fn invalid_route_evidence_from_context(
    context: RouteExecutionInvalidContext<'_>,
    reason: RouteExecutionInvalidReason,
) -> RouteExecutionEvidence {
    let cadence = SourceTransitionCadence {
        sample_hz: context.scenario.sim.sample_hz,
        physics_hz: context.scenario.sim.physics_hz,
    };
    let provenance = RouteExecutionProvenance {
        resolved_input_digest: context.source_provenance.resolved_input_digest,
        request_digest: context.source_provenance.request_digest,
        policy_digest: context.source_provenance.policy_digest,
        route_plan_digest: context.route_plan.plan_digest.clone(),
        source_target_geometry_digest: context.source_provenance.source_target_geometry_digest,
        raw_bundle_digest: context.source_provenance.raw_bundle_digest,
        samples_digest: context.source_provenance.samples_digest,
        action_log_digest: context.source_provenance.action_log_digest,
        source_evidence_digest: context.source_evidence.evidence_digest.clone(),
        artifact_identity: context.source_provenance.artifact_identity,
    };
    invalid_route_evidence(
        cadence,
        context.samples,
        context.source_evidence,
        provenance,
        context.terminal,
        context.controller_updates,
        reason,
    )
}

/// Stable digest excluding the evidence's own digest field.
pub fn route_execution_evidence_digest(evidence: &RouteExecutionEvidence) -> String {
    let mut material = evidence.clone();
    material.evidence_digest.clear();
    digest_serialized(&material)
}

/// Recompute the complete neutral D0b physical digest.  Provenance, terminal
/// metadata, and controller audit remain deliberately outside this identity.
pub fn route_execution_physical_digest(evidence: &RouteExecutionEvidence) -> String {
    let kernel_digest = digest_serialized(&(
        &evidence.selected_centerline_m,
        &evidence.samples,
        &evidence.legs,
        &evidence.waypoints,
        &evidence.backtracking_events,
        &evidence.reentry_events,
    ));
    digest_serialized(&(
        evidence.source_transition.physical_digest.clone(),
        kernel_digest,
    ))
}

/// Validate the schema and index topology of a persisted neutral route
/// artifact.  This rejects malformed artifacts even when an attacker or
/// broken producer has recomputed otherwise self-consistent digests.
pub fn validate_persisted_route_execution_evidence(
    evidence: &RouteExecutionEvidence,
) -> Result<(), String> {
    if evidence.schema_version != ROUTE_EXECUTION_SCHEMA_VERSION {
        return Err(format!(
            "route evidence schema_version must equal {ROUTE_EXECUTION_SCHEMA_VERSION}"
        ));
    }
    if evidence.extractor_version != ROUTE_EXECUTION_EXTRACTOR_VERSION {
        return Err(format!(
            "route evidence extractor_version must equal {ROUTE_EXECUTION_EXTRACTOR_VERSION}"
        ));
    }
    if evidence.evidence_digest != route_execution_evidence_digest(evidence) {
        return Err("route evidence digest mismatch".to_owned());
    }
    if evidence.status == RouteExecutionEvidenceStatus::Invalid {
        return evidence
            .invalid_reason
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "invalid route evidence has no invalid_reason".to_owned());
    }
    if evidence.invalid_reason.is_some() {
        return Err("non-invalid route evidence carries an invalid_reason".to_owned());
    }
    super::source_transition::validate_persisted_source_transition_evidence(
        &evidence.source_transition,
    )?;
    if evidence.cadence.physics_hz == 0
        || evidence.cadence.sample_hz != Some(evidence.cadence.physics_hz)
        || evidence.cadence != evidence.source_transition.cadence
    {
        return Err("route evidence is not sampled at the source physics rate".to_owned());
    }
    if evidence.provenance.source_evidence_digest != evidence.source_transition.evidence_digest {
        return Err("route provenance does not name the nested source evidence".to_owned());
    }
    if evidence.physical_digest != route_execution_physical_digest(evidence) {
        return Err("route physical digest mismatch".to_owned());
    }
    if evidence.selected_centerline_m.len() < 2
        || evidence
            .selected_centerline_m
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        || evidence
            .selected_centerline_m
            .windows(2)
            .any(|pair| pair[1].x <= pair[0].x || (pair[1] - pair[0]).length() <= 0.0)
    {
        return Err("route evidence selected centerline is malformed".to_owned());
    }
    if evidence.samples.is_empty() {
        return Err("route evidence has no samples".to_owned());
    }
    for (index, sample) in evidence.samples.iter().enumerate() {
        if sample.sample_index != index
            || sample.state.sample_index != index
            || sample.physics_step != sample.state.physics_step
            || sample.sim_time_s != sample.state.sim_time_s
            || (index == 0 && sample.physics_step != 0)
            || (index > 0
                && evidence.samples[index - 1].physics_step.checked_add(1)
                    != Some(sample.physics_step))
            || !route_execution_sample_finite(sample)
            || !route_execution_raw_state_finite(&sample.state)
        {
            return Err(format!(
                "route sample {index} is malformed or non-contiguous"
            ));
        }
    }
    for (leg_index, leg) in evidence.legs.iter().enumerate() {
        if leg.leg_index != leg_index {
            return Err(format!("route leg {leg_index} has a non-canonical index"));
        }
        validate_route_boundary_identity(evidence, &leg.start_boundary)?;
        validate_route_boundary_identity(evidence, &leg.end_boundary)?;
        if let (Some(start), Some(end)) = (
            leg.start_boundary.sample_index,
            leg.end_boundary.sample_index,
        ) && start > end
        {
            return Err(format!("route leg {leg_index} has reversed boundaries"));
        }
        for range in &leg.sample_ranges {
            let Some(first) = range.sample_indices.first().copied() else {
                return Err(format!("route leg {leg_index} has an empty sample range"));
            };
            if first != range.first_sample_index
                || range.sample_indices.last().copied() != Some(range.last_sample_index)
                || range
                    .sample_indices
                    .iter()
                    .any(|index| *index >= evidence.samples.len())
                || range
                    .sample_indices
                    .windows(2)
                    .any(|pair| pair[0].checked_add(1) != Some(pair[1]))
            {
                return Err(format!(
                    "route leg {leg_index} has a malformed sample range"
                ));
            }
        }
    }
    for (waypoint_index, waypoint) in evidence.waypoints.iter().enumerate() {
        if waypoint.waypoint_index != waypoint_index
            || waypoint.waypoint_id.trim().is_empty()
            || waypoint.contract.id != waypoint.waypoint_id
        {
            return Err(format!(
                "route waypoint {waypoint_index} has an invalid identity"
            ));
        }
        waypoint
            .contract
            .validate()
            .map_err(|error| format!("route waypoint {waypoint_index}: {error}"))?;
        for opportunity in &waypoint.opportunity {
            let sample = evidence
                .samples
                .get(opportunity.sample.sample_index)
                .ok_or_else(|| {
                    format!("route waypoint {waypoint_index} opportunity sample is missing")
                })?;
            if sample != &opportunity.sample {
                return Err(format!(
                    "route waypoint {waypoint_index} opportunity sample is not canonical"
                ));
            }
        }
        if let Some(resolution) = &waypoint.resolution {
            let sample = evidence
                .samples
                .get(resolution.sample_index)
                .ok_or_else(|| format!("route waypoint {waypoint_index} resolution is missing"))?;
            if resolution.physics_step != sample.physics_step
                || resolution.sim_time_s != sample.sim_time_s
            {
                return Err(format!(
                    "route waypoint {waypoint_index} resolution identity is inconsistent"
                ));
            }
        }
        if let Some(first_pass) = &waypoint.first_contract_pass
            && (!first_pass.assessment.contract_pass
                || evidence.samples.get(first_pass.sample.sample_index) != Some(&first_pass.sample))
        {
            return Err(format!(
                "route waypoint {waypoint_index} first contract pass is malformed"
            ));
        }
    }
    match evidence.status {
        RouteExecutionEvidenceStatus::Complete => {
            if evidence
                .waypoints
                .iter()
                .any(|waypoint| waypoint.resolution.is_none())
            {
                return Err("complete route evidence has an unresolved waypoint".to_owned());
            }
        }
        RouteExecutionEvidenceStatus::CensoredBeforeWaypoint { waypoint_index } => {
            if waypoint_index >= evidence.waypoints.len()
                || evidence.waypoints[waypoint_index].resolution.is_some()
            {
                return Err(
                    "censored route status does not identify an unresolved waypoint".to_owned(),
                );
            }
        }
        RouteExecutionEvidenceStatus::Invalid => unreachable!(),
    }
    Ok(())
}

fn validate_route_boundary_identity(
    evidence: &RouteExecutionEvidence,
    boundary: &RouteExecutionBoundaryIdentity,
) -> Result<(), String> {
    match (
        boundary.sample_index,
        boundary.physics_step,
        boundary.sim_time_s,
    ) {
        (None, None, None) => Ok(()),
        (Some(index), Some(step), Some(time)) => {
            let sample = evidence
                .samples
                .get(index)
                .ok_or_else(|| format!("route boundary '{}' sample is missing", boundary.kind))?;
            if sample.physics_step != step || sample.sim_time_s != time {
                return Err(format!(
                    "route boundary '{}' identity is inconsistent",
                    boundary.kind
                ));
            }
            Ok(())
        }
        _ => Err(format!(
            "route boundary '{}' has partial sample identity",
            boundary.kind
        )),
    }
}

fn route_execution_raw_state_finite(state: &SourceTransitionRawState) -> bool {
    [
        state.sim_time_s,
        state.position_m.x,
        state.position_m.y,
        state.velocity_mps.x,
        state.velocity_mps.y,
        state.attitude_rad,
        state.angular_rate_radps,
        state.mass_kg,
        state.fuel_kg,
    ]
    .iter()
    .all(|value| value.is_finite())
}

/// Write route evidence and its audit/provenance sidecar.
pub fn write_route_execution_artifacts(
    path: &Path,
    evidence: &RouteExecutionEvidence,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create route evidence directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(path, serde_json::to_string_pretty(evidence)?)
        .with_context(|| format!("failed to write route evidence {}", path.display()))?;
    let sidecar = RouteExecutionAuditSidecar {
        schema_version: evidence.schema_version,
        extractor_version: evidence.extractor_version.clone(),
        evidence_digest: evidence.evidence_digest.clone(),
        status: evidence.status.clone(),
        provenance: evidence.provenance.clone(),
        cadence: evidence.cadence,
        source_evidence_digest: evidence.source_transition.evidence_digest.clone(),
        terminal: evidence.terminal.clone(),
        audit: evidence.audit.clone(),
    };
    let sidecar_path = path.with_extension("audit.json");
    fs::write(&sidecar_path, serde_json::to_string_pretty(&sidecar)?)
        .with_context(|| format!("failed to write route audit {}", sidecar_path.display()))?;
    Ok(())
}

/// Build route provenance from the same raw bundle identity as D0a.
pub fn route_execution_provenance_for_route_plan(
    scenario: &ScenarioSpec,
    route_plan: &RoutePlan,
    artifacts: &pd_core::RunArtifacts,
    controller_updates: &[ControllerUpdateRecord],
    artifact_identity: impl Into<String>,
) -> Result<RouteExecutionProvenance> {
    let source = source_transition_provenance_for_route_plan(
        scenario,
        route_plan,
        artifacts,
        controller_updates,
        artifact_identity,
    )?;
    Ok(RouteExecutionProvenance {
        resolved_input_digest: source.resolved_input_digest,
        request_digest: source.request_digest,
        policy_digest: source.policy_digest,
        route_plan_digest: source.route_plan_digest,
        source_target_geometry_digest: source.source_target_geometry_digest,
        raw_bundle_digest: source.raw_bundle_digest,
        samples_digest: source.samples_digest,
        action_log_digest: source.action_log_digest,
        source_evidence_digest: String::new(),
        artifact_identity: source.artifact_identity,
    })
}

fn fraction(before: f64, after: f64, boundary: f64) -> f64 {
    if (after - before).abs() <= f64::EPSILON {
        0.0
    } else {
        ((boundary - before) / (after - before)).clamp(0.0, 1.0)
    }
}

fn dot(lhs: Vec2, rhs: Vec2) -> f64 {
    lhs.x * rhs.x + lhs.y * rhs.y
}

fn centerline_segment_index(coordinate: f64, centerline: &[Vec2]) -> Option<usize> {
    if !coordinate.is_finite() || centerline.len() < 2 {
        return None;
    }
    for (index, pair) in centerline.windows(2).enumerate() {
        let is_last = index + 2 == centerline.len();
        if coordinate >= pair[0].x - ROUTE_EXECUTION_TOLERANCE_M
            && (coordinate < pair[1].x - ROUTE_EXECUTION_TOLERANCE_M || is_last)
        {
            return Some(index);
        }
    }
    None
}

fn route_execution_sample_finite(sample: &RouteExecutionSample) -> bool {
    let values = [
        sample.sim_time_s,
        sample.source_progress_m,
        sample.path.normalized_position_m.x,
        sample.path.normalized_position_m.y,
        sample.path.normalized_route_progress,
        sample.path.along_route_m,
        sample.path.along_track_m,
        sample.path.cross_track_m,
        sample.path.velocity_along_route_mps,
        sample.path.velocity_cross_route_mps,
        sample.clearance.observed_hull_clearance_m,
        sample
            .clearance
            .observed_touchdown_clearance_m
            .unwrap_or(0.0),
        sample.clearance.contact_envelope.minimum_clearance_m,
        sample.clearance.resolved_envelope.minimum_clearance_m,
        sample.clearance.full_envelope.minimum_clearance_m,
    ];
    values.iter().all(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{Command, CorridorEnvelope, Observation, TerrainDefinition, VehicleGeometry};
    use std::collections::BTreeMap;
    use std::sync::OnceLock;

    fn test_vehicle() -> &'static VehicleSpec {
        static VEHICLE: OnceLock<VehicleSpec> = OnceLock::new();
        VEHICLE.get_or_init(|| VehicleSpec {
            geometry: VehicleGeometry {
                hull_width_m: 2.0,
                hull_height_m: 3.0,
                touchdown_half_span_m: 1.0,
                touchdown_base_offset_m: 1.5,
            },
            dry_mass_kg: 5.0,
            initial_fuel_kg: 5.0,
            max_fuel_kg: 5.0,
            max_thrust_n: 100.0,
            max_fuel_burn_kgps: 1.0,
            min_throttle_frac: 0.0,
            max_rotation_rate_radps: 1.0,
            safe_touchdown_normal_speed_mps: 2.0,
            safe_touchdown_tangential_speed_mps: 2.0,
            safe_touchdown_attitude_error_rad: 0.2,
            safe_touchdown_angular_rate_radps: 0.2,
        })
    }

    fn profile() -> SafetyProfile {
        SafetyProfile {
            source_transition_start_m: 3.0,
            source_transition_end_m: 4.0,
            target_transition_start_m: 8.0,
            target_transition_end_m: 10.0,
            horizontal_span_m: 10.0,
            full_envelope: CorridorEnvelope::new(1.0, 1.0),
            contact_envelope: CorridorEnvelope::new(0.5, 0.5),
        }
    }

    fn terrain() -> TerrainDefinition {
        TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-20.0, 0.0), Vec2::new(20.0, 0.0)],
        }
    }

    fn waypoint(id: &str, x_m: f64, min_progress: f64) -> TransferWaypointSpec {
        TransferWaypointSpec {
            id: id.to_owned(),
            position_m: Vec2::new(x_m, 20.0),
            handoff_tangent_unit: Some(Vec2::new(1.0, 0.0)),
            capture_radius_m: 1.0,
            max_cross_track_m: 1.0,
            max_outbound_heading_error_rad: 0.5,
            min_outbound_progress_mps: min_progress,
            max_outbound_cross_speed_mps: Some(2.0),
            min_speed_mps: 0.0,
            max_speed_mps: 20.0,
            min_vertical_speed_mps: None,
            max_vertical_speed_mps: None,
        }
    }

    fn sample(step: u64, x_m: f64) -> SampleRecord {
        let sim_time_s = step as f64 / 10.0;
        SampleRecord {
            sim_time_s,
            physics_step: step,
            observation: Observation {
                sim_time_s,
                physics_step: step,
                position_m: Vec2::new(x_m, 20.0),
                velocity_mps: Vec2::new(5.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
                mass_kg: 10.0,
                fuel_kg: 5.0,
                gravity_mps2: 1.62,
                target_dx_m: 0.0,
                height_above_target_m: 20.0,
                target_surface_y_m: 0.0,
                target_pad_half_width_m: 2.0,
                touchdown_clearance_m: 19.0,
                min_hull_clearance_m: 18.0,
            },
            held_command: Command::idle(),
        }
    }

    fn sample_with_velocity(step: u64, x_m: f64, velocity_mps: Vec2) -> SampleRecord {
        let mut sample = sample(step, x_m);
        sample.observation.velocity_mps = velocity_mps;
        sample
    }

    fn sample_with_position(step: u64, x_m: f64, y_m: f64) -> SampleRecord {
        let mut sample = sample(step, x_m);
        sample.observation.position_m = Vec2::new(x_m, y_m);
        sample
    }

    fn input<'a>(
        initial: &'a VehicleInitialState,
        geometry: &'a NormalizedRouteGeometry,
        profile: &'a SafetyProfile,
        terrain: &'a TerrainDefinition,
        centerline: &'a [Vec2],
        waypoints: &'a [TransferWaypointSpec],
        samples: &'a [SampleRecord],
    ) -> RouteExecutionKernelInput<'a> {
        RouteExecutionKernelInput {
            terrain,
            source_pad_center_x_m: 0.0,
            source_pad_surface_y_m: 20.0,
            target_pad_center_x_m: 10.0,
            target_pad_surface_y_m: 20.0,
            vehicle: test_vehicle(),
            initial_state: initial,
            geometry,
            profile,
            selected_centerline_m: centerline,
            waypoints,
            samples,
            cadence: SourceTransitionCadence::physics_rate(10),
        }
    }

    fn source_input<'a>(input: &RouteExecutionKernelInput<'a>) -> SourceTransitionKernelInput<'a> {
        SourceTransitionKernelInput {
            terrain: input.terrain,
            source_pad_center_x_m: input.source_pad_center_x_m,
            vehicle: input.vehicle,
            initial_state: input.initial_state,
            geometry: input.geometry,
            profile: input.profile,
            selected_centerline_m: input.selected_centerline_m,
            waypoints: input.waypoints,
            samples: input.samples,
            cadence: input.cadence,
            audit: SourceTransitionAuditInput::default(),
        }
    }

    fn terminal(step: u64, reason: &str) -> SourceTransitionTerminal {
        SourceTransitionTerminal {
            physics_step: step,
            reason: reason.to_owned(),
        }
    }

    #[test]
    fn maps_exact_vertex_and_retains_before_after_and_reentry() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
            sample(4, 5.5),
            sample(5, 4.5),
            sample(6, 5.5),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        assert_eq!(output.samples.len(), samples.len());
        assert_eq!(output.samples[2].path.segment_index, Some(2));
        assert_eq!(output.samples[2].path.route_leg_index, Some(0));
        assert_eq!(
            output.samples[0].path.region,
            RouteExecutionSampleRegion::Centerline
        );
        assert_eq!(
            output.waypoints[0]
                .capture_entry
                .as_ref()
                .unwrap()
                .after
                .sample_index,
            3
        );
        assert!(output.waypoints[0].deadline.is_none());
        assert!(
            output
                .backtracking_events
                .iter()
                .any(|event| event.before_sample_index == 4)
        );
        assert!(
            output
                .reentry_events
                .iter()
                .any(|event| event.before_sample_index == 5)
        );
        assert_eq!(output.reentry_events.len(), 1);
    }

    #[test]
    fn records_first_capture_and_deadline_brackets_with_sticky_window() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
            sample(4, 6.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        let waypoint = &output.waypoints[0];
        assert_eq!(
            waypoint.capture_entry.as_ref().unwrap().before.sample_index,
            2
        );
        assert_eq!(
            waypoint.capture_entry.as_ref().unwrap().after.sample_index,
            3
        );
        assert_eq!(waypoint.deadline.as_ref().unwrap().before.sample_index, 3);
        assert_eq!(waypoint.deadline.as_ref().unwrap().after.sample_index, 4);
        assert_eq!(waypoint.opportunity.len(), 2);
        assert!(waypoint.opportunity.iter().all(|sample| sample.window_seen));
        assert_eq!(
            waypoint.resolution.as_ref().unwrap().kind,
            RouteExecutionResolutionKind::Deadline
        );
    }

    #[test]
    fn initial_deadline_is_explicit_and_has_no_synthetic_bracket() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0)];
        let samples = vec![sample(0, 0.0), sample(1, 2.0), sample(2, 6.0)];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        let waypoint = &output.waypoints[0];
        assert!(waypoint.initially_at_deadline);
        assert!(waypoint.deadline.is_none());
        assert_eq!(waypoint.opportunity.len(), 1);
        assert_eq!(
            waypoint.resolution.as_ref().unwrap().kind,
            RouteExecutionResolutionKind::InitialDeadline
        );
    }

    #[test]
    fn initial_past_deadline_resolves_before_later_reverse_capture_entry() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0), waypoint("wp1", 8.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 7.5),
            sample(3, 5.5),
            sample(4, 7.5),
            sample(5, 8.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        let first = &output.waypoints[0];
        assert!(!first.initially_inside);
        assert!(first.initially_at_deadline);
        assert_eq!(first.capture_entry.as_ref().unwrap().after.sample_index, 3);
        assert!(first.deadline.is_none());
        assert_eq!(first.opportunity.len(), 1);
        assert!(first.first_contract_pass.is_none());
        assert_eq!(
            first.resolution.as_ref().unwrap().kind,
            RouteExecutionResolutionKind::InitialDeadline
        );
        assert_eq!(first.resolution.as_ref().unwrap().sample_index, 2);
        assert_eq!(
            output.waypoints[1]
                .resolution
                .as_ref()
                .unwrap()
                .sample_index,
            5
        );
        assert!(first.crossings.iter().any(|crossing| {
            crossing.before_sample_index >= 2
                && crossing.kind == RouteExecutionCrossingKind::CaptureWindow
                && crossing.direction == SourceTransitionCrossingDirection::Reverse
        }));
        assert!(first.crossings.iter().any(|crossing| {
            crossing.before_sample_index >= 2
                && crossing.kind == RouteExecutionCrossingKind::DeadlinePlane
                && crossing.direction == SourceTransitionCrossingDirection::Reverse
        }));
    }

    #[test]
    fn sequential_waypoints_resolve_from_prior_raw_index() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0), waypoint("wp1", 8.0, 1.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
            sample(4, 6.0),
            sample(5, 7.0),
            sample(6, 8.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        assert_eq!(output.waypoints.len(), 2);
        assert_eq!(
            output.waypoints[0]
                .resolution
                .as_ref()
                .unwrap()
                .sample_index,
            3
        );
        assert_eq!(
            output.waypoints[0]
                .first_contract_pass
                .as_ref()
                .unwrap()
                .sample
                .sample_index,
            3
        );
        assert_eq!(
            output.waypoints[1]
                .resolution
                .as_ref()
                .unwrap()
                .sample_index,
            5
        );
        assert_eq!(output.retained_end_index, 5);
        assert_eq!(output.samples[4].path.segment_index, Some(3));
        assert_eq!(output.samples[4].path.route_leg_index, Some(1));
        assert_eq!(output.legs[2].start_boundary.kind, "waypoint_resolution");
        assert_eq!(output.legs[2].start_boundary.sample_index, Some(5));
        assert_eq!(output.legs[2].end_boundary.kind, "target_scope_unreached");
        assert_eq!(output.legs[2].end_boundary.sample_index, None);
    }

    #[test]
    fn crossings_continue_after_first_pass_until_next_waypoint_resolution() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0), waypoint("wp1", 8.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
            sample(4, 7.5),
            sample(5, 5.5),
            sample(6, 7.5),
            sample(7, 5.5),
            sample(8, 8.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        assert_eq!(
            output.waypoints[0]
                .resolution
                .as_ref()
                .unwrap()
                .sample_index,
            3
        );
        assert_eq!(
            output.waypoints[1]
                .resolution
                .as_ref()
                .unwrap()
                .sample_index,
            8
        );
        let crossings = &output.waypoints[0].crossings;
        assert!(crossings.iter().any(|crossing| {
            crossing.before_sample_index >= 3
                && crossing.kind == RouteExecutionCrossingKind::DeadlinePlane
                && crossing.direction == SourceTransitionCrossingDirection::Reverse
        }));
        assert!(crossings.iter().any(|crossing| {
            crossing.before_sample_index >= 3
                && crossing.kind == RouteExecutionCrossingKind::CaptureWindow
                && crossing.direction == SourceTransitionCrossingDirection::Reverse
        }));
        assert!(
            crossings
                .iter()
                .any(|crossing| crossing.after_sample_index == 8)
        );
        assert!(
            crossings
                .iter()
                .all(|crossing| crossing.after_sample_index <= 8)
        );
    }

    #[test]
    fn initially_inside_starts_at_authoritative_sample_without_capture_bracket() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 5.0),
            sample(3, 6.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        let waypoint = &output.waypoints[0];
        assert!(waypoint.initially_inside);
        assert!(waypoint.capture_entry.is_none());
        assert_eq!(waypoint.opportunity[0].sample.sample_index, 2);
        assert_eq!(waypoint.resolution.as_ref().unwrap().sample_index, 2);
    }

    #[test]
    fn mirrored_route_maps_world_progress_and_velocity_to_same_normalized_path() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(-5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: -1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let mut waypoints = [waypoint("wp0", -6.0, 1.0)];
        waypoints[0].handoff_tangent_unit = Some(Vec2::new(-1.0, 0.0));
        let samples = vec![
            sample(0, 0.0),
            sample(1, -2.0),
            sample(2, -4.0),
            sample(3, -5.0),
            sample(4, -6.0),
        ];
        let mut samples = samples;
        for sample in &mut samples {
            sample.observation.velocity_mps.x = -5.0;
        }
        let input = RouteExecutionKernelInput {
            target_pad_center_x_m: -10.0,
            ..input(
                &initial,
                &geometry,
                &profile,
                &terrain,
                &centerline,
                &waypoints,
                &samples,
            )
        };
        let output = extract_route_execution_kernel(&input).unwrap();
        assert_eq!(output.samples[2].source_progress_m, 4.0);
        assert!(output.samples[2].path.velocity_along_route_mps > 0.0);
        assert_eq!(
            output.waypoints[0]
                .resolution
                .as_ref()
                .unwrap()
                .sample_index,
            3
        );
    }

    #[test]
    fn before_after_scope_and_final_vertex_use_explicit_mapping() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, -1.0),
            sample(2, 1.0),
            sample(3, 4.0),
            sample(4, 6.0),
            sample(5, 10.0),
            sample(6, 11.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        let route_geometry = resolve_route_geometry(&input, None).unwrap();
        let mapped_after = route_execution_sample(&input, &route_geometry, 6, &samples[6]).unwrap();
        assert_eq!(
            output.samples[1].path.region,
            RouteExecutionSampleRegion::BeforeSource
        );
        assert_eq!(
            mapped_after.path.region,
            RouteExecutionSampleRegion::AfterScope
        );
        let mapped_final_vertex =
            route_execution_sample(&input, &route_geometry, 5, &samples[5]).unwrap();
        assert_eq!(mapped_final_vertex.path.segment_index, Some(4));
        assert_eq!(output.samples[4].path.segment_index, Some(3));
    }

    #[test]
    fn sloped_projection_does_not_reach_target_scope_before_horizontal_span() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 11.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 25.0),
            Vec2::new(10.0, 30.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
            sample_with_position(4, 9.0, 40.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        assert!(output.samples[4].path.normalized_route_progress > 1.0);
        assert_eq!(
            output.samples[4].path.region,
            RouteExecutionSampleRegion::Centerline
        );
        let before_target =
            target_scope_boundary(&output.samples, geometry.direct_horizontal_span_m);
        assert_eq!(before_target.kind, "target_scope_unreached");
        assert_eq!(before_target.sample_index, None);

        let route_geometry = resolve_route_geometry(&input, None).unwrap();
        let endpoint = route_execution_sample(
            &input,
            &route_geometry,
            5,
            &sample_with_position(5, 10.0, 30.0),
        )
        .unwrap();
        let after_scope = route_execution_sample(
            &input,
            &route_geometry,
            6,
            &sample_with_position(6, 11.0, 30.0),
        )
        .unwrap();
        let mut reached_samples = output.samples.clone();
        reached_samples.extend([endpoint, after_scope]);
        let reached_target =
            target_scope_boundary(&reached_samples, geometry.direct_horizontal_span_m);
        assert_eq!(reached_target.sample_index, Some(5));
        assert_eq!(reached_target.physics_step, Some(5));
    }

    #[test]
    fn leg_ranges_remain_split_when_samples_backtrack_between_segments() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0), waypoint("wp1", 8.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
            sample(4, 6.0),
            sample(5, 5.0),
            sample(6, 6.5),
            sample(7, 7.0),
            sample(8, 8.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        assert!(output.legs[0].sample_ranges.len() >= 2);
        assert!(output.legs.iter().all(|leg| leg.extrema.is_some()));
        assert_eq!(output.legs[0].start_boundary.sample_index, Some(2));
        assert_eq!(output.legs[0].end_boundary.sample_index, Some(4));
        assert_eq!(output.samples[4].path.normalized_route_progress, 0.6);
    }

    #[test]
    fn sound_source_censor_before_tracking_is_valid_route_censor() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0)];
        let samples = vec![sample(0, 0.0), sample(1, 2.0)];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let source = super::super::source_transition::assemble_source_transition_evidence(
            &source_input(&input),
            SourceTransitionProvenance::default(),
            Some(terminal(1, "max_time_reached")),
        );
        assert_eq!(
            source.status,
            SourceTransitionEvidenceStatus::CensoredBeforeContactExit
        );
        let route = assemble_route_execution_evidence(
            &input,
            source,
            RouteExecutionProvenance::default(),
            Some(terminal(1, "max_time_reached")),
            &[],
        );
        assert_eq!(
            route.status,
            RouteExecutionEvidenceStatus::CensoredBeforeWaypoint { waypoint_index: 0 }
        );
        assert!(matches!(
            &route.source_transition.status,
            SourceTransitionEvidenceStatus::CensoredBeforeContactExit
                | SourceTransitionEvidenceStatus::CensoredBeforeTrackingEntry
        ));
        assert_eq!(route.samples.len(), samples.len());
        assert_eq!(
            route.legs[0].start_boundary.kind,
            "tracking_entry_unreached"
        );
        assert_eq!(route.legs[1].end_boundary.kind, "target_scope_unreached");
        assert_eq!(route.legs[1].end_boundary.sample_index, None);
        assert!(route.invalid_reason.is_none());
    }

    #[test]
    fn sound_terminal_prefix_censors_at_unresolved_second_waypoint() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0), waypoint("wp1", 8.0, 100.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
            sample(4, 6.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let source = super::super::source_transition::assemble_source_transition_evidence(
            &source_input(&input),
            SourceTransitionProvenance::default(),
            Some(terminal(4, "max_time_reached")),
        );
        let route = assemble_route_execution_evidence(
            &input,
            source,
            RouteExecutionProvenance::default(),
            Some(terminal(4, "max_time_reached")),
            &[],
        );
        assert_eq!(
            route.status,
            RouteExecutionEvidenceStatus::CensoredBeforeWaypoint { waypoint_index: 1 }
        );
        assert_eq!(
            route.waypoints[0].resolution.as_ref().unwrap().sample_index,
            3
        );
        assert!(route.waypoints[1].resolution.is_none());
        assert_eq!(route.samples.len(), samples.len());
        assert_eq!(
            route.legs[2].start_boundary.kind,
            "waypoint_resolution_unreached"
        );
        assert_eq!(route.legs[2].start_boundary.sample_index, None);
        assert_eq!(route.legs[2].end_boundary.kind, "target_scope_unreached");
        assert_eq!(route.legs[2].end_boundary.sample_index, None);
    }

    #[test]
    fn malformed_terminal_reason_is_route_invalid() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0)];
        let samples = vec![sample(0, 0.0), sample(1, 2.0), sample(2, 4.0)];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let source = super::super::source_transition::assemble_source_transition_evidence(
            &source_input(&input),
            SourceTransitionProvenance::default(),
            Some(terminal(2, "max_time_reached")),
        );
        let route = assemble_route_execution_evidence(
            &input,
            source,
            RouteExecutionProvenance::default(),
            Some(terminal(2, "not_a_terminal_reason")),
            &[],
        );
        assert_eq!(route.status, RouteExecutionEvidenceStatus::Invalid);
        assert_eq!(
            route.invalid_reason,
            Some(RouteExecutionInvalidReason::InvalidTerminalReason)
        );
    }

    #[test]
    fn invalid_nested_source_and_bad_terminal_are_not_silently_censored() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 100.0)];
        let samples = vec![sample(0, 0.0), sample(1, 2.0), sample(2, 4.0)];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let mut invalid_source =
            super::super::source_transition::assemble_source_transition_evidence(
                &source_input(&input),
                SourceTransitionProvenance::default(),
                None,
            );
        invalid_source.status = SourceTransitionEvidenceStatus::Invalid;
        let route = assemble_route_execution_evidence(
            &input,
            invalid_source,
            RouteExecutionProvenance::default(),
            Some(terminal(2, "max_time_reached")),
            &[],
        );
        assert_eq!(route.status, RouteExecutionEvidenceStatus::Invalid);
        assert!(matches!(
            route.invalid_reason,
            Some(RouteExecutionInvalidReason::SourceEvidenceInvalid { .. })
        ));
    }

    #[test]
    fn route_rejects_direct_scope_wrong_cadence_and_stale_or_invalid_centerline() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0)];
        let samples = vec![sample(0, 0.0), sample(1, 2.0), sample(2, 4.0)];
        let base = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let no_waypoints: [TransferWaypointSpec; 0] = [];
        let direct = RouteExecutionKernelInput {
            waypoints: &no_waypoints,
            ..base
        };
        assert!(matches!(
            extract_route_execution_kernel(&direct),
            Err(RouteExecutionInvalidReason::DirectRouteOutOfScope)
        ));

        let wrong_cadence = RouteExecutionKernelInput {
            cadence: SourceTransitionCadence {
                sample_hz: Some(5),
                physics_hz: 10,
            },
            ..base
        };
        assert!(matches!(
            extract_route_execution_kernel(&wrong_cadence),
            Err(RouteExecutionInvalidReason::WrongCadence {
                sample_hz: 5,
                physics_hz: 10,
            })
        ));

        let stale_centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(9.0, 20.0),
        ];
        let stale = RouteExecutionKernelInput {
            selected_centerline_m: &stale_centerline,
            ..base
        };
        assert!(matches!(
            extract_route_execution_kernel(&stale),
            Err(RouteExecutionInvalidReason::InvalidGeometry { .. })
        ));

        let malformed_centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let malformed = RouteExecutionKernelInput {
            selected_centerline_m: &malformed_centerline,
            ..base
        };
        assert!(matches!(
            extract_route_execution_kernel(&malformed),
            Err(RouteExecutionInvalidReason::SourceEvidenceInvalid { .. })
        ));
    }

    #[test]
    fn route_has_full_state_clearance_and_named_sample_identity() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        let physical = &output.samples[0];
        assert_eq!(physical.state.mass_kg, 10.0);
        assert_eq!(physical.state.fuel_kg, 5.0);
        assert!(physical.clearance.observed_touchdown_clearance_m.is_some());
        assert!(physical.clearance.observed_hull_clearance_m.is_finite());
        assert!(
            physical
                .clearance
                .contact_envelope
                .minimum_clearance_m
                .is_finite()
        );
        assert!(
            physical
                .clearance
                .resolved_envelope
                .minimum_clearance_m
                .is_finite()
        );
        assert!(
            physical
                .clearance
                .full_envelope
                .minimum_clearance_m
                .is_finite()
        );
        assert_eq!(output.samples[2].sample_index, 2);
        assert_eq!(output.samples[2].physics_step, 2);
        assert_eq!(output.samples[2].sim_time_s, 0.2);
    }

    #[test]
    fn controller_markers_change_audit_only_and_compute_time_is_ignored() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 1.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample(3, 5.0),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let source = super::super::source_transition::assemble_source_transition_evidence(
            &source_input(&input),
            SourceTransitionProvenance::default(),
            Some(terminal(3, "checkpoint_satisfied")),
        );
        let update = |phase: &str, compute_time_us: Option<u64>| ControllerUpdateRecord {
            sim_time_s: 0.3,
            physics_step: 3,
            controller_update_index: 0,
            compute_time_us,
            frame: pd_control::ControllerFrame {
                command: Command::idle(),
                status: phase.to_owned(),
                phase: Some(phase.to_owned()),
                metrics: BTreeMap::new(),
                markers: vec![ControllerMarker {
                    id: "waypoint/handoff".to_owned(),
                    label: "handoff".to_owned(),
                    x_m: Some(6.0),
                    y_m: Some(20.0),
                    metadata: BTreeMap::new(),
                }],
            },
        };
        let first = assemble_route_execution_evidence(
            &input,
            source.clone(),
            RouteExecutionProvenance::default(),
            Some(terminal(3, "checkpoint_satisfied")),
            &[update("handoff_a", Some(1))],
        );
        let phase_changed = assemble_route_execution_evidence(
            &input,
            source.clone(),
            RouteExecutionProvenance::default(),
            Some(terminal(3, "checkpoint_satisfied")),
            &[update("handoff_b", Some(1))],
        );
        let compute_changed = assemble_route_execution_evidence(
            &input,
            source,
            RouteExecutionProvenance::default(),
            Some(terminal(3, "checkpoint_satisfied")),
            &[update("handoff_a", Some(999_999))],
        );
        assert_eq!(first.physical_digest, phase_changed.physical_digest);
        assert_ne!(first.evidence_digest, phase_changed.evidence_digest);
        assert_eq!(first.evidence_digest, compute_changed.evidence_digest);
        assert_eq!(
            first.audit.controller_handoff_markers[0].phase.as_deref(),
            Some("handoff_a")
        );
        let serialized = serde_json::to_string(&first).unwrap();
        let decoded: RouteExecutionEvidence = serde_json::from_str(&serialized).unwrap();
        assert_eq!(decoded.evidence_digest, first.evidence_digest);
        assert_eq!(decoded.physical_digest, first.physical_digest);
        assert_eq!(
            route_execution_evidence_digest(&decoded),
            decoded.evidence_digest
        );
        assert_eq!(
            route_execution_physical_digest(&decoded),
            decoded.physical_digest
        );
        validate_persisted_route_execution_evidence(&decoded).unwrap();
        let mut malformed = decoded;
        malformed.legs[0].leg_index = 99;
        let malformed = seal_route_execution_evidence(malformed);
        assert!(validate_persisted_route_execution_evidence(&malformed).is_err());
    }

    #[test]
    fn neutral_kinematics_matches_zero_speed_review_formula() {
        let initial = VehicleInitialState {
            position_m: Vec2::new(0.0, 20.0),
            velocity_mps: Vec2::new(5.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        };
        let geometry = NormalizedRouteGeometry {
            horizontal_sign: 1,
            direct_horizontal_span_m: 10.0,
            direct_distance_m: 10.0,
            route_angle_rad: 0.0,
            route_angle_deg: 0.0,
        };
        let profile = profile();
        let terrain = terrain();
        let centerline = [
            Vec2::new(0.0, 20.0),
            Vec2::new(3.0, 20.0),
            Vec2::new(4.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(8.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        let waypoints = [waypoint("wp0", 6.0, 0.0)];
        let samples = vec![
            sample(0, 0.0),
            sample(1, 2.0),
            sample(2, 4.0),
            sample_with_velocity(3, 5.0, Vec2::new(0.0, 0.0)),
            sample_with_velocity(4, 6.0, Vec2::new(0.0, 0.0)),
        ];
        let input = input(
            &initial,
            &geometry,
            &profile,
            &terrain,
            &centerline,
            &waypoints,
            &samples,
        );
        let output = extract_route_execution_kernel(&input).unwrap();
        let expected = neutral_waypoint_sample_stats(
            Vec2::new(5.0, 20.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 20.0),
            Vec2::new(6.0, 20.0),
            Vec2::new(10.0, 20.0),
            Some(Vec2::new(1.0, 0.0)),
        )
        .unwrap();
        let actual = output.waypoints[0].opportunity[0].kinematics;
        assert_eq!(actual.distance_m, expected.distance_m);
        assert_eq!(actual.plane_progress_m, expected.plane_progress_m);
        assert_eq!(actual.outbound_progress_mps, 0.0);
        assert_eq!(actual.speed_mps, 0.0);
        assert_eq!(
            actual.outbound_heading_error_rad,
            std::f64::consts::FRAC_PI_2
        );
    }
}
