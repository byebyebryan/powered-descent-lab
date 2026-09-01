//! Deterministic numerical foundations for the D1b progress-interval envelope.
//!
//! The model is intentionally evaluator-owned.  Nothing in this module is a
//! planner or controller decision surface, and none of its transforms admit
//! scenario identity, outcome, controller, or audit fields.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    f64::consts::{PI, TAU},
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{
    CircularAttitudeIntervalV1, FiniteIntervalV1, PhaseStateSetV1, RouteCapabilityArtifactV1,
    RouteCapabilityConfigurationV1, RouteCapabilityDomainV1, RouteCapabilityInputV1,
    RouteCapabilityTransformV1, RouteExecutionEvidence, RouteExecutionEvidenceStatus,
    RouteExecutionSample, RouteTopology, SourceTransitionEvidenceStatus, SourceTransitionSample,
    canonical_digest, route_execution_evidence_digest, route_execution_physical_digest,
    source_transition_evidence_digest, source_transition_physical_digest,
    validate_persisted_route_execution_evidence,
};

pub const PROGRESS_INTERVAL_ENVELOPE_ALTERNATIVE: &str = "progress_interval_envelope_v1";
pub const PROGRESS_INTERVAL_BIN_COUNT: usize = 32;
pub const PROGRESS_INTERVAL_MIN_DISTINCT_INPUT_DIGESTS: usize = 3;
pub const REASON_AMBIGUOUS_ATTITUDE_ARC: &str = "unknown/numerical/ambiguous_attitude_arc";

const DEFAULT_HULL_TOLERANCE: f64 = 1.0e-9;
const DEFAULT_GEOMETRY_TOLERANCE_M: f64 = 1.0e-9;

/// Complete fixed numerical configuration for the D1b baseline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalEnvelopeConfigV1 {
    pub version: String,
    pub bin_count: usize,
    pub minimum_distinct_input_digests: usize,
    pub bin_boundary_rule: String,
    pub scalar_bound_rule: String,
    pub attitude_bound_rule: String,
    pub fitted_padding: f64,
    pub hull_tolerance: f64,
    pub geometry_tolerance_m: f64,
}

impl Default for ProgressIntervalEnvelopeConfigV1 {
    fn default() -> Self {
        Self {
            version: PROGRESS_INTERVAL_ENVELOPE_ALTERNATIVE.to_owned(),
            bin_count: PROGRESS_INTERVAL_BIN_COUNT,
            minimum_distinct_input_digests: PROGRESS_INTERVAL_MIN_DISTINCT_INPUT_DIGESTS,
            bin_boundary_rule: "equal_normalized_half_open_final_closed".to_owned(),
            scalar_bound_rule: "observed_min_max_outward_one_f64_ulp".to_owned(),
            attitude_bound_rule: "unique_smallest_circular_arc_below_pi".to_owned(),
            fitted_padding: 0.0,
            hull_tolerance: DEFAULT_HULL_TOLERANCE,
            geometry_tolerance_m: DEFAULT_GEOMETRY_TOLERANCE_M,
        }
    }
}

impl ProgressIntervalEnvelopeConfigV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != PROGRESS_INTERVAL_ENVELOPE_ALTERNATIVE {
            return Err(format!(
                "progress-interval version must equal {PROGRESS_INTERVAL_ENVELOPE_ALTERNATIVE}"
            ));
        }
        if self.bin_count != PROGRESS_INTERVAL_BIN_COUNT {
            return Err(format!(
                "progress-interval bin_count must equal {PROGRESS_INTERVAL_BIN_COUNT}"
            ));
        }
        if self.minimum_distinct_input_digests != PROGRESS_INTERVAL_MIN_DISTINCT_INPUT_DIGESTS {
            return Err(format!(
                "minimum distinct input digests must equal {PROGRESS_INTERVAL_MIN_DISTINCT_INPUT_DIGESTS}"
            ));
        }
        if self.bin_boundary_rule != "equal_normalized_half_open_final_closed"
            || self.scalar_bound_rule != "observed_min_max_outward_one_f64_ulp"
            || self.attitude_bound_rule != "unique_smallest_circular_arc_below_pi"
        {
            return Err("progress-interval numerical rules do not match V1".to_owned());
        }
        if self.fitted_padding != 0.0 {
            return Err("progress-interval V1 has no fitted padding".to_owned());
        }
        for (name, value) in [
            ("hull_tolerance", self.hull_tolerance),
            ("geometry_tolerance_m", self.geometry_tolerance_m),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("{name} must be finite and non-negative"));
            }
        }
        if self.hull_tolerance != DEFAULT_HULL_TOLERANCE
            || self.geometry_tolerance_m != DEFAULT_GEOMETRY_TOLERANCE_M
        {
            return Err("progress-interval V1 tolerances are fixed, not tunable".to_owned());
        }
        Ok(())
    }

    pub fn capability_configuration(&self) -> Result<RouteCapabilityConfigurationV1, String> {
        self.validate()?;
        let configuration = RouteCapabilityConfigurationV1 {
            version: self.version.clone(),
            transforms: vec![RouteCapabilityTransformV1 {
                name: "route_relative_progress_interval_envelope".to_owned(),
                version: "v1".to_owned(),
                parameters: BTreeMap::from([
                    ("bin_count".to_owned(), self.bin_count as f64),
                    (
                        "minimum_distinct_input_digests".to_owned(),
                        self.minimum_distinct_input_digests as f64,
                    ),
                ]),
            }],
            interpolation: BTreeMap::from([
                ("binning".to_owned(), self.bin_boundary_rule.clone()),
                ("scalar_bounds".to_owned(), self.scalar_bound_rule.clone()),
                (
                    "attitude_bounds".to_owned(),
                    self.attitude_bound_rule.clone(),
                ),
                (
                    "stratum_interpolation".to_owned(),
                    "same_stratum_convex_hull_only".to_owned(),
                ),
            ]),
            padding: BTreeMap::new(),
            solver_limits: BTreeMap::from([
                ("hull_tolerance".to_owned(), self.hull_tolerance),
                ("geometry_tolerance_m".to_owned(), self.geometry_tolerance_m),
            ]),
            reason_code_mapping: BTreeMap::from([
                (
                    "ambiguous_attitude_arc".to_owned(),
                    REASON_AMBIGUOUS_ATTITUDE_ARC.to_owned(),
                ),
                (
                    "outside_same_stratum_convex_hull".to_owned(),
                    "unknown/coverage/outside_same_stratum_convex_hull".to_owned(),
                ),
                (
                    "undercovered_phase_bin".to_owned(),
                    "unknown/coverage/undercovered_phase_bin".to_owned(),
                ),
            ]),
        };
        configuration.validate()?;
        Ok(configuration)
    }
}

/// Assign normalized phase progress to the exact V1 bin.
pub fn progress_bin_index(normalized_progress: f64) -> Result<usize, String> {
    if !normalized_progress.is_finite() || !(0.0..=1.0).contains(&normalized_progress) {
        return Err("normalized progress must be finite and within [0, 1]".to_owned());
    }
    if normalized_progress == 1.0 {
        return Ok(PROGRESS_INTERVAL_BIN_COUNT - 1);
    }
    Ok((normalized_progress * PROGRESS_INTERVAL_BIN_COUNT as f64).floor() as usize)
}

pub fn next_up(value: f64) -> Result<f64, String> {
    if !value.is_finite() {
        return Err("next-up input must be finite".to_owned());
    }
    let next = if value == 0.0 {
        f64::from_bits(1)
    } else if value > 0.0 {
        f64::from_bits(value.to_bits() + 1)
    } else {
        f64::from_bits(value.to_bits() - 1)
    };
    next.is_finite()
        .then_some(next)
        .ok_or_else(|| "one-ULP upper expansion is not finite".to_owned())
}

pub fn next_down(value: f64) -> Result<f64, String> {
    if !value.is_finite() {
        return Err("next-down input must be finite".to_owned());
    }
    let next = if value == 0.0 {
        -f64::from_bits(1)
    } else if value > 0.0 {
        f64::from_bits(value.to_bits() - 1)
    } else {
        f64::from_bits(value.to_bits() + 1)
    };
    next.is_finite()
        .then_some(next)
        .ok_or_else(|| "one-ULP lower expansion is not finite".to_owned())
}

/// Expand a finite observed range outward by exactly one representable value.
pub fn outward_interval(minimum: f64, maximum: f64) -> Result<FiniteIntervalV1, String> {
    if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum {
        return Err("observed interval must be finite and ordered".to_owned());
    }
    FiniteIntervalV1::new(next_down(minimum)?, next_up(maximum)?)
}

fn normalized_angle(angle_rad: f64) -> f64 {
    let normalized = angle_rad.rem_euclid(TAU);
    if normalized >= PI {
        normalized - TAU
    } else {
        normalized
    }
}

/// Return the unique smallest circular interval containing all samples.
pub fn smallest_circular_interval(
    angles_rad: &[f64],
) -> Result<CircularAttitudeIntervalV1, String> {
    if angles_rad.is_empty() || angles_rad.iter().any(|angle| !angle.is_finite()) {
        return Err("attitude samples must be non-empty and finite".to_owned());
    }
    let mut angles = angles_rad
        .iter()
        .map(|angle| normalized_angle(*angle).rem_euclid(TAU))
        .collect::<Vec<_>>();
    angles.sort_by(f64::total_cmp);
    angles.dedup_by(|left, right| left.total_cmp(right) == Ordering::Equal);
    if angles.len() == 1 {
        return CircularAttitudeIntervalV1::new(normalized_angle(angles[0]), 0.0);
    }

    let mut gaps = angles
        .windows(2)
        .enumerate()
        .map(|(index, pair)| (index, pair[1] - pair[0]))
        .collect::<Vec<_>>();
    gaps.push((angles.len() - 1, angles[0] + TAU - angles[angles.len() - 1]));
    let largest_gap = gaps
        .iter()
        .map(|(_, gap)| *gap)
        .max_by(f64::total_cmp)
        .ok_or_else(|| "attitude gap computation failed".to_owned())?;
    let largest_count = gaps
        .iter()
        .filter(|(_, gap)| gap.total_cmp(&largest_gap) == Ordering::Equal)
        .count();
    let gap_index = gaps
        .iter()
        .position(|(_, gap)| gap.total_cmp(&largest_gap) == Ordering::Equal)
        .ok_or_else(|| "attitude gap selection failed".to_owned())?;
    let start = angles[(gap_index + 1) % angles.len()];
    let end = angles[gap_index];
    let arc_width = if gap_index + 1 == angles.len() {
        end - start
    } else {
        end + TAU - start
    };
    if largest_count != 1 || arc_width >= PI {
        return Err(REASON_AMBIGUOUS_ATTITUDE_ARC.to_owned());
    }
    let center = normalized_angle(start + arc_width * 0.5);
    CircularAttitudeIntervalV1::new(center, arc_width * 0.5)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HullPointV1 {
    pub route_angle_deg: f64,
    pub route_radius_m: f64,
}

impl HullPointV1 {
    fn validate(self) -> Result<(), String> {
        if !self.route_angle_deg.is_finite() || !self.route_radius_m.is_finite() {
            return Err("hull points must be finite".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HullContainmentV1 {
    Inside,
    Boundary,
    Outside,
}

fn point_order(left: &HullPointV1, right: &HullPointV1) -> Ordering {
    left.route_angle_deg
        .total_cmp(&right.route_angle_deg)
        .then_with(|| left.route_radius_m.total_cmp(&right.route_radius_m))
}

fn cross(origin: HullPointV1, first: HullPointV1, second: HullPointV1) -> f64 {
    (first.route_angle_deg - origin.route_angle_deg)
        * (second.route_radius_m - origin.route_radius_m)
        - (first.route_radius_m - origin.route_radius_m)
            * (second.route_angle_deg - origin.route_angle_deg)
}

/// Deterministic counter-clockwise monotone-chain hull with canonical start.
pub fn convex_hull(points: &[HullPointV1], tolerance: f64) -> Result<Vec<HullPointV1>, String> {
    if points.is_empty() {
        return Err("convex hull requires at least one point".to_owned());
    }
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err("convex hull tolerance must be finite and non-negative".to_owned());
    }
    let mut points = points
        .iter()
        .map(|point| HullPointV1 {
            route_angle_deg: canonical_zero(point.route_angle_deg),
            route_radius_m: canonical_zero(point.route_radius_m),
        })
        .collect::<Vec<_>>();
    for point in &points {
        point.validate()?;
    }
    points.sort_by(point_order);
    points.dedup_by(|left, right| point_order(left, right) == Ordering::Equal);
    if points.len() <= 2 {
        return Ok(points);
    }
    let mut lower = Vec::new();
    for point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }
    let mut upper = Vec::new();
    for point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    Ok(lower)
}

fn point_on_segment(
    point: HullPointV1,
    start: HullPointV1,
    end: HullPointV1,
    tolerance: f64,
) -> bool {
    perpendicular_distance(point, start, end) <= tolerance
        && point.route_angle_deg >= start.route_angle_deg.min(end.route_angle_deg) - tolerance
        && point.route_angle_deg <= start.route_angle_deg.max(end.route_angle_deg) + tolerance
        && point.route_radius_m >= start.route_radius_m.min(end.route_radius_m) - tolerance
        && point.route_radius_m <= start.route_radius_m.max(end.route_radius_m) + tolerance
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn perpendicular_distance(point: HullPointV1, start: HullPointV1, end: HullPointV1) -> f64 {
    let angle_delta = end.route_angle_deg - start.route_angle_deg;
    let radius_delta = end.route_radius_m - start.route_radius_m;
    let length = angle_delta.hypot(radius_delta);
    if length == 0.0 {
        (point.route_angle_deg - start.route_angle_deg)
            .hypot(point.route_radius_m - start.route_radius_m)
    } else {
        cross(start, end, point).abs() / length
    }
}

fn signed_perpendicular_distance(point: HullPointV1, start: HullPointV1, end: HullPointV1) -> f64 {
    let length = (end.route_angle_deg - start.route_angle_deg)
        .hypot(end.route_radius_m - start.route_radius_m);
    if length == 0.0 {
        0.0
    } else {
        cross(start, end, point) / length
    }
}

/// Classify a query against a canonical convex hull, including its boundary.
pub fn classify_hull_point(
    hull: &[HullPointV1],
    query: HullPointV1,
    tolerance: f64,
) -> Result<HullContainmentV1, String> {
    query.validate()?;
    if hull.is_empty() || !tolerance.is_finite() || tolerance < 0.0 {
        return Err("hull and tolerance must be valid".to_owned());
    }
    for point in hull {
        point.validate()?;
    }
    if hull.len() == 1 {
        let point = hull[0];
        return Ok(
            if (point.route_angle_deg - query.route_angle_deg).abs() <= tolerance
                && (point.route_radius_m - query.route_radius_m).abs() <= tolerance
            {
                HullContainmentV1::Boundary
            } else {
                HullContainmentV1::Outside
            },
        );
    }
    if hull.len() == 2 {
        return Ok(if point_on_segment(query, hull[0], hull[1], tolerance) {
            HullContainmentV1::Boundary
        } else {
            HullContainmentV1::Outside
        });
    }

    let mut boundary = false;
    for index in 0..hull.len() {
        let start = hull[index];
        let end = hull[(index + 1) % hull.len()];
        let side = signed_perpendicular_distance(query, start, end);
        if side < -tolerance {
            return Ok(HullContainmentV1::Outside);
        }
        if side.abs() <= tolerance && point_on_segment(query, start, end, tolerance) {
            boundary = true;
        }
    }
    Ok(if boundary {
        HullContainmentV1::Boundary
    } else {
        HullContainmentV1::Inside
    })
}

/// Exact physical partition used for all fitting and prediction lookup.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ProgressIntervalStratumKeyV1 {
    pub terrain_geometry_digest: String,
    pub vehicle_physics_digest: String,
    pub waypoint_count: usize,
}

impl ProgressIntervalStratumKeyV1 {
    pub fn from_input(input: &RouteCapabilityInputV1) -> Result<Self, String> {
        input.validate()?;
        Ok(Self {
            terrain_geometry_digest: canonical_digest(&input.physical.terrain)?,
            vehicle_physics_digest: canonical_digest(&input.physical.vehicle)?,
            waypoint_count: input.physical.waypoints.len(),
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.terrain_geometry_digest.trim().is_empty()
            || self.vehicle_physics_digest.trim().is_empty()
            || self.waypoint_count == 0
            || self.waypoint_count > 2
        {
            return Err("progress-interval stratum key is invalid".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressIntervalContinuousPhaseV1 {
    PadDeparture,
    Acquisition,
    RouteLeg { leg_index: usize },
}

/// One neutral physical state transformed into a phase-relative observation.
/// Orthogonal projection defines path progress, so the longitudinal residual
/// is identically zero; the D0 cross-track value is the lateral residual.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalStateObservationV1 {
    pub normalized_phase_progress: f64,
    pub progress_m: f64,
    pub along_track_error_m: f64,
    pub cross_track_error_m: f64,
    pub along_track_velocity_mps: f64,
    pub cross_track_velocity_mps: f64,
    pub attitude_rad: f64,
    pub angular_rate_radps: f64,
    pub mass_kg: f64,
    pub fuel_kg: f64,
}

impl ProgressIntervalStateObservationV1 {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("normalized_phase_progress", self.normalized_phase_progress),
            ("progress_m", self.progress_m),
            ("along_track_error_m", self.along_track_error_m),
            ("cross_track_error_m", self.cross_track_error_m),
            ("along_track_velocity_mps", self.along_track_velocity_mps),
            ("cross_track_velocity_mps", self.cross_track_velocity_mps),
            ("attitude_rad", self.attitude_rad),
            ("angular_rate_radps", self.angular_rate_radps),
            ("mass_kg", self.mass_kg),
            ("fuel_kg", self.fuel_kg),
        ] {
            if !value.is_finite() {
                return Err(format!("training observation {name} must be finite"));
            }
        }
        if !(0.0..=1.0).contains(&self.normalized_phase_progress)
            || self.mass_kg <= 0.0
            || self.fuel_kg < 0.0
            || self.fuel_kg > self.mass_kg
        {
            return Err("training observation has invalid progress or mass".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalTrainingPhaseV1 {
    pub phase: ProgressIntervalContinuousPhaseV1,
    pub observations: Vec<ProgressIntervalStateObservationV1>,
    pub terminal: Option<ProgressIntervalStateObservationV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalTrainingHandoffV1 {
    pub waypoint_index: usize,
    pub terminal: ProgressIntervalStateObservationV1,
}

/// Outcome-blind training record.  It carries no run/case/seed/controller,
/// provenance, audit, terminal outcome, or development-comparison field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalTrainingRecordV1 {
    pub input_digest: String,
    pub evidence_physical_digest: String,
    pub stratum: ProgressIntervalStratumKeyV1,
    pub training_point: HullPointV1,
    pub horizontal_sign: i8,
    pub vehicle_dry_mass_kg: f64,
    pub initial_fuel_kg: f64,
    pub initial: ProgressIntervalStateObservationV1,
    pub continuous_phases: Vec<ProgressIntervalTrainingPhaseV1>,
    pub handoffs: Vec<ProgressIntervalTrainingHandoffV1>,
}

impl ProgressIntervalTrainingRecordV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.input_digest.trim().is_empty() || self.evidence_physical_digest.trim().is_empty() {
            return Err("training record digests must not be empty".to_owned());
        }
        self.stratum.validate()?;
        self.training_point.validate()?;
        if !matches!(self.horizontal_sign, -1 | 1)
            || !self.vehicle_dry_mass_kg.is_finite()
            || self.vehicle_dry_mass_kg <= 0.0
            || !self.initial_fuel_kg.is_finite()
            || self.initial_fuel_kg < 0.0
        {
            return Err("training record physical summary is invalid".to_owned());
        }
        self.initial.validate()?;
        let mut phases = BTreeSet::new();
        for phase in &self.continuous_phases {
            if !phases.insert(phase.phase.clone()) {
                return Err("training record contains duplicate continuous phases".to_owned());
            }
            for observation in &phase.observations {
                observation.validate()?;
            }
            if let Some(terminal) = &phase.terminal {
                terminal.validate()?;
            }
        }
        let mut handoffs = BTreeSet::new();
        for handoff in &self.handoffs {
            if handoff.waypoint_index >= self.stratum.waypoint_count
                || !handoffs.insert(handoff.waypoint_index)
            {
                return Err("training record contains an invalid handoff".to_owned());
            }
            handoff.terminal.validate()?;
        }
        Ok(())
    }

    pub fn from_evidence(
        input: &RouteCapabilityInputV1,
        evidence: &RouteExecutionEvidence,
    ) -> Result<Self, String> {
        validate_neutral_training_evidence(input, evidence)?;
        let source = &evidence.source_transition;
        let initial_anchor = source
            .initial_anchor
            .as_ref()
            .ok_or_else(|| "source evidence has no initial anchor".to_owned())?;
        let initial_sample = source
            .samples
            .iter()
            .find(|sample| sample.sample_index == initial_anchor.state.sample_index)
            .ok_or_else(|| "initial anchor has no authoritative source sample".to_owned())?;
        let initial_route_sample = route_sample(evidence, initial_sample.sample_index)
            .ok_or_else(|| "initial anchor has no authoritative route sample".to_owned())?;
        validate_initial_route_projection(input, initial_route_sample)?;
        let initial = route_observation(initial_route_sample, 0.0)?;

        let mut continuous_phases = Vec::new();
        let pad_end = source.contact_exit.as_ref().map_or_else(
            || source.samples.last().map(|sample| sample.sample_index),
            |bracket| Some(bracket.after.sample_index),
        );
        if let Some(pad_end) = pad_end {
            let observations = source_phase_observations(
                evidence,
                &source.samples,
                initial_anchor.state.sample_index,
                pad_end,
                initial_anchor.source_progress_m,
                source.source_transition_start_m,
            )?;
            let terminal = source
                .contact_exit
                .as_ref()
                .map(|bracket| route_boundary_observation(evidence, bracket.after.sample_index))
                .transpose()?;
            continuous_phases.push(ProgressIntervalTrainingPhaseV1 {
                phase: ProgressIntervalContinuousPhaseV1::PadDeparture,
                observations,
                terminal,
            });
        }

        if let Some(contact) = &source.contact_exit {
            let acquisition_end = source.tracking_entry.as_ref().map_or_else(
                || source.samples.last().map(|sample| sample.sample_index),
                |bracket| Some(bracket.after.sample_index),
            );
            if let Some(acquisition_end) = acquisition_end {
                let observations = source_phase_observations(
                    evidence,
                    &source.samples,
                    contact.after.sample_index,
                    acquisition_end,
                    source.source_transition_start_m,
                    source.source_transition_end_m,
                )?;
                let terminal = source
                    .tracking_entry
                    .as_ref()
                    .map(|bracket| route_boundary_observation(evidence, bracket.after.sample_index))
                    .transpose()?;
                continuous_phases.push(ProgressIntervalTrainingPhaseV1 {
                    phase: ProgressIntervalContinuousPhaseV1::Acquisition,
                    observations,
                    terminal,
                });
            }
        }

        let mut handoffs = Vec::new();
        for waypoint_index in 0..input.physical.waypoints.len() {
            let Some(leg) = evidence
                .legs
                .iter()
                .find(|leg| leg.leg_index == waypoint_index)
            else {
                continue;
            };
            let Some(start_index) = leg.start_boundary.sample_index else {
                continue;
            };
            let Some(start_sample) = route_sample(evidence, start_index) else {
                return Err(format!(
                    "route leg {waypoint_index} start sample is missing"
                ));
            };
            let resolution = evidence
                .waypoints
                .get(waypoint_index)
                .and_then(|waypoint| waypoint.resolution.as_ref());
            let end_index = if let Some(resolution) = resolution {
                route_sample(evidence, resolution.sample_index).ok_or_else(|| {
                    format!("route leg {waypoint_index} resolution sample is missing")
                })?;
                resolution.sample_index
            } else {
                evidence
                    .samples
                    .last()
                    .ok_or_else(|| "route evidence has no samples".to_owned())?
                    .sample_index
            };
            let end_progress = waypoint_path_progress(input, waypoint_index)?;
            let observations = if end_progress > start_sample.path.along_route_m {
                route_leg_observations(
                    evidence,
                    start_index,
                    end_index,
                    start_sample.path.along_route_m,
                    end_progress,
                )?
            } else {
                // A prior handoff may resolve after the next planned waypoint
                // progress on a diagnostic trace.  Such a leg has no forward
                // progress-indexed support in Alternative A; retain its
                // boundary evidence but leave every continuous bin uncovered.
                Vec::new()
            };
            let terminal = resolution
                .map(|resolution| route_boundary_observation(evidence, resolution.sample_index))
                .transpose()?;
            continuous_phases.push(ProgressIntervalTrainingPhaseV1 {
                phase: ProgressIntervalContinuousPhaseV1::RouteLeg {
                    leg_index: waypoint_index,
                },
                observations,
                terminal: terminal.clone(),
            });
            if let Some(terminal) = terminal {
                handoffs.push(ProgressIntervalTrainingHandoffV1 {
                    waypoint_index,
                    terminal,
                });
            }
        }

        continuous_phases.sort_by(|left, right| left.phase.cmp(&right.phase));
        let record = Self {
            input_digest: input.input_digest.clone(),
            evidence_physical_digest: evidence.physical_digest.clone(),
            stratum: ProgressIntervalStratumKeyV1::from_input(input)?,
            training_point: HullPointV1 {
                route_angle_deg: input.physical.route_angle_deg,
                route_radius_m: input.physical.route_radius_m,
            },
            horizontal_sign: input.physical.horizontal_sign,
            vehicle_dry_mass_kg: input.physical.vehicle.dry_mass_kg,
            initial_fuel_kg: input.physical.vehicle.initial_fuel_kg,
            initial,
            continuous_phases,
            handoffs,
        };
        record.validate()?;
        Ok(record)
    }
}

fn validate_neutral_training_evidence(
    input: &RouteCapabilityInputV1,
    evidence: &RouteExecutionEvidence,
) -> Result<(), String> {
    input.validate()?;
    validate_persisted_route_execution_evidence(evidence)?;
    if input.physical.topology != RouteTopology::Waypoint {
        return Err("progress-interval fitting requires waypoint topology".to_owned());
    }
    if evidence.status == RouteExecutionEvidenceStatus::Invalid
        || evidence.invalid_reason.is_some()
        || evidence.source_transition.status == SourceTransitionEvidenceStatus::Invalid
        || evidence.source_transition.invalid_reason.is_some()
    {
        return Err("progress-interval fitting cannot consume invalid D0 evidence".to_owned());
    }
    let recomputed_evidence_digest = route_execution_evidence_digest(evidence);
    if evidence.evidence_digest != recomputed_evidence_digest {
        let source_evidence_digest = source_transition_evidence_digest(&evidence.source_transition);
        let source_physical_digest = source_transition_physical_digest(&evidence.source_transition)
            .unwrap_or_else(|error| format!("error:{error}"));
        let route_physical_digest = route_execution_physical_digest(evidence);
        return Err(format!(
            "route evidence digest mismatch: stored {}, recomputed {recomputed_evidence_digest}; source stored {}, recomputed {source_evidence_digest}; source physical stored {}, recomputed {source_physical_digest}; route physical stored {}, recomputed {route_physical_digest}",
            evidence.evidence_digest,
            evidence.source_transition.evidence_digest,
            evidence.source_transition.physical_digest,
            evidence.physical_digest,
        ));
    }
    if evidence.source_transition.evidence_digest
        != source_transition_evidence_digest(&evidence.source_transition)
    {
        return Err("nested source evidence digest mismatch".to_owned());
    }
    if evidence.source_transition.physical_digest
        != source_transition_physical_digest(&evidence.source_transition)?
    {
        return Err("nested source physical digest mismatch".to_owned());
    }
    if evidence.physical_digest != route_execution_physical_digest(evidence) {
        return Err("route physical digest mismatch".to_owned());
    }
    if evidence.provenance.request_digest != input.provenance.request_digest
        || evidence.provenance.route_plan_digest != input.provenance.route_plan_digest
        || evidence.source_transition.provenance.request_digest != input.provenance.request_digest
        || evidence.source_transition.provenance.route_plan_digest
            != input.provenance.route_plan_digest
    {
        return Err("route evidence provenance does not match capability input".to_owned());
    }
    if evidence.provenance.source_evidence_digest != evidence.source_transition.evidence_digest
        || evidence.provenance.resolved_input_digest.trim().is_empty()
        || evidence.provenance.resolved_input_digest
            != evidence.source_transition.provenance.resolved_input_digest
        || evidence.provenance.request_digest
            != evidence.source_transition.provenance.request_digest
        || evidence.provenance.policy_digest != evidence.source_transition.provenance.policy_digest
        || evidence.provenance.route_plan_digest
            != evidence.source_transition.provenance.route_plan_digest
        || evidence.provenance.source_target_geometry_digest
            != evidence
                .source_transition
                .provenance
                .source_target_geometry_digest
        || evidence.provenance.raw_bundle_digest
            != evidence.source_transition.provenance.raw_bundle_digest
        || evidence.provenance.samples_digest
            != evidence.source_transition.provenance.samples_digest
        || evidence.provenance.action_log_digest
            != evidence.source_transition.provenance.action_log_digest
    {
        return Err("route and nested source evidence provenance do not match".to_owned());
    }
    if evidence.cadence.sample_hz != Some(evidence.cadence.physics_hz)
        || evidence.waypoints.len() != input.physical.waypoints.len()
        || evidence.legs.len() < input.physical.waypoints.len()
    {
        return Err("route evidence cadence or topology does not match input".to_owned());
    }
    if evidence.source_transition.source_transition_start_m
        != input.physical.safety_profile.source_transition_start_m
        || evidence.source_transition.source_transition_end_m
            != input.physical.safety_profile.source_transition_end_m
    {
        return Err("source transition bounds do not match capability input".to_owned());
    }
    for (index, (observed, expected)) in evidence
        .waypoints
        .iter()
        .zip(&input.physical.waypoints)
        .enumerate()
    {
        if observed.waypoint_index != index || !expected.matches_waypoint(&observed.contract) {
            return Err(format!(
                "route waypoint {index} does not match capability input"
            ));
        }
    }
    let expected_centerline = normalized_centerline(input);
    if evidence.selected_centerline_m != expected_centerline {
        return Err(
            "route evidence selected centerline does not match capability input".to_owned(),
        );
    }
    validate_recomputed_training_evidence(input, evidence)?;
    Ok(())
}

fn validate_recomputed_training_evidence(
    input: &RouteCapabilityInputV1,
    evidence: &RouteExecutionEvidence,
) -> Result<(), String> {
    let profile = pd_core::SafetyProfile {
        source_transition_start_m: input.physical.safety_profile.source_transition_start_m,
        source_transition_end_m: input.physical.safety_profile.source_transition_end_m,
        target_transition_start_m: input.physical.safety_profile.target_transition_start_m,
        target_transition_end_m: input.physical.safety_profile.target_transition_end_m,
        horizontal_span_m: input.physical.safety_profile.horizontal_span_m,
        full_envelope: input.physical.safety_profile.full_envelope,
        contact_envelope: input.physical.safety_profile.contact_envelope,
    };
    let waypoints = evidence
        .waypoints
        .iter()
        .map(|waypoint| waypoint.contract.clone())
        .collect::<Vec<_>>();
    let mut raw_states = BTreeMap::new();
    for sample in &evidence.samples {
        insert_reconstructed_state(&mut raw_states, sample.state)?;
    }
    for bracket in [
        evidence.source_transition.contact_exit.as_ref(),
        evidence.source_transition.tracking_entry.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        insert_reconstructed_state(&mut raw_states, bracket.before)?;
        insert_reconstructed_state(&mut raw_states, bracket.after)?;
    }
    for waypoint in &evidence.waypoints {
        for bracket in [waypoint.capture_entry.as_ref(), waypoint.deadline.as_ref()]
            .into_iter()
            .flatten()
        {
            insert_reconstructed_state(&mut raw_states, bracket.before)?;
            insert_reconstructed_state(&mut raw_states, bracket.after)?;
        }
    }
    let samples = raw_states
        .into_iter()
        .enumerate()
        .map(|(expected_index, (sample_index, state))| {
            if sample_index != expected_index {
                return Err(format!(
                    "reconstructed raw sample stream is not contiguous at {expected_index}"
                ));
            }
            reconstructed_sample_record(input, state)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let source_input = crate::SourceTransitionKernelInput {
        terrain: &input.physical.terrain,
        source_pad_center_x_m: input.physical.source_pad.center_x_m,
        vehicle: &input.physical.vehicle,
        initial_state: &input.physical.initial_state,
        geometry: &input.physical.normalized_geometry,
        profile: &profile,
        selected_centerline_m: &evidence.selected_centerline_m,
        waypoints: &waypoints,
        samples: &samples,
        cadence: evidence.cadence,
        audit: crate::SourceTransitionAuditInput::default(),
    };
    let recomputed_source = crate::extract_source_transition_kernel(&source_input)
        .map_err(|error| format!("source evidence recomputation failed: {error}"))?;
    let source = &evidence.source_transition;
    if source.initial_anchor.as_ref() != Some(&recomputed_source.initial_anchor)
        || source.contact_exit != recomputed_source.contact_exit
        || source.tracking_entry != recomputed_source.tracking_entry
        || source.boundary_centerline_references != recomputed_source.boundary_centerline_references
        || source.first_outbound_reference.as_ref()
            != Some(&recomputed_source.first_outbound_reference)
        || source.samples != recomputed_source.samples
        || source.pad_departure_extrema != recomputed_source.pad_departure_extrema
        || source.acquisition_extrema != recomputed_source.acquisition_extrema
        || source.boundary_crossings != recomputed_source.boundary_crossings
        || source.backtracking_events != recomputed_source.backtracking_events
        || source.reentry_events != recomputed_source.reentry_events
        || source.physical_digest != recomputed_source.physical_digest
    {
        return Err(
            "persisted source derived fields do not recompute from raw evidence".to_owned(),
        );
    }
    let route_input = crate::RouteExecutionKernelInput {
        terrain: &input.physical.terrain,
        source_pad_center_x_m: input.physical.source_pad.center_x_m,
        source_pad_surface_y_m: input.physical.source_pad.surface_y_m,
        target_pad_center_x_m: input.physical.target_pad.center_x_m,
        target_pad_surface_y_m: input.physical.target_pad.surface_y_m,
        vehicle: &input.physical.vehicle,
        initial_state: &input.physical.initial_state,
        geometry: &input.physical.normalized_geometry,
        profile: &profile,
        selected_centerline_m: &evidence.selected_centerline_m,
        waypoints: &waypoints,
        samples: &samples,
        cadence: evidence.cadence,
    };
    let recomputed_route = crate::extract_route_execution_kernel(&route_input)
        .map_err(|error| format!("route evidence recomputation failed: {error}"))?;
    if evidence.waypoints != recomputed_route.waypoints {
        let difference = crate::first_json_difference(
            &serde_json::to_value(&evidence.waypoints)
                .map_err(|error| format!("failed to inspect persisted waypoints: {error}"))?,
            &serde_json::to_value(&recomputed_route.waypoints)
                .map_err(|error| format!("failed to inspect recomputed waypoints: {error}"))?,
            "$.waypoints",
        )
        .unwrap_or_else(|| "unknown path".to_owned());
        return Err(format!(
            "persisted route waypoints do not recompute from raw evidence at {difference}"
        ));
    }
    for (matches, field) in [
        (evidence.samples == recomputed_route.samples, "samples"),
        (evidence.legs == recomputed_route.legs, "legs"),
        (
            evidence.backtracking_events == recomputed_route.backtracking_events,
            "backtracking_events",
        ),
        (
            evidence.reentry_events == recomputed_route.reentry_events,
            "reentry_events",
        ),
        (
            evidence.physical_digest
                == crate::source_transition_canonical_digest(&(
                    source.physical_digest.clone(),
                    recomputed_route.physical_digest.clone(),
                )),
            "physical_digest",
        ),
    ] {
        if !matches {
            return Err(format!(
                "persisted route {field} do not recompute from raw evidence"
            ));
        }
    }
    Ok(())
}

fn insert_reconstructed_state(
    states: &mut BTreeMap<usize, crate::SourceTransitionRawState>,
    state: crate::SourceTransitionRawState,
) -> Result<(), String> {
    if let Some(previous) = states.insert(state.sample_index, state)
        && previous != state
    {
        return Err(format!(
            "conflicting raw states for sample {}",
            state.sample_index
        ));
    }
    Ok(())
}

fn reconstructed_sample_record(
    input: &RouteCapabilityInputV1,
    state: crate::SourceTransitionRawState,
) -> Result<pd_core::SampleRecord, String> {
    let geometry = &input.physical.vehicle.geometry;
    let touchdown_points = [
        pd_core::Vec2::new(
            -geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
        pd_core::Vec2::new(
            geometry.touchdown_half_span_m,
            -geometry.touchdown_base_offset_m,
        ),
    ]
    .map(|point| state.position_m + point.rotated(state.attitude_rad));
    let touchdown_clearance_m = touchdown_points
        .iter()
        .map(|point| point.y - input.physical.terrain.sample_height(point.x))
        .fold(f64::INFINITY, f64::min);
    let half_width = geometry.hull_width_m * 0.5;
    let half_height = geometry.hull_height_m * 0.5;
    let hull_points = [
        pd_core::Vec2::new(-half_width, -half_height),
        pd_core::Vec2::new(half_width, -half_height),
        pd_core::Vec2::new(half_width, half_height),
        pd_core::Vec2::new(-half_width, half_height),
    ]
    .map(|point| state.position_m + point.rotated(state.attitude_rad));
    let min_hull_clearance_m = hull_points
        .iter()
        .map(|point| point.y - input.physical.terrain.sample_height(point.x))
        .fold(f64::INFINITY, f64::min);
    if !touchdown_clearance_m.is_finite() || !min_hull_clearance_m.is_finite() {
        return Err(format!(
            "reconstructed clearance is non-finite at sample {}",
            state.sample_index
        ));
    }
    Ok(pd_core::SampleRecord {
        sim_time_s: state.sim_time_s,
        physics_step: state.physics_step,
        observation: pd_core::Observation {
            sim_time_s: state.sim_time_s,
            physics_step: state.physics_step,
            position_m: state.position_m,
            velocity_mps: state.velocity_mps,
            attitude_rad: state.attitude_rad,
            angular_rate_radps: state.angular_rate_radps,
            mass_kg: state.mass_kg,
            fuel_kg: state.fuel_kg,
            gravity_mps2: input.physical.gravity_mps2,
            target_dx_m: input.physical.target_pad.center_x_m - state.position_m.x,
            height_above_target_m: state.position_m.y - input.physical.target_pad.surface_y_m,
            target_surface_y_m: input.physical.target_pad.surface_y_m,
            target_pad_half_width_m: input.physical.target_pad.width_m * 0.5,
            touchdown_clearance_m,
            min_hull_clearance_m,
        },
        held_command: pd_core::Command::idle(),
    })
}

fn normalized_phase_progress(value: f64, start: f64, end: f64) -> Result<f64, String> {
    if !value.is_finite() || !start.is_finite() || !end.is_finite() || end < start {
        return Err(format!(
            "phase progress bounds must be finite and non-decreasing: value={value}, start={start}, end={end}"
        ));
    }
    if end == start {
        return Ok(0.0);
    }
    Ok(((value - start) / (end - start)).clamp(0.0, 1.0))
}

fn source_phase_observations(
    evidence: &RouteExecutionEvidence,
    samples: &[SourceTransitionSample],
    first_index: usize,
    last_index: usize,
    start_progress_m: f64,
    end_progress_m: f64,
) -> Result<Vec<ProgressIntervalStateObservationV1>, String> {
    samples
        .iter()
        .filter(|sample| sample.sample_index >= first_index && sample.sample_index <= last_index)
        .map(|sample| {
            route_observation(
                route_sample(evidence, sample.sample_index).ok_or_else(|| {
                    format!("source sample {} has no route sample", sample.sample_index)
                })?,
                normalized_phase_progress(
                    sample.source_progress_m,
                    start_progress_m,
                    end_progress_m,
                )?,
            )
        })
        .collect()
}

fn route_boundary_observation(
    evidence: &RouteExecutionEvidence,
    sample_index: usize,
) -> Result<ProgressIntervalStateObservationV1, String> {
    route_observation(
        route_sample(evidence, sample_index)
            .ok_or_else(|| format!("route boundary sample {sample_index} is missing"))?,
        1.0,
    )
}

fn route_sample(
    evidence: &RouteExecutionEvidence,
    sample_index: usize,
) -> Option<&RouteExecutionSample> {
    evidence
        .samples
        .iter()
        .find(|sample| sample.sample_index == sample_index)
}

fn route_leg_observations(
    evidence: &RouteExecutionEvidence,
    first_index: usize,
    last_index: usize,
    start_progress_m: f64,
    end_progress_m: f64,
) -> Result<Vec<ProgressIntervalStateObservationV1>, String> {
    if last_index < first_index {
        return Err("route leg boundary indices are reversed".to_owned());
    }
    let observations = evidence
        .samples
        .iter()
        .filter(|sample| sample.sample_index >= first_index && sample.sample_index <= last_index)
        .map(|sample| {
            route_observation(
                sample,
                normalized_phase_progress(
                    sample.path.along_route_m,
                    start_progress_m,
                    end_progress_m,
                )?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if observations.is_empty() {
        return Err("route leg boundary interval has no authoritative samples".to_owned());
    }
    Ok(observations)
}

fn route_observation(
    sample: &RouteExecutionSample,
    normalized_progress: f64,
) -> Result<ProgressIntervalStateObservationV1, String> {
    let observation = ProgressIntervalStateObservationV1 {
        normalized_phase_progress: normalized_progress,
        progress_m: sample.path.along_route_m,
        along_track_error_m: 0.0,
        cross_track_error_m: sample.path.cross_track_m,
        along_track_velocity_mps: sample.path.velocity_along_route_mps,
        cross_track_velocity_mps: sample.path.velocity_cross_route_mps,
        attitude_rad: sample.state.attitude_rad,
        angular_rate_radps: sample.state.angular_rate_radps,
        mass_kg: sample.state.mass_kg,
        fuel_kg: sample.state.fuel_kg,
    };
    observation.validate()?;
    Ok(observation)
}

fn validate_initial_route_projection(
    input: &RouteCapabilityInputV1,
    sample: &RouteExecutionSample,
) -> Result<(), String> {
    let initial = &input.physical.initial_state;
    for (name, observed, expected) in [
        (
            "position_x",
            sample.state.position_m.x,
            initial.position_m.x,
        ),
        (
            "position_y",
            sample.state.position_m.y,
            initial.position_m.y,
        ),
        (
            "velocity_x",
            sample.state.velocity_mps.x,
            initial.velocity_mps.x,
        ),
        (
            "velocity_y",
            sample.state.velocity_mps.y,
            initial.velocity_mps.y,
        ),
        ("attitude", sample.state.attitude_rad, initial.attitude_rad),
        (
            "angular_rate",
            sample.state.angular_rate_radps,
            initial.angular_rate_radps,
        ),
    ] {
        if (observed - expected).abs() > DEFAULT_GEOMETRY_TOLERANCE_M {
            return Err(format!("initial route sample {name} does not match input"));
        }
    }
    let projected = route_coordinates_for_state(
        input,
        initial.position_m,
        initial.velocity_mps,
        DEFAULT_GEOMETRY_TOLERANCE_M,
    )?;
    for (name, observed, expected) in [
        ("progress", sample.path.along_route_m, projected.progress_m),
        (
            "cross_track",
            sample.path.cross_track_m,
            projected.cross_track_error_m,
        ),
        (
            "along_velocity",
            sample.path.velocity_along_route_mps,
            projected.along_track_velocity_mps,
        ),
        (
            "cross_velocity",
            sample.path.velocity_cross_route_mps,
            projected.cross_track_velocity_mps,
        ),
    ] {
        if (observed - expected).abs() > DEFAULT_GEOMETRY_TOLERANCE_M {
            return Err(format!("initial route projection {name} does not match D0"));
        }
    }
    Ok(())
}

fn waypoint_path_progress(
    input: &RouteCapabilityInputV1,
    waypoint_index: usize,
) -> Result<f64, String> {
    let waypoint = input
        .physical
        .waypoints
        .get(waypoint_index)
        .ok_or_else(|| format!("waypoint {waypoint_index} is missing"))?;
    let centerline = &input.physical.selected_centerline_m;
    let mut cumulative = 0.0;
    let mut best: Option<(f64, f64)> = None;
    for pair in centerline.windows(2) {
        let start = pair[0];
        let end = pair[1];
        let delta = end - start;
        let length_squared = delta.x.mul_add(delta.x, delta.y * delta.y);
        if length_squared <= 0.0 || !length_squared.is_finite() {
            return Err("selected centerline contains a degenerate segment".to_owned());
        }
        let relative = waypoint.position_m - start;
        let fraction =
            ((relative.x.mul_add(delta.x, relative.y * delta.y)) / length_squared).clamp(0.0, 1.0);
        let projected = start + delta * fraction;
        let residual = waypoint.position_m - projected;
        let distance_squared = residual.x.mul_add(residual.x, residual.y * residual.y);
        let segment_length = length_squared.sqrt();
        let progress = cumulative + fraction * segment_length;
        if best.is_none_or(|(best_distance, _)| distance_squared < best_distance) {
            best = Some((distance_squared, progress));
        }
        cumulative += segment_length;
    }
    let (distance_squared, progress) =
        best.ok_or_else(|| "selected centerline has no segments".to_owned())?;
    if distance_squared.sqrt() > DEFAULT_GEOMETRY_TOLERANCE_M {
        return Err(format!(
            "waypoint {waypoint_index} is not on the selected centerline"
        ));
    }
    Ok(progress)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalEnvelopeCellV1 {
    pub bin_index: usize,
    pub distinct_input_digests: Vec<String>,
    pub envelope: Option<PhaseStateSetV1>,
    pub issue_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalBoundaryCellV1 {
    pub distinct_input_digests: Vec<String>,
    pub envelope: Option<PhaseStateSetV1>,
    pub issue_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalPhaseModelV1 {
    pub phase: ProgressIntervalContinuousPhaseV1,
    pub bins: Vec<ProgressIntervalEnvelopeCellV1>,
    pub terminal: ProgressIntervalBoundaryCellV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalHandoffModelV1 {
    pub waypoint_index: usize,
    pub terminal: ProgressIntervalBoundaryCellV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalStratumModelV1 {
    pub key: ProgressIntervalStratumKeyV1,
    pub training_points: Vec<HullPointV1>,
    pub convex_hull: Vec<HullPointV1>,
    pub phases: Vec<ProgressIntervalPhaseModelV1>,
    pub handoffs: Vec<ProgressIntervalHandoffModelV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalEnvelopeModelV1 {
    pub schema_version: u32,
    pub configuration: ProgressIntervalEnvelopeConfigV1,
    pub strata: Vec<ProgressIntervalStratumModelV1>,
    pub model_digest: String,
}

impl ProgressIntervalEnvelopeModelV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.strata.is_empty() {
            return Err("progress-interval model schema or strata are invalid".to_owned());
        }
        self.configuration.validate()?;
        let mut previous_key = None;
        for stratum in &self.strata {
            stratum.key.validate()?;
            if previous_key.as_ref().is_some_and(|key| key >= &stratum.key) {
                return Err("progress-interval strata must be uniquely sorted".to_owned());
            }
            previous_key = Some(stratum.key.clone());
            validate_sorted_unique_points(&stratum.training_points)?;
            if stratum.convex_hull
                != convex_hull(&stratum.training_points, self.configuration.hull_tolerance)?
            {
                return Err("progress-interval stratum hull is not canonical".to_owned());
            }
            if stratum
                .phases
                .iter()
                .map(|phase| phase.phase.clone())
                .collect::<Vec<_>>()
                != expected_continuous_phases(stratum.key.waypoint_count)
            {
                return Err(
                    "progress-interval phases are not in canonical composition order".to_owned(),
                );
            }
            for phase in &stratum.phases {
                if phase.bins.len() != PROGRESS_INTERVAL_BIN_COUNT {
                    return Err("progress-interval phase layout is invalid".to_owned());
                }
                for (index, cell) in phase.bins.iter().enumerate() {
                    if cell.bin_index != index {
                        return Err("progress-interval bin indices are not canonical".to_owned());
                    }
                    validate_boundary_cell(
                        &cell.distinct_input_digests,
                        cell.envelope.as_ref(),
                        cell.issue_reason.as_deref(),
                    )?;
                }
                validate_boundary_cell(
                    &phase.terminal.distinct_input_digests,
                    phase.terminal.envelope.as_ref(),
                    phase.terminal.issue_reason.as_deref(),
                )?;
            }
            if stratum
                .handoffs
                .iter()
                .map(|handoff| handoff.waypoint_index)
                .collect::<Vec<_>>()
                != (0..stratum.key.waypoint_count).collect::<Vec<_>>()
            {
                return Err("progress-interval handoffs are not complete and ordered".to_owned());
            }
            for handoff in &stratum.handoffs {
                if handoff.waypoint_index >= stratum.key.waypoint_count {
                    return Err("progress-interval handoff layout is invalid".to_owned());
                }
                validate_boundary_cell(
                    &handoff.terminal.distinct_input_digests,
                    handoff.terminal.envelope.as_ref(),
                    handoff.terminal.issue_reason.as_deref(),
                )?;
            }
        }
        if self.model_digest != self.compute_digest()? {
            return Err("progress-interval model digest mismatch".to_owned());
        }
        Ok(())
    }

    fn compute_digest(&self) -> Result<String, String> {
        let mut material = self.clone();
        material.model_digest.clear();
        canonical_digest(&material)
    }

    fn seal(mut self) -> Result<Self, String> {
        self.model_digest = self.compute_digest()?;
        self.validate()?;
        Ok(self)
    }
}

fn validate_sorted_unique_points(points: &[HullPointV1]) -> Result<(), String> {
    if points.is_empty() {
        return Err("progress-interval training points must not be empty".to_owned());
    }
    for point in points {
        point.validate()?;
        if point.route_angle_deg.to_bits() == (-0.0_f64).to_bits()
            || point.route_radius_m.to_bits() == (-0.0_f64).to_bits()
        {
            return Err("progress-interval training points must use canonical zero".to_owned());
        }
    }
    if points
        .windows(2)
        .any(|pair| point_order(&pair[0], &pair[1]) != Ordering::Less)
    {
        return Err("progress-interval training points must be uniquely sorted".to_owned());
    }
    Ok(())
}

fn expected_continuous_phases(waypoint_count: usize) -> Vec<ProgressIntervalContinuousPhaseV1> {
    std::iter::once(ProgressIntervalContinuousPhaseV1::PadDeparture)
        .chain(std::iter::once(
            ProgressIntervalContinuousPhaseV1::Acquisition,
        ))
        .chain(
            (0..waypoint_count)
                .map(|leg_index| ProgressIntervalContinuousPhaseV1::RouteLeg { leg_index }),
        )
        .collect()
}

fn validate_boundary_cell(
    digests: &[String],
    envelope: Option<&PhaseStateSetV1>,
    issue_reason: Option<&str>,
) -> Result<(), String> {
    if digests.iter().any(|digest| digest.trim().is_empty())
        || digests.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("progress-interval coverage digests must be uniquely sorted".to_owned());
    }
    if let Some(envelope) = envelope {
        envelope.validate()?;
    }
    if envelope.is_none() && issue_reason.is_none() {
        return Err("an empty progress-interval cell requires an issue reason".to_owned());
    }
    if let Some(reason) = issue_reason {
        crate::validate_reason_code(reason)?;
    }
    Ok(())
}

/// Fit the fixed D1b baseline without access to outcomes or comparison rows.
pub fn fit_progress_interval_envelope(
    records: &[ProgressIntervalTrainingRecordV1],
    configuration: ProgressIntervalEnvelopeConfigV1,
) -> Result<RouteCapabilityArtifactV1, String> {
    configuration.validate()?;
    if records.is_empty() {
        return Err("progress-interval fitting requires training records".to_owned());
    }
    for record in records {
        record.validate()?;
    }
    let mut grouped =
        BTreeMap::<ProgressIntervalStratumKeyV1, Vec<&ProgressIntervalTrainingRecordV1>>::new();
    for record in records {
        grouped
            .entry(record.stratum.clone())
            .or_default()
            .push(record);
    }
    let mut strata = Vec::with_capacity(grouped.len());
    for (key, mut stratum_records) in grouped {
        stratum_records.sort_by(|left, right| {
            left.input_digest.cmp(&right.input_digest).then_with(|| {
                left.evidence_physical_digest
                    .cmp(&right.evidence_physical_digest)
            })
        });
        strata.push(fit_stratum(&key, &stratum_records, &configuration)?);
    }
    let model = ProgressIntervalEnvelopeModelV1 {
        schema_version: 1,
        configuration: configuration.clone(),
        strata,
        model_digest: String::new(),
    }
    .seal()?;

    let angles = records
        .iter()
        .map(|record| record.training_point.route_angle_deg);
    let radii = records
        .iter()
        .map(|record| record.training_point.route_radius_m);
    let dry_masses = records.iter().map(|record| record.vehicle_dry_mass_kg);
    let initial_fuels = records.iter().map(|record| record.initial_fuel_kg);
    let domain = RouteCapabilityDomainV1 {
        route_angle_deg: exact_range(angles)?,
        route_radius_m: exact_range(radii)?,
        horizontal_signs: records
            .iter()
            .map(|record| record.horizontal_sign)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        waypoint_counts: records
            .iter()
            .map(|record| record.stratum.waypoint_count)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        vehicle_dry_mass_kg: exact_range(dry_masses)?,
        initial_fuel_kg: exact_range(initial_fuels)?,
        terrain_digests: records
            .iter()
            .map(|record| record.stratum.terrain_geometry_digest.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    domain.validate()?;
    let training_evidence_digests = records
        .iter()
        .map(|record| record.evidence_physical_digest.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    RouteCapabilityArtifactV1::new(
        PROGRESS_INTERVAL_ENVELOPE_ALTERNATIVE,
        serde_json::to_value(model)
            .map_err(|error| format!("failed to serialize progress-interval model: {error}"))?,
        configuration.capability_configuration()?,
        domain,
        training_evidence_digests,
    )
}

fn fit_stratum(
    key: &ProgressIntervalStratumKeyV1,
    records: &[&ProgressIntervalTrainingRecordV1],
    configuration: &ProgressIntervalEnvelopeConfigV1,
) -> Result<ProgressIntervalStratumModelV1, String> {
    let mut training_points = records
        .iter()
        .map(|record| HullPointV1 {
            route_angle_deg: canonical_zero(record.training_point.route_angle_deg),
            route_radius_m: canonical_zero(record.training_point.route_radius_m),
        })
        .collect::<Vec<_>>();
    training_points.sort_by(point_order);
    training_points.dedup_by(|left, right| point_order(left, right) == Ordering::Equal);
    let hull = convex_hull(&training_points, configuration.hull_tolerance)?;
    let expected_phases = expected_continuous_phases(key.waypoint_count);
    let mut phases = Vec::with_capacity(expected_phases.len());
    for phase in expected_phases {
        let mut bins = Vec::with_capacity(PROGRESS_INTERVAL_BIN_COUNT);
        for bin_index in 0..PROGRESS_INTERVAL_BIN_COUNT {
            let mut observations = Vec::new();
            let mut digests = BTreeSet::new();
            for record in records {
                if let Some(training_phase) = record
                    .continuous_phases
                    .iter()
                    .find(|candidate| candidate.phase == phase)
                {
                    for observation in &training_phase.observations {
                        if progress_bin_index(observation.normalized_phase_progress)? == bin_index {
                            observations.push(observation);
                            digests.insert(record.input_digest.clone());
                        }
                    }
                }
            }
            bins.push(fit_envelope_cell(
                format!("{}:{phase:?}:bin_{bin_index}", canonical_digest(key)?),
                bin_index,
                digests,
                &observations,
                configuration.minimum_distinct_input_digests,
            )?);
        }
        let mut terminal_observations = Vec::new();
        let mut terminal_digests = BTreeSet::new();
        for record in records {
            if let Some(terminal) = record
                .continuous_phases
                .iter()
                .find(|candidate| candidate.phase == phase)
                .and_then(|candidate| candidate.terminal.as_ref())
            {
                terminal_observations.push(terminal);
                terminal_digests.insert(record.input_digest.clone());
            }
        }
        phases.push(ProgressIntervalPhaseModelV1 {
            phase: phase.clone(),
            bins,
            terminal: fit_boundary_cell(
                format!("{}:{phase:?}:terminal", canonical_digest(key)?),
                terminal_digests,
                &terminal_observations,
                configuration.minimum_distinct_input_digests,
            )?,
        });
    }
    let mut handoffs = Vec::with_capacity(key.waypoint_count);
    for waypoint_index in 0..key.waypoint_count {
        let mut observations = Vec::new();
        let mut digests = BTreeSet::new();
        for record in records {
            if let Some(handoff) = record
                .handoffs
                .iter()
                .find(|handoff| handoff.waypoint_index == waypoint_index)
            {
                observations.push(&handoff.terminal);
                digests.insert(record.input_digest.clone());
            }
        }
        handoffs.push(ProgressIntervalHandoffModelV1 {
            waypoint_index,
            terminal: fit_boundary_cell(
                format!("{}:handoff_{waypoint_index}", canonical_digest(key)?),
                digests,
                &observations,
                configuration.minimum_distinct_input_digests,
            )?,
        });
    }
    Ok(ProgressIntervalStratumModelV1 {
        key: key.clone(),
        training_points,
        convex_hull: hull,
        phases,
        handoffs,
    })
}

fn fit_envelope_cell(
    identity: String,
    bin_index: usize,
    digests: BTreeSet<String>,
    observations: &[&ProgressIntervalStateObservationV1],
    minimum_coverage: usize,
) -> Result<ProgressIntervalEnvelopeCellV1, String> {
    let boundary = fit_boundary_cell(identity, digests, observations, minimum_coverage)?;
    Ok(ProgressIntervalEnvelopeCellV1 {
        bin_index,
        distinct_input_digests: boundary.distinct_input_digests,
        envelope: boundary.envelope,
        issue_reason: boundary.issue_reason,
    })
}

fn fit_boundary_cell(
    identity: String,
    digests: BTreeSet<String>,
    observations: &[&ProgressIntervalStateObservationV1],
    minimum_coverage: usize,
) -> Result<ProgressIntervalBoundaryCellV1, String> {
    let distinct_input_digests = digests.into_iter().collect::<Vec<_>>();
    if observations.is_empty() {
        return Ok(ProgressIntervalBoundaryCellV1 {
            distinct_input_digests,
            envelope: None,
            issue_reason: Some("unknown/coverage/missing_phase_or_bin".to_owned()),
        });
    }
    let envelope = match fit_state_set(&identity, observations) {
        Ok(envelope) => Some(envelope),
        Err(reason) if reason == REASON_AMBIGUOUS_ATTITUDE_ARC => {
            return Ok(ProgressIntervalBoundaryCellV1 {
                distinct_input_digests,
                envelope: None,
                issue_reason: Some(reason),
            });
        }
        Err(reason) => return Err(reason),
    };
    let issue_reason = (distinct_input_digests.len() < minimum_coverage)
        .then(|| "unknown/coverage/undercovered_phase_bin".to_owned());
    Ok(ProgressIntervalBoundaryCellV1 {
        distinct_input_digests,
        envelope,
        issue_reason,
    })
}

fn fit_state_set(
    identity: &str,
    observations: &[&ProgressIntervalStateObservationV1],
) -> Result<PhaseStateSetV1, String> {
    let interval_for = |selector: fn(&ProgressIntervalStateObservationV1) -> f64| {
        exact_observed_outward_range(observations.iter().map(|observation| selector(observation)))
    };
    let attitudes = observations
        .iter()
        .map(|observation| observation.attitude_rad)
        .collect::<Vec<_>>();
    PhaseStateSetV1::new(
        identity,
        interval_for(|observation| observation.progress_m)?,
        interval_for(|observation| observation.along_track_error_m)?,
        interval_for(|observation| observation.cross_track_error_m)?,
        interval_for(|observation| observation.along_track_velocity_mps)?,
        interval_for(|observation| observation.cross_track_velocity_mps)?,
        smallest_circular_interval(&attitudes)?,
        interval_for(|observation| observation.angular_rate_radps)?,
        interval_for(|observation| observation.mass_kg)?,
        interval_for(|observation| observation.fuel_kg)?,
        None,
    )
}

fn exact_observed_outward_range(
    values: impl Iterator<Item = f64>,
) -> Result<FiniteIntervalV1, String> {
    let mut values = values.peekable();
    let first = values
        .peek()
        .copied()
        .ok_or_else(|| "cannot fit an empty scalar interval".to_owned())?;
    let (minimum, maximum) = values.fold((first, first), |(minimum, maximum), value| {
        (minimum.min(value), maximum.max(value))
    });
    outward_interval(minimum, maximum)
}

fn exact_range(values: impl Iterator<Item = f64>) -> Result<FiniteIntervalV1, String> {
    let mut values = values.peekable();
    let first = values
        .peek()
        .copied()
        .ok_or_else(|| "cannot derive an empty physical domain".to_owned())?;
    let (minimum, maximum) = values.fold((first, first), |(minimum, maximum), value| {
        (minimum.min(value), maximum.max(value))
    });
    FiniteIntervalV1::new(minimum, maximum)
}

fn model_from_artifact(
    artifact: &RouteCapabilityArtifactV1,
) -> Result<ProgressIntervalEnvelopeModelV1, String> {
    artifact.validate()?;
    if artifact.status != crate::CapabilityArtifactStatus::Complete
        || artifact.alternative != PROGRESS_INTERVAL_ENVELOPE_ALTERNATIVE
    {
        return Err("capability artifact is not a complete progress-interval model".to_owned());
    }
    let model: ProgressIntervalEnvelopeModelV1 =
        serde_json::from_value(artifact.model_payload.clone())
            .map_err(|error| format!("failed to decode progress-interval model: {error}"))?;
    model.validate()?;
    if artifact.configuration != model.configuration.capability_configuration()? {
        return Err("progress-interval artifact configuration does not match model".to_owned());
    }
    Ok(model)
}

/// Evaluate the fixed empirical envelope without reading observations or
/// development outcomes.
pub fn predict_progress_interval_envelope(
    input: &RouteCapabilityInputV1,
    artifact: &RouteCapabilityArtifactV1,
) -> Result<crate::RouteExecutionPredictionV1, String> {
    input.validate()?;
    let model = model_from_artifact(artifact)?;
    if input.physical.topology == RouteTopology::Direct {
        return crate::RouteExecutionPredictionV1::unknown_scope_direct_route(input, artifact);
    }
    let geometry_tolerance_m = model.configuration.geometry_tolerance_m;
    let initial = initial_state_set(input, geometry_tolerance_m)?;
    if !artifact.declared_domain.contains(input)? {
        return crate::RouteExecutionPredictionV1::complete(
            input,
            artifact,
            vec![crate::RouteCapabilityPhaseDecisionV1::unknown(
                crate::RouteCapabilityPhaseV1::InitialState,
                "unknown/domain/outside_declared_domain",
            )?],
        );
    }
    let key = ProgressIntervalStratumKeyV1::from_input(input)?;
    let Some(stratum) = model.strata.iter().find(|stratum| stratum.key == key) else {
        return crate::RouteExecutionPredictionV1::complete(
            input,
            artifact,
            vec![crate::RouteCapabilityPhaseDecisionV1::unknown(
                crate::RouteCapabilityPhaseV1::InitialState,
                "unknown/domain/missing_physical_stratum",
            )?],
        );
    };
    let query = HullPointV1 {
        route_angle_deg: input.physical.route_angle_deg,
        route_radius_m: input.physical.route_radius_m,
    };
    if classify_hull_point(
        &stratum.convex_hull,
        query,
        model.configuration.hull_tolerance,
    )? == HullContainmentV1::Outside
    {
        return crate::RouteExecutionPredictionV1::complete(
            input,
            artifact,
            vec![crate::RouteCapabilityPhaseDecisionV1::unknown(
                crate::RouteCapabilityPhaseV1::InitialState,
                "unknown/coverage/outside_same_stratum_convex_hull",
            )?],
        );
    }

    let mut phases = Vec::new();
    let pad = phase_model(stratum, &ProgressIntervalContinuousPhaseV1::PadDeparture)?;
    let initial_containment = first_bin_containment("initial_to_pad_departure", &initial, pad)?;
    phases.push(
        crate::RouteCapabilityPhaseDecisionV1::supported(
            crate::RouteCapabilityPhaseV1::InitialState,
            Some(initial),
        )?
        .with_containment(initial_containment)?,
    );
    if phases[0]
        .containment
        .as_ref()
        .is_some_and(|containment| containment.decision != crate::CapabilityDecision::Supported)
    {
        return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
    }

    let acquisition = phase_model(stratum, &ProgressIntervalContinuousPhaseV1::Acquisition)?;
    if !append_continuous_phase(
        input,
        &mut phases,
        crate::RouteCapabilityPhaseV1::PadDeparture,
        pad,
        Some(acquisition),
        None,
        geometry_tolerance_m,
    )? {
        return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
    }
    let first_leg = phase_model(
        stratum,
        &ProgressIntervalContinuousPhaseV1::RouteLeg { leg_index: 0 },
    )?;
    if !append_continuous_phase(
        input,
        &mut phases,
        crate::RouteCapabilityPhaseV1::Acquisition,
        acquisition,
        Some(first_leg),
        None,
        geometry_tolerance_m,
    )? {
        return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
    }

    for waypoint_index in 0..key.waypoint_count {
        let leg = phase_model(
            stratum,
            &ProgressIntervalContinuousPhaseV1::RouteLeg {
                leg_index: waypoint_index,
            },
        )?;
        if !append_continuous_phase(
            input,
            &mut phases,
            crate::RouteCapabilityPhaseV1::RouteLeg {
                leg_index: waypoint_index,
            },
            leg,
            None,
            Some(waypoint_index),
            geometry_tolerance_m,
        )? {
            return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
        }
        let handoff = stratum
            .handoffs
            .iter()
            .find(|handoff| handoff.waypoint_index == waypoint_index)
            .ok_or_else(|| format!("model is missing handoff {waypoint_index}"))?;
        let Some(terminal) = handoff.terminal.envelope.clone() else {
            phases.push(crate::RouteCapabilityPhaseDecisionV1::unknown(
                crate::RouteCapabilityPhaseV1::Handoff { waypoint_index },
                handoff
                    .terminal
                    .issue_reason
                    .clone()
                    .unwrap_or_else(|| "unknown/coverage/missing_handoff_boundary".to_owned()),
            )?);
            return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
        };
        if let Some(reason) = coverage_reason(&handoff.terminal) {
            phases.push(crate::RouteCapabilityPhaseDecisionV1::unknown(
                crate::RouteCapabilityPhaseV1::Handoff { waypoint_index },
                reason,
            )?);
            return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
        }
        if !state_set_geometry_is_proved(
            input,
            &terminal,
            input.physical.safety_profile.full_envelope,
            geometry_tolerance_m,
        )? {
            phases.push(crate::RouteCapabilityPhaseDecisionV1::unknown(
                crate::RouteCapabilityPhaseV1::Handoff { waypoint_index },
                format!("unknown/numerical/handoff_{waypoint_index}_terrain_projection"),
            )?);
            return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
        }
        let mut decision = crate::RouteCapabilityPhaseDecisionV1::supported(
            crate::RouteCapabilityPhaseV1::Handoff { waypoint_index },
            Some(terminal.clone()),
        )?;
        if waypoint_index + 1 < key.waypoint_count {
            let next_leg = phase_model(
                stratum,
                &ProgressIntervalContinuousPhaseV1::RouteLeg {
                    leg_index: waypoint_index + 1,
                },
            )?;
            decision = decision.with_containment(first_bin_containment(
                &format!(
                    "handoff_{waypoint_index}_to_route_leg_{}",
                    waypoint_index + 1
                ),
                &terminal,
                next_leg,
            )?)?;
            let supported = decision.containment.as_ref().is_some_and(|containment| {
                containment.decision == crate::CapabilityDecision::Supported
            });
            phases.push(decision);
            if !supported {
                return crate::RouteExecutionPredictionV1::complete(input, artifact, phases);
            }
        } else {
            phases.push(decision);
        }
    }
    crate::RouteExecutionPredictionV1::complete(input, artifact, phases)
}

fn phase_model<'a>(
    stratum: &'a ProgressIntervalStratumModelV1,
    phase: &ProgressIntervalContinuousPhaseV1,
) -> Result<&'a ProgressIntervalPhaseModelV1, String> {
    stratum
        .phases
        .iter()
        .find(|candidate| &candidate.phase == phase)
        .ok_or_else(|| format!("model is missing phase {phase:?}"))
}

fn append_continuous_phase(
    input: &RouteCapabilityInputV1,
    phases: &mut Vec<crate::RouteCapabilityPhaseDecisionV1>,
    public_phase: crate::RouteCapabilityPhaseV1,
    model: &ProgressIntervalPhaseModelV1,
    downstream: Option<&ProgressIntervalPhaseModelV1>,
    waypoint_contract: Option<usize>,
    geometry_tolerance_m: f64,
) -> Result<bool, String> {
    if let Some(reason) = phase_coverage_reason(model) {
        phases.push(crate::RouteCapabilityPhaseDecisionV1::unknown(
            public_phase,
            reason,
        )?);
        return Ok(false);
    }
    if !phase_geometry_is_proved(input, &model.phase, model, geometry_tolerance_m)? {
        phases.push(crate::RouteCapabilityPhaseDecisionV1::unknown(
            public_phase,
            format!(
                "unknown/numerical/{}_terrain_projection",
                phase_reason_suffix(&model.phase)
            ),
        )?);
        return Ok(false);
    }
    let terminal = model
        .terminal
        .envelope
        .clone()
        .ok_or_else(|| "covered phase has no terminal envelope".to_owned())?;
    let containment = if let Some(waypoint_index) = waypoint_contract {
        waypoint_contract_containment(input, waypoint_index, &terminal, geometry_tolerance_m)?
    } else if let Some(downstream) = downstream {
        first_bin_containment(
            &format!(
                "{}_to_{}",
                phase_reason_suffix(&model.phase),
                phase_reason_suffix(&downstream.phase)
            ),
            &terminal,
            downstream,
        )?
    } else {
        return Err("continuous phase has no downstream containment".to_owned());
    };
    let supported = containment.decision == crate::CapabilityDecision::Supported;
    phases.push(
        crate::RouteCapabilityPhaseDecisionV1::supported(public_phase, Some(terminal))?
            .with_containment(containment)?,
    );
    Ok(supported)
}

fn phase_coverage_reason(model: &ProgressIntervalPhaseModelV1) -> Option<String> {
    model
        .bins
        .iter()
        .find_map(|cell| {
            if cell.distinct_input_digests.len() < PROGRESS_INTERVAL_MIN_DISTINCT_INPUT_DIGESTS
                || cell.envelope.is_none()
            {
                Some(
                    cell.issue_reason
                        .clone()
                        .unwrap_or_else(|| "unknown/coverage/undercovered_phase_bin".to_owned()),
                )
            } else {
                None
            }
        })
        .or_else(|| coverage_reason(&model.terminal))
}

fn coverage_reason(cell: &ProgressIntervalBoundaryCellV1) -> Option<String> {
    if cell.distinct_input_digests.len() < PROGRESS_INTERVAL_MIN_DISTINCT_INPUT_DIGESTS
        || cell.envelope.is_none()
    {
        Some(
            cell.issue_reason
                .clone()
                .unwrap_or_else(|| "unknown/coverage/undercovered_phase_boundary".to_owned()),
        )
    } else {
        None
    }
}

fn phase_reason_suffix(phase: &ProgressIntervalContinuousPhaseV1) -> String {
    match phase {
        ProgressIntervalContinuousPhaseV1::PadDeparture => "pad_departure".to_owned(),
        ProgressIntervalContinuousPhaseV1::Acquisition => "acquisition".to_owned(),
        ProgressIntervalContinuousPhaseV1::RouteLeg { leg_index } => {
            format!("route_leg_{leg_index}")
        }
    }
}

fn initial_state_set(
    input: &RouteCapabilityInputV1,
    geometry_tolerance_m: f64,
) -> Result<PhaseStateSetV1, String> {
    let coordinates = route_coordinates_for_state(
        input,
        input.physical.initial_state.position_m,
        input.physical.initial_state.velocity_mps,
        geometry_tolerance_m,
    )?;
    let singleton = FiniteIntervalV1::singleton;
    PhaseStateSetV1::new(
        "authoritative_initial_state",
        singleton(coordinates.progress_m)?,
        singleton(0.0)?,
        singleton(coordinates.cross_track_error_m)?,
        singleton(coordinates.along_track_velocity_mps)?,
        singleton(coordinates.cross_track_velocity_mps)?,
        CircularAttitudeIntervalV1::new(input.physical.initial_state.attitude_rad, 0.0)?,
        singleton(input.physical.initial_state.angular_rate_radps)?,
        singleton(input.physical.vehicle.dry_mass_kg + input.physical.vehicle.initial_fuel_kg)?,
        singleton(input.physical.vehicle.initial_fuel_kg)?,
        None,
    )
}

#[derive(Clone, Copy)]
struct RouteStateCoordinates {
    progress_m: f64,
    cross_track_error_m: f64,
    along_track_velocity_mps: f64,
    cross_track_velocity_mps: f64,
}

fn normalized_centerline(input: &RouteCapabilityInputV1) -> Vec<pd_core::Vec2> {
    input
        .physical
        .selected_centerline_m
        .iter()
        .map(|point| {
            pd_core::Vec2::new(
                f64::from(input.physical.horizontal_sign)
                    * (point.x - input.physical.source_pad.center_x_m),
                point.y,
            )
        })
        .collect()
}

fn route_coordinates_for_state(
    input: &RouteCapabilityInputV1,
    world_position: pd_core::Vec2,
    world_velocity: pd_core::Vec2,
    geometry_tolerance_m: f64,
) -> Result<RouteStateCoordinates, String> {
    let centerline = normalized_centerline(input);
    let normalized_position = pd_core::Vec2::new(
        f64::from(input.physical.horizontal_sign)
            * (world_position.x - input.physical.source_pad.center_x_m),
        world_position.y,
    );
    let normalized_velocity = pd_core::Vec2::new(
        world_velocity.x * f64::from(input.physical.horizontal_sign),
        world_velocity.y,
    );
    let first_x = centerline
        .first()
        .ok_or_else(|| "selected centerline has no points".to_owned())?
        .x;
    let last_x = centerline
        .last()
        .ok_or_else(|| "selected centerline has no points".to_owned())?
        .x;
    let segment_index = if normalized_position.x < first_x {
        0
    } else if normalized_position.x > last_x {
        centerline.len().saturating_sub(2)
    } else {
        centerline
            .windows(2)
            .enumerate()
            .find_map(|(index, pair)| {
                let is_last = index + 2 == centerline.len();
                (normalized_position.x >= pair[0].x - geometry_tolerance_m
                    && (normalized_position.x < pair[1].x - geometry_tolerance_m || is_last))
                    .then_some(index)
            })
            .ok_or_else(|| "state is outside the monotone selected centerline".to_owned())?
    };
    let mut cumulative = 0.0;
    for pair in centerline.windows(2).take(segment_index) {
        let length = (pair[1] - pair[0]).length();
        if !length.is_finite() || length <= 0.0 {
            return Err("selected centerline contains a degenerate segment".to_owned());
        }
        cumulative += length;
    }
    let start = centerline[segment_index];
    let delta = centerline[segment_index + 1] - start;
    let length = delta.length();
    if !length.is_finite() || length <= 0.0 {
        return Err("selected centerline contains a degenerate segment".to_owned());
    }
    let tangent = delta * (1.0 / length);
    let normal = pd_core::Vec2::new(-tangent.y, tangent.x);
    let relative = normalized_position - start;
    Ok(RouteStateCoordinates {
        progress_m: cumulative + dot_vec(relative, tangent),
        cross_track_error_m: dot_vec(relative, normal),
        along_track_velocity_mps: dot_vec(normalized_velocity, tangent),
        cross_track_velocity_mps: dot_vec(normalized_velocity, normal),
    })
}

fn dot_vec(left: pd_core::Vec2, right: pd_core::Vec2) -> f64 {
    left.x.mul_add(right.x, left.y * right.y)
}

fn first_bin_containment(
    identity: &str,
    terminal: &PhaseStateSetV1,
    downstream: &ProgressIntervalPhaseModelV1,
) -> Result<crate::StateSetContainmentDecisionV1, String> {
    let first = &downstream.bins[0];
    if first.distinct_input_digests.len() < PROGRESS_INTERVAL_MIN_DISTINCT_INPUT_DIGESTS
        || first.envelope.is_none()
    {
        return crate::StateSetContainmentDecisionV1::new(
            identity,
            crate::CapabilityDecision::Unknown,
            Some(
                first
                    .issue_reason
                    .clone()
                    .unwrap_or_else(|| "unknown/coverage/undercovered_phase_bin".to_owned()),
            ),
        );
    }
    state_set_containment(identity, terminal, first.envelope.as_ref().unwrap())
}

fn state_set_containment(
    identity: &str,
    inner: &PhaseStateSetV1,
    outer: &PhaseStateSetV1,
) -> Result<crate::StateSetContainmentDecisionV1, String> {
    inner.validate()?;
    outer.validate()?;
    let scalar_pairs = [
        (inner.progress_m, outer.progress_m),
        (inner.along_track_error_m, outer.along_track_error_m),
        (inner.cross_track_error_m, outer.cross_track_error_m),
        (
            inner.along_track_velocity_mps,
            outer.along_track_velocity_mps,
        ),
        (
            inner.cross_track_velocity_mps,
            outer.cross_track_velocity_mps,
        ),
        (inner.angular_rate_radps, outer.angular_rate_radps),
        (inner.mass_kg, outer.mass_kg),
        (inner.fuel_kg, outer.fuel_kg),
    ];
    let scalar_supported = scalar_pairs
        .iter()
        .all(|(inner, outer)| interval_subset(*inner, *outer));
    let scalar_disjoint = scalar_pairs
        .iter()
        .any(|(inner, outer)| interval_disjoint(*inner, *outer));
    let center_distance =
        circular_distance_public(inner.attitude.center_rad, outer.attitude.center_rad);
    let attitude_supported = outer.attitude.half_width_rad >= PI
        || center_distance + inner.attitude.half_width_rad <= outer.attitude.half_width_rad;
    let attitude_disjoint = outer.attitude.half_width_rad < PI
        && center_distance > inner.attitude.half_width_rad + outer.attitude.half_width_rad;
    let (decision, reason) = if scalar_supported && attitude_supported {
        (crate::CapabilityDecision::Supported, None)
    } else if scalar_disjoint || attitude_disjoint {
        (
            crate::CapabilityDecision::Unsupported,
            Some(format!("unsupported/containment/{identity}_disjoint")),
        )
    } else {
        (
            crate::CapabilityDecision::Unknown,
            Some(format!("unknown/numerical/{identity}_partial_overlap")),
        )
    };
    crate::StateSetContainmentDecisionV1::new(identity, decision, reason)
}

fn interval_subset(inner: FiniteIntervalV1, outer: FiniteIntervalV1) -> bool {
    outer.lower <= inner.lower && inner.upper <= outer.upper
}

fn interval_disjoint(left: FiniteIntervalV1, right: FiniteIntervalV1) -> bool {
    left.upper < right.lower || right.upper < left.lower
}

fn circular_distance_public(left: f64, right: f64) -> f64 {
    let mut delta = (left - right).rem_euclid(TAU);
    if delta > PI {
        delta = TAU - delta;
    }
    delta
}

fn phase_geometry_is_proved(
    input: &RouteCapabilityInputV1,
    phase: &ProgressIntervalContinuousPhaseV1,
    model: &ProgressIntervalPhaseModelV1,
    geometry_tolerance_m: f64,
) -> Result<bool, String> {
    let physical_envelope = if matches!(phase, ProgressIntervalContinuousPhaseV1::PadDeparture) {
        input.physical.safety_profile.contact_envelope
    } else {
        input.physical.safety_profile.full_envelope
    };
    for envelope in model
        .bins
        .iter()
        .map(|cell| cell.envelope.as_ref())
        .chain(std::iter::once(model.terminal.envelope.as_ref()))
    {
        let Some(envelope) = envelope else {
            return Ok(false);
        };
        if !state_set_geometry_is_proved(input, envelope, physical_envelope, geometry_tolerance_m)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn state_set_geometry_is_proved(
    input: &RouteCapabilityInputV1,
    envelope: &PhaseStateSetV1,
    physical_envelope: pd_core::CorridorEnvelope,
    geometry_tolerance_m: f64,
) -> Result<bool, String> {
    let base_points = route_progress_points(
        input,
        envelope.progress_m.lower,
        envelope.progress_m.upper,
        geometry_tolerance_m,
    )?;
    let expansion = max_abs(envelope.cross_track_error_m) + max_abs(envelope.along_track_error_m);
    let base_min_x = base_points
        .iter()
        .map(|point| point.x)
        .min_by(f64::total_cmp)
        .ok_or_else(|| "terrain projection has no base points".to_owned())?;
    let base_max_x = base_points
        .iter()
        .map(|point| point.x)
        .max_by(f64::total_cmp)
        .ok_or_else(|| "terrain projection has no base points".to_owned())?;
    let base_min_y = base_points
        .iter()
        .map(|point| point.y)
        .min_by(f64::total_cmp)
        .ok_or_else(|| "terrain projection has no base points".to_owned())?;
    let base_max_y = base_points
        .iter()
        .map(|point| point.y)
        .max_by(f64::total_cmp)
        .ok_or_else(|| "terrain projection has no base points".to_owned())?;
    let x = outward_interval(base_min_x - expansion, base_max_x + expansion)?;
    let y = outward_interval(base_min_y - expansion, base_max_y + expansion)?;
    let center = pd_core::Vec2::new((x.lower + x.upper) * 0.5, (y.lower + y.upper) * 0.5);
    let projected_envelope = pd_core::CorridorEnvelope::new(
        next_up((x.upper - x.lower) * 0.5 + physical_envelope.horizontal_extent_m)?,
        next_up((y.upper - y.lower) * 0.5 + physical_envelope.vertical_extent_m)?,
    );
    Ok(input
        .physical
        .terrain
        .exact_point_clearance(center, projected_envelope)
        .is_ok_and(|clearance| clearance.clear))
}

fn route_progress_points(
    input: &RouteCapabilityInputV1,
    lower: f64,
    upper: f64,
    geometry_tolerance_m: f64,
) -> Result<Vec<pd_core::Vec2>, String> {
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err("route progress interval is invalid".to_owned());
    }
    let mut points = Vec::new();
    for slice in route_frame_slices(
        input,
        FiniteIntervalV1::new(lower, upper)?,
        geometry_tolerance_m,
    )? {
        for progress in [slice.progress_lower_m, slice.progress_upper_m] {
            let normalized =
                slice.start_position_m + slice.tangent * (progress - slice.start_progress_m);
            points.push(normalized_to_world(input, normalized));
        }
    }
    Ok(points)
}

fn max_abs(interval: FiniteIntervalV1) -> f64 {
    interval.lower.abs().max(interval.upper.abs())
}

fn waypoint_contract_containment(
    input: &RouteCapabilityInputV1,
    waypoint_index: usize,
    terminal: &PhaseStateSetV1,
    geometry_tolerance_m: f64,
) -> Result<crate::StateSetContainmentDecisionV1, String> {
    let waypoint = input
        .physical
        .waypoints
        .get(waypoint_index)
        .ok_or_else(|| format!("waypoint {waypoint_index} is missing"))?;
    let projected =
        project_route_state_to_handoff(input, waypoint_index, terminal, geometry_tolerance_m)?;
    let plane = projected.plane_progress_m;
    let cross = projected.cross_track_m;
    let maximum_distance = max_abs(plane).hypot(max_abs(cross));
    let minimum_distance = interval_min_abs(plane).hypot(interval_min_abs(cross));
    let spatial_supported = maximum_distance <= waypoint.capture_radius_m
        || (max_abs(cross) <= waypoint.max_cross_track_m
            && plane.lower >= -waypoint.capture_radius_m);
    let spatial_unsupported = minimum_distance > waypoint.capture_radius_m
        && (interval_min_abs(cross) > waypoint.max_cross_track_m
            || plane.upper < -waypoint.capture_radius_m);

    let along = projected.outbound_progress_mps;
    let cross_velocity = projected.outbound_cross_speed_mps;
    let maximum_cross_speed = max_abs(cross_velocity);
    let minimum_speed = interval_min_abs(along).hypot(interval_min_abs(cross_velocity));
    let maximum_speed = max_abs(along).hypot(maximum_cross_speed);
    let heading_supported = along.lower > 0.0
        && maximum_cross_speed.atan2(along.lower) <= waypoint.max_outbound_heading_error_rad;
    let heading_unsupported =
        along.upper <= 0.0 && waypoint.max_outbound_heading_error_rad < PI * 0.5;
    let progress_supported = along.lower >= waypoint.min_outbound_progress_mps;
    let progress_unsupported = along.upper < waypoint.min_outbound_progress_mps;
    let cross_speed_supported = waypoint
        .max_outbound_cross_speed_mps
        .is_none_or(|limit| maximum_cross_speed <= limit);
    let cross_speed_unsupported = waypoint
        .max_outbound_cross_speed_mps
        .is_some_and(|limit| interval_min_abs(cross_velocity) > limit);
    let speed_supported =
        minimum_speed >= waypoint.min_speed_mps && maximum_speed <= waypoint.max_speed_mps;
    let speed_unsupported =
        maximum_speed < waypoint.min_speed_mps || minimum_speed > waypoint.max_speed_mps;
    let vertical = projected.vertical_speed_mps;
    let vertical_supported = waypoint
        .min_vertical_speed_mps
        .is_none_or(|minimum| vertical.lower >= minimum)
        && waypoint
            .max_vertical_speed_mps
            .is_none_or(|maximum| vertical.upper <= maximum);
    let vertical_unsupported = waypoint
        .min_vertical_speed_mps
        .is_some_and(|minimum| vertical.upper < minimum)
        || waypoint
            .max_vertical_speed_mps
            .is_some_and(|maximum| vertical.lower > maximum);

    let unsupported = spatial_unsupported
        || heading_unsupported
        || progress_unsupported
        || cross_speed_unsupported
        || speed_unsupported
        || vertical_unsupported;
    let supported = spatial_supported
        && heading_supported
        && progress_supported
        && cross_speed_supported
        && speed_supported
        && vertical_supported;
    let identity = format!("route_leg_{waypoint_index}_to_handoff_{waypoint_index}");
    let (decision, reason) = if supported {
        (crate::CapabilityDecision::Supported, None)
    } else if unsupported {
        (
            crate::CapabilityDecision::Unsupported,
            Some(format!("unsupported/containment/{identity}_disjoint")),
        )
    } else {
        (
            crate::CapabilityDecision::Unknown,
            Some(format!("unknown/numerical/{identity}_partial_overlap")),
        )
    };
    crate::StateSetContainmentDecisionV1::new(identity, decision, reason)
}

#[derive(Clone, Copy)]
struct HandoffCoordinateIntervals {
    plane_progress_m: FiniteIntervalV1,
    cross_track_m: FiniteIntervalV1,
    outbound_progress_mps: FiniteIntervalV1,
    outbound_cross_speed_mps: FiniteIntervalV1,
    vertical_speed_mps: FiniteIntervalV1,
}

#[derive(Clone, Copy)]
struct RouteFrameSlice {
    start_progress_m: f64,
    start_position_m: pd_core::Vec2,
    tangent: pd_core::Vec2,
    progress_lower_m: f64,
    progress_upper_m: f64,
}

fn project_route_state_to_handoff(
    input: &RouteCapabilityInputV1,
    waypoint_index: usize,
    state: &PhaseStateSetV1,
    geometry_tolerance_m: f64,
) -> Result<HandoffCoordinateIntervals, String> {
    state.validate()?;
    let waypoint = input
        .physical
        .waypoints
        .get(waypoint_index)
        .ok_or_else(|| format!("waypoint {waypoint_index} is missing"))?;
    let waypoint_position = world_to_normalized(input, waypoint.position_m);
    let inbound_anchor = if waypoint_index == 0 {
        world_to_normalized(
            input,
            pd_core::Vec2::new(
                input.physical.source_pad.center_x_m,
                input.physical.source_pad.surface_y_m,
            ),
        )
    } else {
        world_to_normalized(
            input,
            input.physical.waypoints[waypoint_index - 1].position_m,
        )
    };
    let inbound_tangent = unit_vector(waypoint_position - inbound_anchor, "inbound waypoint leg")?;
    let inbound_normal = pd_core::Vec2::new(-inbound_tangent.y, inbound_tangent.x);
    let handoff_tangent_world = waypoint_handoff_tangent(input, waypoint_index)?;
    let handoff_tangent = pd_core::Vec2::new(
        handoff_tangent_world.x * f64::from(input.physical.horizontal_sign),
        handoff_tangent_world.y,
    );
    let handoff_normal = pd_core::Vec2::new(-handoff_tangent.y, handoff_tangent.x);

    let mut plane = None;
    let mut cross = None;
    let mut outbound = None;
    let mut cross_speed = None;
    let mut vertical = None;
    for slice in route_frame_slices(input, state.progress_m, geometry_tolerance_m)? {
        let route_normal = pd_core::Vec2::new(-slice.tangent.y, slice.tangent.x);
        let progress_positions = [slice.progress_lower_m, slice.progress_upper_m].map(|progress| {
            slice.start_position_m + slice.tangent * (progress - slice.start_progress_m)
        });
        let base_plane = interval_from_values(
            progress_positions
                .map(|position| dot_vec(position - waypoint_position, inbound_tangent)),
        )?;
        let base_cross = interval_from_values(
            progress_positions
                .map(|position| dot_vec(position - waypoint_position, inbound_normal)),
        )?;
        let position_plane_error = linear_velocity_interval(
            state.along_track_error_m,
            state.cross_track_error_m,
            dot_vec(slice.tangent, inbound_tangent),
            dot_vec(route_normal, inbound_tangent),
        )?;
        let position_cross_error = linear_velocity_interval(
            state.along_track_error_m,
            state.cross_track_error_m,
            dot_vec(slice.tangent, inbound_normal),
            dot_vec(route_normal, inbound_normal),
        )?;
        union_interval(&mut plane, add_intervals(base_plane, position_plane_error)?);
        union_interval(&mut cross, add_intervals(base_cross, position_cross_error)?);
        union_interval(
            &mut outbound,
            linear_velocity_interval(
                state.along_track_velocity_mps,
                state.cross_track_velocity_mps,
                dot_vec(slice.tangent, handoff_tangent),
                dot_vec(route_normal, handoff_tangent),
            )?,
        );
        union_interval(
            &mut cross_speed,
            linear_velocity_interval(
                state.along_track_velocity_mps,
                state.cross_track_velocity_mps,
                dot_vec(slice.tangent, handoff_normal),
                dot_vec(route_normal, handoff_normal),
            )?,
        );
        union_interval(
            &mut vertical,
            linear_velocity_interval(
                state.along_track_velocity_mps,
                state.cross_track_velocity_mps,
                slice.tangent.y,
                route_normal.y,
            )?,
        );
    }
    Ok(HandoffCoordinateIntervals {
        plane_progress_m: required_interval(plane, "handoff plane progress")?,
        cross_track_m: required_interval(cross, "handoff cross track")?,
        outbound_progress_mps: required_interval(outbound, "handoff outbound progress")?,
        outbound_cross_speed_mps: required_interval(cross_speed, "handoff cross speed")?,
        vertical_speed_mps: required_interval(vertical, "handoff vertical speed")?,
    })
}

fn route_frame_slices(
    input: &RouteCapabilityInputV1,
    progress: FiniteIntervalV1,
    geometry_tolerance_m: f64,
) -> Result<Vec<RouteFrameSlice>, String> {
    let centerline = normalized_centerline(input);
    let mut cumulative = 0.0;
    let mut slices = Vec::new();
    for (index, pair) in centerline.windows(2).enumerate() {
        let delta = pair[1] - pair[0];
        let length = delta.length();
        if !length.is_finite() || length <= 0.0 {
            return Err("selected centerline contains a degenerate segment".to_owned());
        }
        let segment_lower = if index == 0 {
            f64::NEG_INFINITY
        } else {
            cumulative - geometry_tolerance_m
        };
        let segment_upper = if index + 2 == centerline.len() {
            f64::INFINITY
        } else {
            cumulative + length + geometry_tolerance_m
        };
        let lower = progress.lower.max(segment_lower);
        let upper = progress.upper.min(segment_upper);
        if lower <= upper {
            slices.push(RouteFrameSlice {
                start_progress_m: cumulative,
                start_position_m: pair[0],
                tangent: delta * (1.0 / length),
                progress_lower_m: lower,
                progress_upper_m: upper,
            });
        }
        cumulative += length;
    }
    if slices.is_empty() {
        return Err("route progress interval has no centerline frame".to_owned());
    }
    Ok(slices)
}

fn world_to_normalized(input: &RouteCapabilityInputV1, point: pd_core::Vec2) -> pd_core::Vec2 {
    pd_core::Vec2::new(
        f64::from(input.physical.horizontal_sign)
            * (point.x - input.physical.source_pad.center_x_m),
        point.y,
    )
}

fn normalized_to_world(input: &RouteCapabilityInputV1, point: pd_core::Vec2) -> pd_core::Vec2 {
    pd_core::Vec2::new(
        input.physical.source_pad.center_x_m + f64::from(input.physical.horizontal_sign) * point.x,
        point.y,
    )
}

fn unit_vector(vector: pd_core::Vec2, identity: &str) -> Result<pd_core::Vec2, String> {
    let length = vector.length();
    if !length.is_finite() || length <= 0.0 {
        return Err(format!("{identity} is degenerate"));
    }
    Ok(vector * (1.0 / length))
}

fn interval_from_values<const N: usize>(values: [f64; N]) -> Result<FiniteIntervalV1, String> {
    let lower = values
        .iter()
        .copied()
        .min_by(f64::total_cmp)
        .ok_or_else(|| "interval requires values".to_owned())?;
    let upper = values
        .iter()
        .copied()
        .max_by(f64::total_cmp)
        .ok_or_else(|| "interval requires values".to_owned())?;
    outward_interval(lower, upper)
}

fn add_intervals(
    left: FiniteIntervalV1,
    right: FiniteIntervalV1,
) -> Result<FiniteIntervalV1, String> {
    outward_interval(left.lower + right.lower, left.upper + right.upper)
}

fn union_interval(target: &mut Option<FiniteIntervalV1>, value: FiniteIntervalV1) {
    *target = Some(target.map_or(value, |current| FiniteIntervalV1 {
        lower: current.lower.min(value.lower),
        upper: current.upper.max(value.upper),
    }));
}

fn required_interval(
    interval: Option<FiniteIntervalV1>,
    identity: &str,
) -> Result<FiniteIntervalV1, String> {
    interval.ok_or_else(|| format!("{identity} projection is empty"))
}

fn waypoint_handoff_tangent(
    input: &RouteCapabilityInputV1,
    waypoint_index: usize,
) -> Result<pd_core::Vec2, String> {
    let waypoint = &input.physical.waypoints[waypoint_index];
    if let Some(tangent) = waypoint.handoff_tangent_unit {
        return Ok(tangent);
    }
    let next_target = input.physical.waypoints.get(waypoint_index + 1).map_or(
        pd_core::Vec2::new(
            input.physical.target_pad.center_x_m,
            input.physical.target_pad.surface_y_m,
        ),
        |next| next.position_m,
    );
    let delta = next_target - waypoint.position_m;
    let length = delta.length();
    if !length.is_finite() || length <= 0.0 {
        return Err(format!("waypoint {waypoint_index} has no outbound tangent"));
    }
    Ok(delta * (1.0 / length))
}

fn interval_min_abs(interval: FiniteIntervalV1) -> f64 {
    if interval.contains(0.0) {
        0.0
    } else {
        interval.lower.abs().min(interval.upper.abs())
    }
}

fn linear_velocity_interval(
    along: FiniteIntervalV1,
    cross: FiniteIntervalV1,
    along_factor: f64,
    cross_factor: f64,
) -> Result<FiniteIntervalV1, String> {
    add_intervals(
        scale_interval(along, along_factor)?,
        scale_interval(cross, cross_factor)?,
    )
}

fn scale_interval(interval: FiniteIntervalV1, factor: f64) -> Result<FiniteIntervalV1, String> {
    if !factor.is_finite() {
        return Err("interval scale factor must be finite".to_owned());
    }
    let (lower, upper) = if factor >= 0.0 {
        (interval.lower * factor, interval.upper * factor)
    } else {
        (interval.upper * factor, interval.lower * factor)
    };
    outward_interval(lower, upper)
}

pub const PROGRESS_INTERVAL_DEVELOPMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressIntervalDevelopmentCorpusV1 {
    Baseline,
    Diagnostic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalDevelopmentInputRowV1 {
    pub row_id: String,
    pub corpus: ProgressIntervalDevelopmentCorpusV1,
    pub resolved_input_digest: String,
    pub input: RouteCapabilityInputV1,
    pub evidence_physical_digest: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalDevelopmentInputManifestV1 {
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub rows: Vec<ProgressIntervalDevelopmentInputRowV1>,
    pub manifest_digest: String,
}

impl ProgressIntervalDevelopmentInputManifestV1 {
    fn new(
        source_d0_input_digest: String,
        rows: Vec<ProgressIntervalDevelopmentInputRowV1>,
    ) -> Result<Self, String> {
        let mut manifest = Self {
            schema_version: PROGRESS_INTERVAL_DEVELOPMENT_SCHEMA_VERSION,
            source_d0_input_digest,
            rows,
            manifest_digest: String::new(),
        };
        manifest.manifest_digest = manifest.compute_digest()?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROGRESS_INTERVAL_DEVELOPMENT_SCHEMA_VERSION {
            return Err(format!(
                "input manifest schema_version must equal {PROGRESS_INTERVAL_DEVELOPMENT_SCHEMA_VERSION}"
            ));
        }
        if self.source_d0_input_digest.trim().is_empty() || self.rows.is_empty() {
            return Err("input manifest source digest and rows must not be empty".to_owned());
        }
        let mut previous = None;
        for row in &self.rows {
            if row.row_id.trim().is_empty()
                || previous.is_some_and(|value: &str| value >= row.row_id.as_str())
                || row.resolved_input_digest.trim().is_empty()
                || row.evidence_physical_digest.trim().is_empty()
            {
                return Err(
                    "input manifest row IDs must be non-empty, unique, and sorted".to_owned(),
                );
            }
            row.input.validate()?;
            previous = Some(row.row_id.as_str());
        }
        if self.manifest_digest != self.compute_digest()? {
            return Err("input manifest digest mismatch".to_owned());
        }
        Ok(())
    }

    fn compute_digest(&self) -> Result<String, String> {
        let mut material = self.clone();
        material.manifest_digest.clear();
        canonical_digest(&material)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalOutcomeOverlayRowV1 {
    pub row_id: String,
    pub outcome: crate::DevelopmentScopedOutcomeV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalOutcomeOverlayV1 {
    pub schema_version: u32,
    pub rows: Vec<ProgressIntervalOutcomeOverlayRowV1>,
    pub overlay_digest: String,
}

impl ProgressIntervalOutcomeOverlayV1 {
    fn new(rows: Vec<ProgressIntervalOutcomeOverlayRowV1>) -> Result<Self, String> {
        let mut overlay = Self {
            schema_version: PROGRESS_INTERVAL_DEVELOPMENT_SCHEMA_VERSION,
            rows,
            overlay_digest: String::new(),
        };
        overlay.overlay_digest = overlay.compute_digest()?;
        overlay.validate()?;
        Ok(overlay)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROGRESS_INTERVAL_DEVELOPMENT_SCHEMA_VERSION
            || self.rows.is_empty()
        {
            return Err("outcome overlay schema or rows are invalid".to_owned());
        }
        let mut previous = None;
        for row in &self.rows {
            if row.row_id.trim().is_empty()
                || previous.is_some_and(|value: &str| value >= row.row_id.as_str())
            {
                return Err(
                    "outcome overlay row IDs must be non-empty, unique, and sorted".to_owned(),
                );
            }
            previous = Some(row.row_id.as_str());
        }
        if self.overlay_digest != self.compute_digest()? {
            return Err("outcome overlay digest mismatch".to_owned());
        }
        Ok(())
    }

    fn compute_digest(&self) -> Result<String, String> {
        let mut material = self.clone();
        material.overlay_digest.clear();
        canonical_digest(&material)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalOutOfFoldPredictionV1 {
    pub row_id: String,
    pub excluded_input_digest: String,
    pub fold_capability_artifact_digest: String,
    pub prediction: crate::RouteExecutionPredictionV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressIntervalDevelopmentSummaryV1 {
    pub schema_version: u32,
    pub source_d0_input_digest: String,
    pub source_d0_summary_digest: String,
    pub input_manifest_digest: String,
    pub outcome_overlay_digest: String,
    pub candidate_configuration_digest: String,
    pub unique_input_digest_count: usize,
    pub fold_count: usize,
    pub fold_digest: String,
    pub out_of_fold_prediction_digest: String,
    pub comparison_digest: String,
    pub gate_report_digest: String,
    pub final_capability_artifact_digest: String,
    pub baseline_count: usize,
    pub diagnostic_count: usize,
    pub gate_passed: bool,
    pub gate_failure_reasons: Vec<String>,
}

pub(crate) struct LoadedDevelopmentRow {
    pub(crate) row_id: String,
    pub(crate) corpus: ProgressIntervalDevelopmentCorpusV1,
    pub(crate) input: RouteCapabilityInputV1,
    pub(crate) training: ProgressIntervalTrainingRecordV1,
    pub(crate) resolved_input_digest: String,
    pub(crate) physics_hz: u32,
    pub(crate) manifest_path: PathBuf,
}

/// Run the complete D1b development protocol against an explicit, already
/// generated D0b evidence root.  Outcomes are not read until every out-of-fold
/// prediction has been sealed.
pub fn run_progress_interval_envelope_development_gate(
    evidence_dir: &Path,
    output_dir: &Path,
) -> anyhow::Result<ProgressIntervalDevelopmentSummaryV1> {
    // This committed manifest is input-only.  The D0 summary carries outcome
    // and pass/fail fields and therefore remains unopened until predictions
    // have been sealed below.
    let d0_input_manifest = crate::load_source_transition_development_manifest(
        &crate::repo_root().join("fixtures/manifests/source_transition_d0a_development.json"),
    )?;
    let source_d0_input_digest = d0_input_manifest.input_digest.clone();
    let source_d0_manifest_id = d0_input_manifest.manifest_id.clone();
    let expected_baseline_order = d0_input_manifest
        .baseline_cases
        .iter()
        .flat_map(crate::SourceTransitionDevelopmentCase::resolved_case_keys)
        .collect::<Vec<_>>();
    let expected_diagnostic_order = d0_input_manifest
        .diagnostic_cases
        .iter()
        .flat_map(crate::SourceTransitionDevelopmentCase::resolved_case_keys)
        .collect::<Vec<_>>();
    let expected_baseline_rows = expected_baseline_order
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_diagnostic_rows = expected_diagnostic_order
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create D1b output directory {}",
            output_dir.display()
        )
    })?;

    let mut loaded = Vec::new();
    load_development_corpus(
        evidence_dir,
        "baseline",
        ProgressIntervalDevelopmentCorpusV1::Baseline,
        &mut loaded,
    )?;
    load_development_corpus(
        evidence_dir,
        "diagnostic",
        ProgressIntervalDevelopmentCorpusV1::Diagnostic,
        &mut loaded,
    )?;
    loaded.sort_by(|left, right| left.row_id.cmp(&right.row_id));
    if loaded.len() != 60 {
        anyhow::bail!("D1b resolved {} rows instead of 60", loaded.len());
    }
    if loaded
        .windows(2)
        .any(|pair| pair[0].row_id == pair[1].row_id)
    {
        anyhow::bail!("D1b evidence root contains duplicate row IDs");
    }
    let baseline_count = loaded
        .iter()
        .filter(|row| row.corpus == ProgressIntervalDevelopmentCorpusV1::Baseline)
        .count();
    let diagnostic_count = loaded.len() - baseline_count;
    if baseline_count != 36 || diagnostic_count != 24 {
        anyhow::bail!(
            "D1b requires 36 baseline and 24 diagnostic rows, found {baseline_count} + {diagnostic_count}"
        );
    }
    let resolved_baseline_rows = loaded
        .iter()
        .filter(|row| row.corpus == ProgressIntervalDevelopmentCorpusV1::Baseline)
        .map(|row| row.row_id.clone())
        .collect::<BTreeSet<_>>();
    let resolved_diagnostic_rows = loaded
        .iter()
        .filter(|row| row.corpus == ProgressIntervalDevelopmentCorpusV1::Diagnostic)
        .map(|row| row.row_id.clone())
        .collect::<BTreeSet<_>>();
    if resolved_baseline_rows != expected_baseline_rows
        || resolved_diagnostic_rows != expected_diagnostic_rows
    {
        anyhow::bail!("D1b corpus directories do not match the committed input-only case keys");
    }
    let resolved_inputs_by_row = loaded
        .iter()
        .map(|row| (row.row_id.as_str(), row.resolved_input_digest.as_str()))
        .collect::<BTreeMap<_, _>>();
    let corpus_digest = |ordered_ids: &[String]| -> anyhow::Result<String> {
        let digests = ordered_ids
            .iter()
            .map(|row_id| {
                resolved_inputs_by_row
                    .get(row_id.as_str())
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("resolved input is missing for row {row_id}"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(format!(
            "fnv1a64:{}",
            crate::source_transition_canonical_digest(&digests)
        ))
    };
    let baseline_resolved_input_digest = corpus_digest(&expected_baseline_order)?;
    let diagnostic_resolved_input_digest = corpus_digest(&expected_diagnostic_order)?;
    if baseline_resolved_input_digest != d0_input_manifest.baseline_resolved_input_digest
        || diagnostic_resolved_input_digest != d0_input_manifest.diagnostic_resolved_input_digest
    {
        anyhow::bail!("D1b resolved-input content does not match the committed corpus digests");
    }

    let input_manifest = ProgressIntervalDevelopmentInputManifestV1::new(
        source_d0_input_digest.clone(),
        loaded
            .iter()
            .map(|row| ProgressIntervalDevelopmentInputRowV1 {
                row_id: row.row_id.clone(),
                corpus: row.corpus,
                resolved_input_digest: row.resolved_input_digest.clone(),
                input: row.input.clone(),
                evidence_physical_digest: row.training.evidence_physical_digest.clone(),
            })
            .collect(),
    )
    .map_err(anyhow::Error::msg)?;
    write_pretty_json(&output_dir.join("input_manifest.json"), &input_manifest)?;

    // The fixed candidate is serialized before any outcome file is opened.
    let configuration = ProgressIntervalEnvelopeConfigV1::default();
    let candidate_configuration_digest =
        canonical_digest(&configuration).map_err(anyhow::Error::msg)?;
    write_pretty_json(
        &output_dir.join("candidate_configuration.json"),
        &configuration,
    )?;

    let digests = loaded
        .iter()
        .map(|row| row.input.input_digest.clone())
        .collect::<Vec<_>>();
    let folds = crate::build_digest_grouped_folds(&digests).map_err(anyhow::Error::msg)?;
    write_pretty_json(&output_dir.join("folds.json"), &folds)?;
    let mut out_of_fold = Vec::with_capacity(loaded.len());
    for fold in &folds.folds {
        let fit_records = fold
            .fit_row_indices
            .iter()
            .map(|index| loaded[*index].training.clone())
            .collect::<Vec<_>>();
        let fold_artifact = fit_progress_interval_envelope(&fit_records, configuration.clone())
            .map_err(anyhow::Error::msg)?;
        for index in &fold.comparison_row_indices {
            let row = &loaded[*index];
            let prediction = predict_progress_interval_envelope(&row.input, &fold_artifact)
                .map_err(anyhow::Error::msg)?;
            out_of_fold.push(ProgressIntervalOutOfFoldPredictionV1 {
                row_id: row.row_id.clone(),
                excluded_input_digest: fold.excluded_input_digest.clone(),
                fold_capability_artifact_digest: fold_artifact.capability_artifact_digest.clone(),
                prediction,
            });
        }
    }
    out_of_fold.sort_by(|left, right| left.row_id.cmp(&right.row_id));
    if out_of_fold.len() != loaded.len() {
        anyhow::bail!("out-of-fold predictions do not cover all development rows");
    }
    let out_of_fold_prediction_digest =
        canonical_digest(&out_of_fold).map_err(anyhow::Error::msg)?;
    write_pretty_json(
        &output_dir.join("out_of_fold_predictions.json"),
        &out_of_fold,
    )?;

    // Outcome isolation boundary: only after prediction sealing do we open
    // the D0 gate summary and per-row run manifests.
    let d0_summary: crate::RouteExecutionDevelopmentGateSummary =
        read_json(&evidence_dir.join("summary.json"))?;
    d0_summary.validate().map_err(anyhow::Error::msg)?;
    if !d0_summary.overall_passed
        || d0_summary.case_filter.is_some()
        || d0_summary.manifest_id != source_d0_manifest_id
        || d0_summary.baseline_total_count != baseline_count
        || d0_summary.diagnostic_total_count != diagnostic_count
        || d0_summary.input_digest != source_d0_input_digest
        || !d0_summary.route_invalidations.is_empty()
        || !d0_summary.source_invalidations.is_empty()
    {
        anyhow::bail!("D1b requires a complete passing 36 + 24 D0b evidence root");
    }
    let outcome_overlay = ProgressIntervalOutcomeOverlayV1::new(
        loaded
            .iter()
            .map(scoped_outcome)
            .collect::<anyhow::Result<Vec<_>>>()?,
    )
    .map_err(anyhow::Error::msg)?;
    write_pretty_json(&output_dir.join("outcome_overlay.json"), &outcome_overlay)?;

    let predictions_by_row = out_of_fold
        .iter()
        .map(|prediction| (prediction.row_id.as_str(), &prediction.prediction))
        .collect::<BTreeMap<_, _>>();
    let outcomes_by_row = outcome_overlay
        .rows
        .iter()
        .map(|row| (row.row_id.as_str(), row.outcome))
        .collect::<BTreeMap<_, _>>();
    let mut comparison_rows = Vec::with_capacity(loaded.len());
    for row in &loaded {
        let prediction = predictions_by_row
            .get(row.row_id.as_str())
            .ok_or_else(|| anyhow::anyhow!("row {} has no sealed prediction", row.row_id))?;
        let outcome = outcomes_by_row
            .get(row.row_id.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("row {} has no outcome overlay", row.row_id))?;
        comparison_rows.push(
            crate::DevelopmentComparisonRowV1::new(
                row.row_id.clone(),
                row.input.input_digest.clone(),
                row.training.evidence_physical_digest.clone(),
                (*prediction).clone(),
                outcome,
                row.input.physical.topology,
                row.training.stratum.vehicle_physics_digest.clone(),
            )
            .map_err(anyhow::Error::msg)?,
        );
    }
    let comparison =
        crate::DevelopmentComparisonV1::new(comparison_rows).map_err(anyhow::Error::msg)?;
    let gate_report = comparison.evaluate_gate();
    let gate_report_digest = gate_report.digest().map_err(anyhow::Error::msg)?;
    write_pretty_json(&output_dir.join("comparison.json"), &comparison)?;
    write_pretty_json(&output_dir.join("gate_report.json"), &gate_report)?;

    // The one fixed candidate is selected before the overlay; only now is the
    // final all-development capability fitted from all neutral records.
    let final_capability = fit_progress_interval_envelope(
        &loaded
            .iter()
            .map(|row| row.training.clone())
            .collect::<Vec<_>>(),
        configuration,
    )
    .map_err(anyhow::Error::msg)?;
    write_pretty_json(&output_dir.join("final_capability.json"), &final_capability)?;

    let summary = ProgressIntervalDevelopmentSummaryV1 {
        schema_version: PROGRESS_INTERVAL_DEVELOPMENT_SCHEMA_VERSION,
        source_d0_input_digest,
        source_d0_summary_digest: d0_summary.summary_digest,
        input_manifest_digest: input_manifest.manifest_digest,
        outcome_overlay_digest: outcome_overlay.overlay_digest,
        candidate_configuration_digest,
        unique_input_digest_count: folds.unique_input_digest_count,
        fold_count: folds.folds.len(),
        fold_digest: folds.fold_digest,
        out_of_fold_prediction_digest,
        comparison_digest: comparison.comparison_digest,
        gate_report_digest,
        final_capability_artifact_digest: final_capability.capability_artifact_digest,
        baseline_count,
        diagnostic_count,
        gate_passed: gate_report.passed,
        gate_failure_reasons: gate_report.failure_reasons,
    };
    write_pretty_json(&output_dir.join("summary.json"), &summary)?;
    Ok(summary)
}

pub(crate) fn load_development_corpus(
    evidence_dir: &Path,
    directory_name: &str,
    corpus: ProgressIntervalDevelopmentCorpusV1,
    loaded: &mut Vec<LoadedDevelopmentRow>,
) -> anyhow::Result<()> {
    let corpus_dir = evidence_dir.join(directory_name);
    let mut directories = fs::read_dir(&corpus_dir)
        .with_context(|| format!("failed to read D0b corpus {}", corpus_dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    directories.retain(|path| path.is_dir());
    directories.sort();
    for directory in directories {
        let row_id = directory
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("D0b row path is not UTF-8"))?
            .to_owned();
        let scenario: pd_core::ScenarioSpec = read_json(&directory.join("scenario.json"))?;
        let route_plan: pd_core::RoutePlan = read_json(&directory.join("route_plan.json"))?;
        let evidence: RouteExecutionEvidence = read_json(&directory.join("route_execution.json"))?;
        if scenario.id != row_id
            || evidence.provenance.artifact_identity != row_id
            || evidence.source_transition.provenance.artifact_identity != row_id
        {
            anyhow::bail!("D1b row {row_id} has inconsistent scenario or artifact identity");
        }
        let request = pd_core::RoutePlanningRequest {
            world: scenario.world.clone(),
            vehicle: scenario.vehicle.clone(),
            initial_state: scenario.initial_state.clone(),
            source_pad_id: route_plan.route.source_pad_id.clone(),
            target_pad_id: route_plan.route.target_pad_id.clone(),
            policy: route_plan.policy.clone(),
        };
        let input = crate::build_route_capability_input(&request, &route_plan)
            .map_err(anyhow::Error::msg)?;
        let training = ProgressIntervalTrainingRecordV1::from_evidence(&input, &evidence)
            .map_err(|error| anyhow::anyhow!("D1b row {row_id} training failed: {error}"))?;
        loaded.push(LoadedDevelopmentRow {
            row_id,
            corpus,
            input,
            training,
            resolved_input_digest: evidence.provenance.resolved_input_digest.clone(),
            physics_hz: evidence.cadence.physics_hz,
            manifest_path: directory.join("manifest.json"),
        });
    }
    Ok(())
}

fn scoped_outcome(
    row: &LoadedDevelopmentRow,
) -> anyhow::Result<ProgressIntervalOutcomeOverlayRowV1> {
    let manifest: pd_core::RunManifest = read_json(&row.manifest_path)?;
    if manifest.scenario_id != row.row_id || manifest.physics_hz != row.physics_hz {
        anyhow::bail!(
            "row {} outcome manifest identity is inconsistent",
            row.row_id
        );
    }
    let outcome = match row.corpus {
        ProgressIntervalDevelopmentCorpusV1::Baseline => {
            if !matches!(
                (
                    &manifest.mission_outcome,
                    &manifest.end_reason,
                    &manifest.physical_outcome,
                ),
                (
                    pd_core::MissionOutcome::Success,
                    pd_core::EndReason::CheckpointSatisfied,
                    pd_core::PhysicalOutcome::Flying,
                )
            ) {
                anyhow::bail!("maintained row {} is not a scoped success", row.row_id);
            }
            crate::DevelopmentScopedOutcomeV1::MaintainedSuccess
        }
        ProgressIntervalDevelopmentCorpusV1::Diagnostic => match (
            &manifest.mission_outcome,
            &manifest.end_reason,
            &manifest.physical_outcome,
        ) {
            (
                pd_core::MissionOutcome::Success,
                pd_core::EndReason::CheckpointSatisfied,
                pd_core::PhysicalOutcome::Flying,
            ) => crate::DevelopmentScopedOutcomeV1::DiagnosticPass,
            (
                pd_core::MissionOutcome::FailedCheckpoint,
                pd_core::EndReason::CheckpointFailed,
                pd_core::PhysicalOutcome::Flying,
            ) => crate::DevelopmentScopedOutcomeV1::DiagnosticFailure,
            other => anyhow::bail!(
                "diagnostic row {} has unexpected scoped outcome {other:?}",
                row.row_id
            ),
        },
    };
    Ok(ProgressIntervalOutcomeOverlayRowV1 {
        row_id: row.row_id.clone(),
        outcome,
    })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn write_pretty_json<T>(path: &Path, value: &T) -> anyhow::Result<()>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    let serialized = serde_json::to_string_pretty(value)?;
    let decoded: T = serde_json::from_str(&serialized)?;
    if decoded != *value {
        anyhow::bail!("D1b artifact {} changes on JSON round-trip", path.display());
    }
    fs::write(path, serialized).with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bins_are_half_open_with_a_closed_final_bin() {
        assert_eq!(progress_bin_index(0.0).unwrap(), 0);
        for index in 1..PROGRESS_INTERVAL_BIN_COUNT {
            let boundary = index as f64 / PROGRESS_INTERVAL_BIN_COUNT as f64;
            assert_eq!(progress_bin_index(boundary).unwrap(), index);
            assert_eq!(
                progress_bin_index(next_down(boundary).unwrap()).unwrap(),
                index - 1
            );
        }
        assert_eq!(progress_bin_index(1.0).unwrap(), 31);
        assert!(progress_bin_index(-f64::EPSILON).is_err());
        assert!(progress_bin_index(1.0 + f64::EPSILON).is_err());
        assert!(progress_bin_index(f64::NAN).is_err());
    }

    #[test]
    fn scalar_bounds_expand_exactly_one_ulp() {
        let interval = outward_interval(1.0, 2.0).unwrap();
        assert_eq!(interval.lower.to_bits(), 1.0_f64.to_bits() - 1);
        assert_eq!(interval.upper.to_bits(), 2.0_f64.to_bits() + 1);
        let negative = outward_interval(-2.0, -1.0).unwrap();
        assert_eq!(negative.lower.to_bits(), (-2.0_f64).to_bits() + 1);
        assert_eq!(negative.upper.to_bits(), (-1.0_f64).to_bits() - 1);
        let zero = outward_interval(-0.0, 0.0).unwrap();
        assert_eq!(zero.lower, -f64::from_bits(1));
        assert_eq!(zero.upper, f64::from_bits(1));
        assert!(next_down(-f64::MAX).is_err());
        assert!(next_up(f64::MAX).is_err());
        assert_eq!(
            next_up(-f64::MAX).unwrap().to_bits(),
            (-f64::MAX).to_bits() - 1
        );
        assert_eq!(
            next_down(f64::MAX).unwrap().to_bits(),
            f64::MAX.to_bits() - 1
        );

        let persisted_observation: f64 = serde_json::from_str("117.08969037462965").unwrap();
        let persisted_interval =
            outward_interval(persisted_observation, persisted_observation).unwrap();
        let decoded_interval: FiniteIntervalV1 =
            serde_json::from_str(&serde_json::to_string(&persisted_interval).unwrap()).unwrap();
        assert_eq!(decoded_interval, persisted_interval);
    }

    #[test]
    fn circular_interval_wraps_and_rejects_ambiguity() {
        let singleton = smallest_circular_interval(&[0.25]).unwrap();
        assert_eq!(singleton.center_rad, 0.25);
        assert_eq!(singleton.half_width_rad, 0.0);

        let wrapped = smallest_circular_interval(&[PI - 0.1, -PI + 0.1]).unwrap();
        assert!(wrapped.center_rad.abs() > 3.0);
        assert!((wrapped.half_width_rad - 0.1).abs() < 1.0e-12);

        assert_eq!(
            smallest_circular_interval(&[0.0, PI]).unwrap_err(),
            REASON_AMBIGUOUS_ATTITUDE_ARC
        );
        assert_eq!(
            smallest_circular_interval(&[0.0, PI * 0.5, PI, -PI * 0.5]).unwrap_err(),
            REASON_AMBIGUOUS_ATTITUDE_ARC
        );
        let below_pi = next_down(PI).unwrap();
        let nearly_semicircular = smallest_circular_interval(&[0.0, below_pi]).unwrap();
        assert!(nearly_semicircular.half_width_rad * 2.0 < PI);
    }

    #[test]
    fn convex_hull_is_canonical_and_classifies_inclusive_queries() {
        let points = [
            HullPointV1 {
                route_angle_deg: 1.0,
                route_radius_m: 1.0,
            },
            HullPointV1 {
                route_angle_deg: 0.0,
                route_radius_m: 0.0,
            },
            HullPointV1 {
                route_angle_deg: 1.0,
                route_radius_m: 0.0,
            },
            HullPointV1 {
                route_angle_deg: 0.0,
                route_radius_m: 1.0,
            },
            HullPointV1 {
                route_angle_deg: 0.0,
                route_radius_m: 0.0,
            },
        ];
        let hull = convex_hull(&points, 0.0).unwrap();
        assert_eq!(hull.len(), 4);
        assert_eq!(hull[0], points[1]);
        assert_eq!(
            classify_hull_point(
                &hull,
                HullPointV1 {
                    route_angle_deg: 0.5,
                    route_radius_m: 0.5
                },
                1.0e-12,
            )
            .unwrap(),
            HullContainmentV1::Inside
        );
        assert_eq!(
            classify_hull_point(
                &hull,
                HullPointV1 {
                    route_angle_deg: 0.0,
                    route_radius_m: 0.5
                },
                1.0e-12,
            )
            .unwrap(),
            HullContainmentV1::Boundary
        );
        assert_eq!(
            classify_hull_point(
                &hull,
                HullPointV1 {
                    route_angle_deg: 2.0,
                    route_radius_m: 0.5
                },
                1.0e-12,
            )
            .unwrap(),
            HullContainmentV1::Outside
        );
    }

    #[test]
    fn collinear_hulls_are_segments() {
        let hull = convex_hull(
            &[
                HullPointV1 {
                    route_angle_deg: 0.0,
                    route_radius_m: 0.0,
                },
                HullPointV1 {
                    route_angle_deg: 1.0,
                    route_radius_m: 1.0,
                },
                HullPointV1 {
                    route_angle_deg: 2.0,
                    route_radius_m: 2.0,
                },
            ],
            0.0,
        )
        .unwrap();
        assert_eq!(hull.len(), 2);
        assert_eq!(
            classify_hull_point(
                &hull,
                HullPointV1 {
                    route_angle_deg: 1.5,
                    route_radius_m: 1.5
                },
                1.0e-12,
            )
            .unwrap(),
            HullContainmentV1::Boundary
        );
        assert_eq!(
            classify_hull_point(
                &hull,
                HullPointV1 {
                    route_angle_deg: 1.5,
                    route_radius_m: 1.6
                },
                1.0e-12,
            )
            .unwrap(),
            HullContainmentV1::Outside
        );
    }

    #[test]
    fn hull_canonicalizes_signed_zero_and_uses_distance_tolerance() {
        let hull = convex_hull(
            &[
                HullPointV1 {
                    route_angle_deg: -0.0,
                    route_radius_m: 0.0,
                },
                HullPointV1 {
                    route_angle_deg: 0.0,
                    route_radius_m: -0.0,
                },
                HullPointV1 {
                    route_angle_deg: 1_000.0,
                    route_radius_m: 0.0,
                },
            ],
            1.0e-9,
        )
        .unwrap();
        assert_eq!(hull.len(), 2);
        assert_eq!(hull[0].route_angle_deg.to_bits(), 0.0_f64.to_bits());
        assert_eq!(hull[0].route_radius_m.to_bits(), 0.0_f64.to_bits());
        assert_eq!(
            classify_hull_point(
                &hull,
                HullPointV1 {
                    route_angle_deg: 500.0,
                    route_radius_m: 0.5e-9,
                },
                1.0e-9,
            )
            .unwrap(),
            HullContainmentV1::Boundary
        );
        assert_eq!(
            classify_hull_point(
                &hull,
                HullPointV1 {
                    route_angle_deg: 500.0,
                    route_radius_m: 2.0e-9,
                },
                1.0e-9,
            )
            .unwrap(),
            HullContainmentV1::Outside
        );
    }

    #[test]
    fn hull_construction_does_not_use_query_tolerance() {
        let hull = convex_hull(
            &[
                HullPointV1 {
                    route_angle_deg: 0.0,
                    route_radius_m: 0.0,
                },
                HullPointV1 {
                    route_angle_deg: 500.0,
                    route_radius_m: -0.5e-9,
                },
                HullPointV1 {
                    route_angle_deg: 1_000.0,
                    route_radius_m: 0.0,
                },
            ],
            1.0e-9,
        )
        .unwrap();
        assert_eq!(hull.len(), 3);
    }

    #[test]
    fn fixed_configuration_is_valid_and_deterministic() {
        let config = ProgressIntervalEnvelopeConfigV1::default();
        config.validate().unwrap();
        let first = serde_json::to_vec(&config.capability_configuration().unwrap()).unwrap();
        let second = serde_json::to_vec(&config.capability_configuration().unwrap()).unwrap();
        assert_eq!(first, second);

        let mut invalid = config;
        invalid.bin_count = 31;
        assert!(invalid.validate().is_err());

        let mutations: [fn(&mut ProgressIntervalEnvelopeConfigV1); 5] = [
            |config: &mut ProgressIntervalEnvelopeConfigV1| config.fitted_padding = 1.0,
            |config: &mut ProgressIntervalEnvelopeConfigV1| config.hull_tolerance = -1.0,
            |config: &mut ProgressIntervalEnvelopeConfigV1| config.hull_tolerance = 1.0e-8,
            |config: &mut ProgressIntervalEnvelopeConfigV1| config.geometry_tolerance_m = f64::NAN,
            |config: &mut ProgressIntervalEnvelopeConfigV1| {
                config.bin_boundary_rule = "closed".to_owned()
            },
        ];
        for mutate in mutations {
            let mut invalid = ProgressIntervalEnvelopeConfigV1::default();
            mutate(&mut invalid);
            assert!(invalid.validate().is_err());
        }
        assert!(
            ProgressIntervalEnvelopeConfigV1::default()
                .capability_configuration()
                .unwrap()
                .padding
                .is_empty()
        );
    }

    #[test]
    fn interval_projection_arithmetic_rounds_outward() {
        let left = FiniteIntervalV1::singleton(0.1).unwrap();
        let right = FiniteIntervalV1::singleton(0.2).unwrap();
        let sum = add_intervals(left, right).unwrap();
        assert!(sum.lower < 0.1 + 0.2);
        assert!(sum.upper > 0.1 + 0.2);

        let projected = linear_velocity_interval(left, right, 0.3, -0.4).unwrap();
        let raw = 0.1 * 0.3 + 0.2 * -0.4;
        assert!(projected.lower < raw);
        assert!(projected.upper > raw);
        assert!(projected.lower < next_down(raw).unwrap());
        assert!(projected.upper > next_up(raw).unwrap());
    }
}
