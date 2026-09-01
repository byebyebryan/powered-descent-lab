//! Common, neutral D1a route-capability contracts.
//!
//! This module deliberately owns only the evaluator-side contract.  It does
//! not fit a model, load D0 evidence, run an executor, or inspect a scenario,
//! controller, or outcome.  The physical input is reconstructed from a
//! planning request and its selected persisted route; identities needed for
//! joins are retained in a separate provenance object.

use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::PI,
};

use pd_core::{
    CorridorEnvelope, LandingPadSpec, NormalizedRouteGeometry, RoutePlan, RoutePlanningPolicy,
    RoutePlanningRequest, RouteTopology, TerrainDefinition, TransferWaypointSpec, Vec2,
    VehicleInitialState, VehicleSpec, build_endpoint_profile, endpoint_shaped_centerline,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema version for the common route-capability input.
pub const ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION: u32 = 1;
const ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M: f64 = 1.0e-9;
/// Schema version for the conservative state-set representation.
pub const PHASE_STATE_SET_SCHEMA_VERSION: u32 = 1;
/// Schema version for capability artifacts.
pub const ROUTE_CAPABILITY_ARTIFACT_SCHEMA_VERSION: u32 = 1;
/// Schema version for route predictions.
pub const ROUTE_EXECUTION_PREDICTION_SCHEMA_VERSION: u32 = 1;
/// Schema version for development comparisons.
pub const DEVELOPMENT_COMPARISON_SCHEMA_VERSION: u32 = 1;

/// Stable reason namespaces locked by the D1 contract.
pub const REASON_INVALID_INPUT: &str = "invalid/input/";
pub const REASON_INVALID_ARTIFACT: &str = "invalid/artifact/";
pub const REASON_UNKNOWN_SCOPE: &str = "unknown/scope/";
pub const REASON_UNKNOWN_DOMAIN: &str = "unknown/domain/";
pub const REASON_UNKNOWN_COVERAGE: &str = "unknown/coverage/";
pub const REASON_UNKNOWN_NUMERICAL: &str = "unknown/numerical/";
pub const REASON_UNSUPPORTED_PHYSICS: &str = "unsupported/physics/";
pub const REASON_UNSUPPORTED_CONTAINMENT: &str = "unsupported/containment/";
/// Direct routes are valid inputs but outside this checkpoint's capability.
pub const REASON_UNKNOWN_SCOPE_DIRECT_ROUTE: &str = "unknown/scope/direct_route";

/// Named aliases keep digest fields readable while preserving the existing
/// repository convention of serializing digests as strings.
pub type RouteCapabilityInputDigest = String;
pub type RouteCapabilityModelConfigDigest = String;
pub type RouteCapabilityArtifactDigest = String;
pub type RouteExecutionPredictionDigest = String;
pub type DevelopmentComparisonDigest = String;
pub type ArtifactStatusV1 = CapabilityArtifactStatus;
pub type TriStateDecisionV1 = CapabilityDecision;
pub type ModelConfigDigestV1 = RouteCapabilityModelConfigDigest;
pub type CapabilityArtifactDigestV1 = RouteCapabilityArtifactDigest;
pub type PredictionDigestV1 = RouteExecutionPredictionDigest;
pub type ComparisonDigestV1 = DevelopmentComparisonDigest;

/// Status of a serialized evaluator artifact.  It is intentionally distinct
/// from [`CapabilityDecision`]: an invalid artifact has no prediction label.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityArtifactStatus {
    Complete,
    Invalid,
}

/// A complete artifact may carry exactly one of these tri-state decisions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityDecision {
    Supported,
    Unsupported,
    Unknown,
}

impl CapabilityDecision {
    pub const fn is_supported(self) -> bool {
        matches!(self, Self::Supported)
    }

    pub const fn is_decisive(self) -> bool {
        !self.is_supported()
    }
}

/// Validate a reason without allowing an alternative to invent a new
/// namespace.  The suffix is intentionally free-form so phase/index detail
/// can remain stable without another enum revision.
pub fn validate_reason_code(reason: &str) -> Result<(), String> {
    let namespace = [
        REASON_INVALID_INPUT,
        REASON_INVALID_ARTIFACT,
        REASON_UNKNOWN_SCOPE,
        REASON_UNKNOWN_DOMAIN,
        REASON_UNKNOWN_COVERAGE,
        REASON_UNKNOWN_NUMERICAL,
        REASON_UNSUPPORTED_PHYSICS,
        REASON_UNSUPPORTED_CONTAINMENT,
    ]
    .into_iter()
    .find(|prefix| reason.starts_with(prefix));
    if namespace.is_none()
        || reason.ends_with('/')
        || reason.trim().len() <= namespace.unwrap().len()
    {
        return Err(format!(
            "reason code has an unsupported namespace: {reason:?}"
        ));
    }
    if !reason
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'.'))
    {
        return Err(format!(
            "reason code contains unstable characters: {reason:?}"
        ));
    }
    Ok(())
}

/// Compute deterministic bytes for a serializable value.  Struct field order
/// is declaration order and maps in this module are BTreeMaps, giving a
/// stable representation without model-specific transforms.
pub fn canonical_serialized_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("canonical serialization failed: {error}"))
}

/// Stable FNV-1a digest used by the existing evaluator artifacts.
pub fn canonical_digest<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = canonical_serialized_bytes(value)?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:012x}"))
}

/// Source/target pad geometry with IDs intentionally removed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityPadGeometryV1 {
    pub center_x_m: f64,
    pub surface_y_m: f64,
    pub width_m: f64,
}

impl RouteCapabilityPadGeometryV1 {
    fn from_pad(pad: &LandingPadSpec) -> Self {
        Self {
            center_x_m: pad.center_x_m,
            surface_y_m: pad.surface_y_m,
            width_m: pad.width_m,
        }
    }

    fn validate(&self, name: &str) -> Result<(), String> {
        for (field, value) in [
            ("center_x_m", self.center_x_m),
            ("surface_y_m", self.surface_y_m),
            ("width_m", self.width_m),
        ] {
            if !value.is_finite() {
                return Err(format!("{name}.{field} must be finite"));
            }
        }
        if self.width_m <= 0.0 {
            return Err(format!("{name}.width_m must be positive"));
        }
        Ok(())
    }
}

/// Physical waypoint data with the string ID removed.  Ordering in the
/// enclosing vector remains part of the physical payload.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityWaypointV1 {
    pub position_m: Vec2,
    pub handoff_tangent_unit: Option<Vec2>,
    pub capture_radius_m: f64,
    pub max_cross_track_m: f64,
    pub max_outbound_heading_error_rad: f64,
    pub min_outbound_progress_mps: f64,
    pub max_outbound_cross_speed_mps: Option<f64>,
    pub min_speed_mps: f64,
    pub max_speed_mps: f64,
    pub min_vertical_speed_mps: Option<f64>,
    pub max_vertical_speed_mps: Option<f64>,
}

impl RouteCapabilityWaypointV1 {
    fn from_waypoint(waypoint: &TransferWaypointSpec) -> Self {
        Self {
            position_m: waypoint.position_m,
            handoff_tangent_unit: waypoint.handoff_tangent_unit,
            capture_radius_m: waypoint.capture_radius_m,
            max_cross_track_m: waypoint.max_cross_track_m,
            max_outbound_heading_error_rad: waypoint.max_outbound_heading_error_rad,
            min_outbound_progress_mps: waypoint.min_outbound_progress_mps,
            max_outbound_cross_speed_mps: waypoint.max_outbound_cross_speed_mps,
            min_speed_mps: waypoint.min_speed_mps,
            max_speed_mps: waypoint.max_speed_mps,
            min_vertical_speed_mps: waypoint.min_vertical_speed_mps,
            max_vertical_speed_mps: waypoint.max_vertical_speed_mps,
        }
    }

    pub fn matches_waypoint(&self, waypoint: &TransferWaypointSpec) -> bool {
        self == &Self::from_waypoint(waypoint)
    }

    pub fn validate(&self, index: usize) -> Result<(), String> {
        let values = [
            ("position_m.x", self.position_m.x),
            ("position_m.y", self.position_m.y),
            ("capture_radius_m", self.capture_radius_m),
            ("max_cross_track_m", self.max_cross_track_m),
            (
                "max_outbound_heading_error_rad",
                self.max_outbound_heading_error_rad,
            ),
            ("min_outbound_progress_mps", self.min_outbound_progress_mps),
            ("min_speed_mps", self.min_speed_mps),
            ("max_speed_mps", self.max_speed_mps),
        ];
        for (field, value) in values {
            if !value.is_finite() {
                return Err(format!("waypoints[{index}].{field} must be finite"));
            }
        }
        for (field, value) in [
            (
                "max_outbound_cross_speed_mps",
                self.max_outbound_cross_speed_mps,
            ),
            ("min_vertical_speed_mps", self.min_vertical_speed_mps),
            ("max_vertical_speed_mps", self.max_vertical_speed_mps),
        ] {
            if let Some(value) = value
                && !value.is_finite()
            {
                return Err(format!("waypoints[{index}].{field} must be finite"));
            }
        }
        if self.capture_radius_m <= 0.0 || self.max_cross_track_m <= 0.0 {
            return Err(format!(
                "waypoints[{index}] capture bounds must be positive"
            ));
        }
        if self.max_outbound_heading_error_rad <= 0.0
            || self.min_outbound_progress_mps <= 0.0
            || self.min_speed_mps < 0.0
            || self.max_speed_mps < self.min_speed_mps
        {
            return Err(format!("waypoints[{index}] handoff bounds are invalid"));
        }
        if let Some(tangent) = self.handoff_tangent_unit
            && (!tangent.x.is_finite()
                || !tangent.y.is_finite()
                || (tangent.length() - 1.0).abs() > 1.0e-6)
        {
            return Err(format!(
                "waypoints[{index}].handoff_tangent_unit is not normalized"
            ));
        }
        if let Some(value) = self.max_outbound_cross_speed_mps
            && value <= 0.0
        {
            return Err(format!(
                "waypoints[{index}].max_outbound_cross_speed_mps must be positive"
            ));
        }
        for (name, minimum, maximum) in [(
            "vertical speed",
            self.min_vertical_speed_mps,
            self.max_vertical_speed_mps,
        )] {
            if let (Some(minimum), Some(maximum)) = (minimum, maximum)
                && maximum < minimum
            {
                return Err(format!("waypoints[{index}] {name} bounds are invalid"));
            }
        }
        Ok(())
    }
}

/// Serializable evaluator-owned form of the planner's resolved safety
/// profile.  `pd_core::SafetyProfile` intentionally remains non-serializable.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilitySafetyProfileV1 {
    pub source_transition_start_m: f64,
    pub source_transition_end_m: f64,
    pub target_transition_start_m: f64,
    pub target_transition_end_m: f64,
    pub horizontal_span_m: f64,
    pub full_envelope: CorridorEnvelope,
    pub contact_envelope: CorridorEnvelope,
}

impl RouteCapabilitySafetyProfileV1 {
    fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("source_transition_start_m", self.source_transition_start_m),
            ("source_transition_end_m", self.source_transition_end_m),
            ("target_transition_start_m", self.target_transition_start_m),
            ("target_transition_end_m", self.target_transition_end_m),
            ("horizontal_span_m", self.horizontal_span_m),
        ] {
            if !value.is_finite() {
                return Err(format!("safety_profile.{field} must be finite"));
            }
        }
        self.full_envelope
            .validate()
            .map_err(|message| format!("safety_profile.full_envelope: {message}"))?;
        self.contact_envelope
            .validate()
            .map_err(|message| format!("safety_profile.contact_envelope: {message}"))?;
        if self.horizontal_span_m <= 0.0
            || self.source_transition_start_m < 0.0
            || self.source_transition_start_m > self.source_transition_end_m
            || self.source_transition_end_m > self.target_transition_start_m
            || self.target_transition_start_m > self.target_transition_end_m
            || self.target_transition_end_m > self.horizontal_span_m
        {
            return Err("safety_profile breakpoints are not ordered".to_owned());
        }
        Ok(())
    }
}

/// Physical portion of the resolved route-planning policy.  The planner's
/// algorithm/version identity is provenance/implementation metadata, not a
/// physical feature.  All numeric and structural policy limits remain here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityPhysicalPolicyV1 {
    pub max_waypoints: u8,
    pub flight_clearance_margin_m: f64,
    pub endpoint_transition_m: f64,
    pub max_extra_loft_ratio: f64,
    pub max_continuation_ratio: f64,
    pub max_handoff_speed_mps: f64,
    pub min_handoff_speed_mps: f64,
    pub min_outbound_progress_mps: f64,
    pub max_outbound_heading_error_rad: f64,
    pub max_outbound_cross_speed_mps: f64,
}

impl RouteCapabilityPhysicalPolicyV1 {
    pub fn from_policy(policy: &RoutePlanningPolicy) -> Result<Self, String> {
        policy.validate()?;
        let value = Self {
            max_waypoints: policy.max_waypoints,
            flight_clearance_margin_m: policy.flight_clearance_margin_m,
            endpoint_transition_m: policy.endpoint_transition_m,
            max_extra_loft_ratio: policy.max_extra_loft_ratio,
            max_continuation_ratio: policy.max_continuation_ratio,
            max_handoff_speed_mps: policy.max_handoff_speed_mps,
            min_handoff_speed_mps: policy.min_handoff_speed_mps,
            min_outbound_progress_mps: policy.min_outbound_progress_mps,
            max_outbound_heading_error_rad: policy.max_outbound_heading_error_rad,
            max_outbound_cross_speed_mps: policy.max_outbound_cross_speed_mps,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.max_waypoints > 2 {
            return Err("max_waypoints must be <= 2 for planner V1".to_owned());
        }
        for (label, value) in [
            ("flight_clearance_margin_m", self.flight_clearance_margin_m),
            ("endpoint_transition_m", self.endpoint_transition_m),
            ("max_extra_loft_ratio", self.max_extra_loft_ratio),
            ("max_continuation_ratio", self.max_continuation_ratio),
            ("max_handoff_speed_mps", self.max_handoff_speed_mps),
            ("min_handoff_speed_mps", self.min_handoff_speed_mps),
            ("min_outbound_progress_mps", self.min_outbound_progress_mps),
            (
                "max_outbound_heading_error_rad",
                self.max_outbound_heading_error_rad,
            ),
            (
                "max_outbound_cross_speed_mps",
                self.max_outbound_cross_speed_mps,
            ),
        ] {
            if !value.is_finite() {
                return Err(format!("{label} must be finite"));
            }
        }
        if self.flight_clearance_margin_m < 0.0 {
            return Err("flight_clearance_margin_m must be non-negative".to_owned());
        }
        if self.endpoint_transition_m <= 0.0 || self.endpoint_transition_m > 96.0 {
            return Err("endpoint_transition_m must be within (0, 96]".to_owned());
        }
        if !(0.0..=1.0).contains(&self.max_extra_loft_ratio) {
            return Err("max_extra_loft_ratio must be within [0, 1]".to_owned());
        }
        if !(0.0..=1.0).contains(&self.max_continuation_ratio) || self.max_continuation_ratio <= 0.0
        {
            return Err("max_continuation_ratio must be within (0, 1]".to_owned());
        }
        if self.min_handoff_speed_mps <= 0.0
            || self.max_handoff_speed_mps <= 0.0
            || self.max_handoff_speed_mps < self.min_handoff_speed_mps
        {
            return Err("handoff speed bounds must be positive and ordered".to_owned());
        }
        if self.min_outbound_progress_mps <= 0.0 {
            return Err("min_outbound_progress_mps must be positive".to_owned());
        }
        if self.max_outbound_heading_error_rad <= 0.0 || self.max_outbound_heading_error_rad > PI {
            return Err("max_outbound_heading_error_rad must be within (0, pi]".to_owned());
        }
        if self.max_outbound_cross_speed_mps <= 0.0 {
            return Err("max_outbound_cross_speed_mps must be positive".to_owned());
        }
        Ok(())
    }
}

/// Canonical physical payload accepted by either D1 predictor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityPhysicalInputV1 {
    pub gravity_mps2: f64,
    pub terrain: TerrainDefinition,
    pub source_pad: RouteCapabilityPadGeometryV1,
    pub target_pad: RouteCapabilityPadGeometryV1,
    pub vehicle: VehicleSpec,
    pub initial_state: VehicleInitialState,
    pub policy: RouteCapabilityPhysicalPolicyV1,
    pub safety_profile: RouteCapabilitySafetyProfileV1,
    pub selected_centerline_m: Vec<Vec2>,
    pub normalized_geometry: NormalizedRouteGeometry,
    pub horizontal_sign: i8,
    pub waypoints: Vec<RouteCapabilityWaypointV1>,
    pub topology: RouteTopology,
    pub route_angle_deg: f64,
    pub route_radius_m: f64,
}

impl RouteCapabilityPhysicalInputV1 {
    pub fn validate(&self) -> Result<(), String> {
        if !self.gravity_mps2.is_finite() || self.gravity_mps2 <= 0.0 {
            return Err("gravity_mps2 must be positive and finite".to_owned());
        }
        self.terrain.validate()?;
        self.source_pad.validate("source_pad")?;
        self.target_pad.validate("target_pad")?;
        self.vehicle.validate()?;
        self.initial_state.validate()?;
        self.policy.validate()?;
        self.safety_profile.validate()?;
        for (field, value) in [
            (
                "normalized_geometry.direct_horizontal_span_m",
                self.normalized_geometry.direct_horizontal_span_m,
            ),
            (
                "normalized_geometry.direct_distance_m",
                self.normalized_geometry.direct_distance_m,
            ),
            (
                "normalized_geometry.route_angle_rad",
                self.normalized_geometry.route_angle_rad,
            ),
            (
                "normalized_geometry.route_angle_deg",
                self.normalized_geometry.route_angle_deg,
            ),
            ("route_angle_deg", self.route_angle_deg),
            ("route_radius_m", self.route_radius_m),
        ] {
            if !value.is_finite() {
                return Err(format!("{field} must be finite"));
            }
        }
        if !matches!(self.horizontal_sign, -1 | 1)
            || self.normalized_geometry.horizontal_sign != self.horizontal_sign
        {
            return Err(
                "horizontal_sign must be -1 or +1 and match normalized_geometry".to_owned(),
            );
        }
        if self.normalized_geometry.direct_horizontal_span_m <= 0.0
            || self.normalized_geometry.direct_distance_m <= 0.0
            || self.route_radius_m <= 0.0
        {
            return Err("route spans and radius must be positive".to_owned());
        }
        if self.selected_centerline_m.len() < 2 {
            return Err("selected_centerline_m must contain at least two points".to_owned());
        }
        for (index, point) in self.selected_centerline_m.iter().enumerate() {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(format!("selected_centerline_m[{index}] must be finite"));
            }
        }
        let normalized_centerline = self
            .selected_centerline_m
            .iter()
            .map(|point| {
                Vec2::new(
                    f64::from(self.horizontal_sign) * (point.x - self.source_pad.center_x_m),
                    point.y,
                )
            })
            .collect::<Vec<_>>();
        if normalized_centerline[0].x.abs() > ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M
            || (normalized_centerline.last().unwrap().x
                - self.normalized_geometry.direct_horizontal_span_m)
                .abs()
                > ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M
            || normalized_centerline.windows(2).any(|pair| {
                pair[1].x <= pair[0].x + ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M
                    || (pair[1] - pair[0]).length() <= ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M
            })
        {
            return Err(
                "selected_centerline_m must be finite, strictly ordered, and cover the route scope"
                    .to_owned(),
            );
        }
        if (self.safety_profile.horizontal_span_m
            - self.normalized_geometry.direct_horizontal_span_m)
            .abs()
            > ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M
        {
            return Err("safety profile and normalized route spans do not match".to_owned());
        }
        let mut previous_waypoint_x = self.safety_profile.source_transition_end_m;
        for (index, waypoint) in self.waypoints.iter().enumerate() {
            waypoint.validate(index)?;
            let normalized = Vec2::new(
                f64::from(self.horizontal_sign)
                    * (waypoint.position_m.x - self.source_pad.center_x_m),
                waypoint.position_m.y,
            );
            if normalized.x <= previous_waypoint_x + ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M
                || normalized.x > self.normalized_geometry.direct_horizontal_span_m
                || !normalized_centerline.iter().any(|point| {
                    (*point - normalized).length() <= ROUTE_CAPABILITY_GEOMETRY_TOLERANCE_M
                })
            {
                return Err(format!(
                    "waypoints[{index}] is not an ordered selected-centerline point after tracking entry"
                ));
            }
            previous_waypoint_x = normalized.x;
        }
        match (self.topology, self.waypoints.is_empty()) {
            (RouteTopology::Direct, true) | (RouteTopology::Waypoint, false) => Ok(()),
            (RouteTopology::Direct, false) => {
                Err("direct topology cannot contain waypoints".to_owned())
            }
            (RouteTopology::Waypoint, true) => {
                Err("waypoint topology must contain at least one waypoint".to_owned())
            }
        }
    }

    /// Digest of the physical payload only.  Provenance, model transforms,
    /// and the digest field itself cannot influence this value.
    pub fn digest(&self) -> Result<RouteCapabilityInputDigest, String> {
        self.validate()?;
        canonical_digest(self)
    }
}

pub fn route_capability_input_digest(
    input: &RouteCapabilityInputV1,
) -> Result<RouteCapabilityInputDigest, String> {
    input.physical_digest()
}

/// Join identities kept outside the physical feature payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCapabilityInputProvenanceV1 {
    pub request_digest: String,
    pub route_plan_digest: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub waypoint_ids: Vec<String>,
}

impl RouteCapabilityInputProvenanceV1 {
    fn validate(&self, expected_waypoint_count: usize) -> Result<(), String> {
        for (field, value) in [
            ("request_digest", self.request_digest.as_str()),
            ("route_plan_digest", self.route_plan_digest.as_str()),
            ("source_pad_id", self.source_pad_id.as_str()),
            ("target_pad_id", self.target_pad_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("provenance.{field} must not be empty"));
            }
        }
        if self.source_pad_id == self.target_pad_id {
            return Err("provenance source and target IDs must differ".to_owned());
        }
        if self.waypoint_ids.len() != expected_waypoint_count {
            return Err("provenance waypoint IDs do not match physical waypoint count".to_owned());
        }
        let mut ids = BTreeSet::new();
        for id in &self.waypoint_ids {
            if id.trim().is_empty() || !ids.insert(id) {
                return Err("provenance waypoint IDs must be non-empty and unique".to_owned());
            }
        }
        Ok(())
    }
}

/// Complete canonical input artifact, including provenance only as a join
/// sidecar.  The digest is over `physical`, never over this enclosing value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityInputV1 {
    pub schema_version: u32,
    pub physical: RouteCapabilityPhysicalInputV1,
    pub provenance: RouteCapabilityInputProvenanceV1,
    pub input_digest: RouteCapabilityInputDigest,
}

impl RouteCapabilityInputV1 {
    pub fn from_request_and_plan(
        request: &RoutePlanningRequest,
        route_plan: &RoutePlan,
    ) -> Result<Self, String> {
        build_route_capability_input(request, route_plan)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION {
            return Err(format!(
                "schema_version must equal {ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION}"
            ));
        }
        self.physical.validate()?;
        self.provenance.validate(self.physical.waypoints.len())?;
        let expected = canonical_digest(&self.physical)?;
        if self.input_digest != expected {
            return Err(format!(
                "input_digest does not match physical payload: expected {expected}"
            ));
        }
        Ok(())
    }

    pub fn canonical_physical_bytes(&self) -> Result<Vec<u8>, String> {
        self.physical.validate()?;
        canonical_serialized_bytes(&self.physical)
    }

    pub fn physical_digest(&self) -> Result<RouteCapabilityInputDigest, String> {
        self.physical.digest()
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.schema_version = ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION;
        self.physical.validate()?;
        self.provenance.validate(self.physical.waypoints.len())?;
        self.input_digest = canonical_digest(&self.physical)?;
        self.validate()?;
        Ok(self)
    }
}

/// Build the canonical input from a request and selected persisted plan.
/// Planner algorithm/ranking diagnostics are deliberately never read.
pub fn build_route_capability_input(
    request: &RoutePlanningRequest,
    route_plan: &RoutePlan,
) -> Result<RouteCapabilityInputV1, String> {
    request.validate().map_err(|error| error.to_string())?;
    route_plan
        .route
        .validate()
        .map_err(|message| format!("selected route is invalid: {message}"))?;
    route_plan.policy.validate()?;
    if request.policy != route_plan.policy {
        return Err("selected route policy does not match planning request".to_owned());
    }
    if route_plan.route.source_pad_id != request.source_pad_id
        || route_plan.route.target_pad_id != request.target_pad_id
    {
        return Err("selected route source/target IDs do not match request".to_owned());
    }
    let expected_topology = if route_plan.route.waypoints.is_empty() {
        RouteTopology::Direct
    } else {
        RouteTopology::Waypoint
    };
    if route_plan.topology != expected_topology {
        return Err("selected route topology does not match waypoint count".to_owned());
    }
    let source_pad = request
        .source_pad()
        .ok_or_else(|| "source pad is missing from request world".to_owned())?;
    let target_pad = request
        .target_pad()
        .ok_or_else(|| "target pad is missing from request world".to_owned())?;
    let (profile, _) = build_endpoint_profile(
        request,
        route_plan.normalized_geometry.direct_horizontal_span_m,
    )
    .map_err(|error| error.to_string())?;
    let selected_centerline_m = if route_plan.diagnostics.selected_centerline_m.len() >= 2 {
        route_plan.diagnostics.selected_centerline_m.clone()
    } else {
        // A direct route often has no persisted diagnostic centerline.  Derive
        // the same endpoint-shaped centerline without importing planner
        // ranking or diagnostic fields.
        let waypoints_normalized = route_plan
            .route
            .waypoints
            .iter()
            .map(|waypoint| {
                Vec2::new(
                    f64::from(route_plan.normalized_geometry.horizontal_sign)
                        * (waypoint.position_m.x - source_pad.center_x_m),
                    waypoint.position_m.y,
                )
            })
            .collect::<Vec<_>>();
        let normalized = endpoint_shaped_centerline(
            request,
            &route_plan.normalized_geometry,
            &profile,
            &waypoints_normalized,
        )
        .map_err(|error| error.to_string())?;
        normalized
            .into_iter()
            .map(|point| {
                Vec2::new(
                    source_pad.center_x_m
                        + (f64::from(route_plan.normalized_geometry.horizontal_sign) * point.x),
                    point.y,
                )
            })
            .collect()
    };
    let physical = RouteCapabilityPhysicalInputV1 {
        gravity_mps2: request.world.gravity_mps2,
        terrain: request.world.terrain.clone(),
        source_pad: RouteCapabilityPadGeometryV1::from_pad(source_pad),
        target_pad: RouteCapabilityPadGeometryV1::from_pad(target_pad),
        vehicle: request.vehicle.clone(),
        initial_state: request.initial_state.clone(),
        policy: RouteCapabilityPhysicalPolicyV1::from_policy(&request.policy)?,
        safety_profile: RouteCapabilitySafetyProfileV1 {
            source_transition_start_m: profile.source_transition_start_m,
            source_transition_end_m: profile.source_transition_end_m,
            target_transition_start_m: profile.target_transition_start_m,
            target_transition_end_m: profile.target_transition_end_m,
            horizontal_span_m: profile.horizontal_span_m,
            full_envelope: profile.full_envelope,
            contact_envelope: profile.contact_envelope,
        },
        selected_centerline_m,
        normalized_geometry: route_plan.normalized_geometry.clone(),
        horizontal_sign: route_plan.normalized_geometry.horizontal_sign,
        waypoints: route_plan
            .route
            .waypoints
            .iter()
            .map(RouteCapabilityWaypointV1::from_waypoint)
            .collect(),
        topology: route_plan.topology,
        route_angle_deg: route_plan.route.route_angle_deg,
        route_radius_m: route_plan.route.route_radius_m,
    };
    let provenance = RouteCapabilityInputProvenanceV1 {
        request_digest: if route_plan.request_digest.trim().is_empty() {
            canonical_digest(request)?
        } else {
            route_plan.request_digest.clone()
        },
        route_plan_digest: if route_plan.plan_digest.trim().is_empty() {
            canonical_digest(route_plan)?
        } else {
            route_plan.plan_digest.clone()
        },
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        waypoint_ids: route_plan
            .route
            .waypoints
            .iter()
            .map(|waypoint| waypoint.id.clone())
            .collect(),
    };
    RouteCapabilityInputV1 {
        schema_version: ROUTE_CAPABILITY_INPUT_SCHEMA_VERSION,
        physical,
        provenance,
        input_digest: String::new(),
    }
    .seal()
}

/// A finite, closed scalar interval.  Non-finite endpoints and reversed
/// bounds are malformed rather than silently widened.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FiniteIntervalV1 {
    pub lower: f64,
    pub upper: f64,
}

pub type ClosedScalarIntervalV1 = FiniteIntervalV1;
pub type ScalarIntervalV1 = FiniteIntervalV1;

impl FiniteIntervalV1 {
    pub fn new(lower: f64, upper: f64) -> Result<Self, String> {
        let interval = Self { lower, upper };
        interval.validate()?;
        Ok(interval)
    }

    pub fn singleton(value: f64) -> Result<Self, String> {
        Self::new(value, value)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.lower.is_finite() || !self.upper.is_finite() {
            return Err("interval endpoints must be finite".to_owned());
        }
        if self.lower > self.upper {
            return Err("interval lower must be <= upper".to_owned());
        }
        Ok(())
    }

    pub fn contains(&self, value: f64) -> bool {
        value.is_finite() && self.lower <= value && value <= self.upper
    }

    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }
}

/// A circular closed attitude interval.  The center is angular and the
/// half-width is bounded by pi; callers must not represent a wraparound arc
/// as a scalar minimum/maximum pair.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CircularAttitudeIntervalV1 {
    pub center_rad: f64,
    pub half_width_rad: f64,
}

pub type CircularIntervalV1 = CircularAttitudeIntervalV1;

impl CircularAttitudeIntervalV1 {
    pub fn new(center_rad: f64, half_width_rad: f64) -> Result<Self, String> {
        let interval = Self {
            center_rad,
            half_width_rad,
        };
        interval.validate()?;
        Ok(interval)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.center_rad.is_finite() || !self.half_width_rad.is_finite() {
            return Err("circular interval values must be finite".to_owned());
        }
        if !(0.0..=PI).contains(&self.half_width_rad) {
            return Err("circular interval half_width_rad must be within [0, pi]".to_owned());
        }
        Ok(())
    }

    pub fn contains(&self, value_rad: f64) -> bool {
        if !value_rad.is_finite() {
            return false;
        }
        if self.half_width_rad >= PI {
            return true;
        }
        circular_distance(value_rad, self.center_rad).abs() <= self.half_width_rad
    }
}

fn circular_distance(value_rad: f64, center_rad: f64) -> f64 {
    let mut delta = (value_rad - center_rad) % std::f64::consts::TAU;
    if delta > PI {
        delta -= std::f64::consts::TAU;
    } else if delta < -PI {
        delta += std::f64::consts::TAU;
    }
    delta
}

/// Route-relative conservative state carried from one phase into the next.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseStateSetV1 {
    pub schema_version: u32,
    pub identity: String,
    pub progress_m: FiniteIntervalV1,
    pub along_track_error_m: FiniteIntervalV1,
    pub cross_track_error_m: FiniteIntervalV1,
    pub along_track_velocity_mps: FiniteIntervalV1,
    pub cross_track_velocity_mps: FiniteIntervalV1,
    pub attitude: CircularAttitudeIntervalV1,
    pub angular_rate_radps: FiniteIntervalV1,
    pub mass_kg: FiniteIntervalV1,
    pub fuel_kg: FiniteIntervalV1,
    pub elapsed_time_s: Option<FiniteIntervalV1>,
    pub digest: String,
}

impl PhaseStateSetV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: impl Into<String>,
        progress_m: FiniteIntervalV1,
        along_track_error_m: FiniteIntervalV1,
        cross_track_error_m: FiniteIntervalV1,
        along_track_velocity_mps: FiniteIntervalV1,
        cross_track_velocity_mps: FiniteIntervalV1,
        attitude: CircularAttitudeIntervalV1,
        angular_rate_radps: FiniteIntervalV1,
        mass_kg: FiniteIntervalV1,
        fuel_kg: FiniteIntervalV1,
        elapsed_time_s: Option<FiniteIntervalV1>,
    ) -> Result<Self, String> {
        Self {
            schema_version: PHASE_STATE_SET_SCHEMA_VERSION,
            identity: identity.into(),
            progress_m,
            along_track_error_m,
            cross_track_error_m,
            along_track_velocity_mps,
            cross_track_velocity_mps,
            attitude,
            angular_rate_radps,
            mass_kg,
            fuel_kg,
            elapsed_time_s,
            digest: String::new(),
        }
        .seal()
    }

    pub fn validate_content(&self) -> Result<(), String> {
        if self.schema_version != PHASE_STATE_SET_SCHEMA_VERSION {
            return Err(format!(
                "state-set schema_version must equal {PHASE_STATE_SET_SCHEMA_VERSION}"
            ));
        }
        if self.identity.trim().is_empty() {
            return Err("state-set identity must not be empty".to_owned());
        }
        for (name, interval) in [
            ("progress_m", self.progress_m),
            ("along_track_error_m", self.along_track_error_m),
            ("cross_track_error_m", self.cross_track_error_m),
            ("along_track_velocity_mps", self.along_track_velocity_mps),
            ("cross_track_velocity_mps", self.cross_track_velocity_mps),
            ("angular_rate_radps", self.angular_rate_radps),
            ("mass_kg", self.mass_kg),
            ("fuel_kg", self.fuel_kg),
        ] {
            interval
                .validate()
                .map_err(|message| format!("state-set {name}: {message}"))?;
        }
        self.attitude
            .validate()
            .map_err(|message| format!("state-set attitude: {message}"))?;
        if let Some(interval) = self.elapsed_time_s {
            interval
                .validate()
                .map_err(|message| format!("state-set elapsed_time_s: {message}"))?;
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_content()?;
        let expected = state_set_digest(self)?;
        if self.digest != expected {
            return Err(format!(
                "state-set digest does not match canonical content: expected {expected}"
            ));
        }
        Ok(())
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.validate_content()?;
        self.digest = state_set_digest(&self)?;
        self.validate()?;
        Ok(self)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn contains_point(
        &self,
        progress_m: f64,
        along_track_error_m: f64,
        cross_track_error_m: f64,
        along_track_velocity_mps: f64,
        cross_track_velocity_mps: f64,
        attitude_rad: f64,
        angular_rate_radps: f64,
        mass_kg: f64,
        fuel_kg: f64,
        elapsed_time_s: Option<f64>,
    ) -> bool {
        self.progress_m.contains(progress_m)
            && self.along_track_error_m.contains(along_track_error_m)
            && self.cross_track_error_m.contains(cross_track_error_m)
            && self
                .along_track_velocity_mps
                .contains(along_track_velocity_mps)
            && self
                .cross_track_velocity_mps
                .contains(cross_track_velocity_mps)
            && self.attitude.contains(attitude_rad)
            && self.angular_rate_radps.contains(angular_rate_radps)
            && self.mass_kg.contains(mass_kg)
            && self.fuel_kg.contains(fuel_kg)
            && match (self.elapsed_time_s, elapsed_time_s) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(interval), Some(value)) => interval.contains(value),
            }
    }
}

fn state_set_digest(state_set: &PhaseStateSetV1) -> Result<String, String> {
    let mut material = state_set.clone();
    material.digest.clear();
    canonical_digest(&material)
}

/// A phase or boundary containment proof in the fixed composition order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateSetContainmentDecisionV1 {
    pub identity: String,
    pub decision: CapabilityDecision,
    pub reason: Option<String>,
    pub digest: String,
}

impl StateSetContainmentDecisionV1 {
    pub fn new(
        identity: impl Into<String>,
        decision: CapabilityDecision,
        reason: Option<String>,
    ) -> Result<Self, String> {
        Self {
            identity: identity.into(),
            decision,
            reason,
            digest: String::new(),
        }
        .seal()
    }

    fn validate_content(&self) -> Result<(), String> {
        if self.identity.trim().is_empty() {
            return Err("containment identity must not be empty".to_owned());
        }
        validate_optional_reason(self.decision, self.reason.as_deref())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_content()?;
        let expected = containment_digest(self)?;
        if self.digest != expected {
            return Err(format!(
                "containment digest does not match canonical content: expected {expected}"
            ));
        }
        Ok(())
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.validate_content()?;
        self.digest = containment_digest(&self)?;
        self.validate()?;
        Ok(self)
    }
}

fn containment_digest(value: &StateSetContainmentDecisionV1) -> Result<String, String> {
    let mut material = value.clone();
    material.digest.clear();
    canonical_digest(&material)
}

fn validate_optional_reason(
    decision: CapabilityDecision,
    reason: Option<&str>,
) -> Result<(), String> {
    match (decision, reason) {
        (CapabilityDecision::Supported, None) => Ok(()),
        (CapabilityDecision::Supported, Some(_)) => {
            Err("supported decisions must not carry a reason code".to_owned())
        }
        (_, None) => Err("non-supported decisions require a stable reason code".to_owned()),
        (_, Some(reason)) => validate_reason_code(reason),
    }
}

/// One declared model transform.  Transforms are capability configuration,
/// never common-input features.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityTransformV1 {
    pub name: String,
    pub version: String,
    pub parameters: BTreeMap<String, f64>,
}

impl RouteCapabilityTransformV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.version.trim().is_empty() {
            return Err("capability transform name and version must not be empty".to_owned());
        }
        validate_finite_map("transform parameters", &self.parameters)
    }
}

/// Complete behavior-bearing numeric/configuration declaration.  This is
/// intentionally explicit so fitting code cannot smuggle case-specific values
/// through a free-form scenario or metadata object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityConfigurationV1 {
    pub version: String,
    pub transforms: Vec<RouteCapabilityTransformV1>,
    pub interpolation: BTreeMap<String, String>,
    pub padding: BTreeMap<String, f64>,
    pub solver_limits: BTreeMap<String, f64>,
    pub reason_code_mapping: BTreeMap<String, String>,
}

impl Default for RouteCapabilityConfigurationV1 {
    fn default() -> Self {
        Self {
            version: "d1a_common_v1".to_owned(),
            transforms: Vec::new(),
            interpolation: BTreeMap::new(),
            padding: BTreeMap::new(),
            solver_limits: BTreeMap::new(),
            reason_code_mapping: BTreeMap::new(),
        }
    }
}

impl RouteCapabilityConfigurationV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.version.trim().is_empty() {
            return Err("capability configuration version must not be empty".to_owned());
        }
        for transform in &self.transforms {
            transform.validate()?;
        }
        validate_finite_map("configuration padding", &self.padding)?;
        validate_finite_map("configuration solver_limits", &self.solver_limits)?;
        for (name, reason) in &self.reason_code_mapping {
            if name.trim().is_empty() {
                return Err("configuration reason-code keys must not be empty".to_owned());
            }
            validate_reason_code(reason)?;
        }
        Ok(())
    }
}

fn validate_finite_map(name: &str, map: &BTreeMap<String, f64>) -> Result<(), String> {
    for (key, value) in map {
        if key.trim().is_empty() || !value.is_finite() {
            return Err(format!("{name} contains an empty key or nonfinite value"));
        }
    }
    Ok(())
}

/// Declared physical domain for a capability artifact.  Empty terrain digests
/// mean no terrain restriction; all other dimensions remain explicit and
/// finite.  This represents a domain declaration, not an outcome label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityDomainV1 {
    pub route_angle_deg: FiniteIntervalV1,
    pub route_radius_m: FiniteIntervalV1,
    pub horizontal_signs: Vec<i8>,
    pub waypoint_counts: Vec<usize>,
    pub vehicle_dry_mass_kg: FiniteIntervalV1,
    pub initial_fuel_kg: FiniteIntervalV1,
    pub terrain_digests: Vec<String>,
}

impl Default for RouteCapabilityDomainV1 {
    fn default() -> Self {
        Self {
            route_angle_deg: FiniteIntervalV1 {
                lower: -180.0,
                upper: 180.0,
            },
            route_radius_m: FiniteIntervalV1 {
                lower: 0.0,
                upper: f64::MAX / 4.0,
            },
            horizontal_signs: vec![-1, 1],
            waypoint_counts: vec![0, 1, 2],
            vehicle_dry_mass_kg: FiniteIntervalV1 {
                lower: 0.0,
                upper: f64::MAX / 4.0,
            },
            initial_fuel_kg: FiniteIntervalV1 {
                lower: 0.0,
                upper: f64::MAX / 4.0,
            },
            terrain_digests: Vec::new(),
        }
    }
}

impl RouteCapabilityDomainV1 {
    pub fn for_input(input: &RouteCapabilityInputV1) -> Result<Self, String> {
        input.validate()?;
        Ok(Self {
            route_angle_deg: FiniteIntervalV1::singleton(input.physical.route_angle_deg)?,
            route_radius_m: FiniteIntervalV1::singleton(input.physical.route_radius_m)?,
            horizontal_signs: vec![input.physical.horizontal_sign],
            waypoint_counts: vec![input.physical.waypoints.len()],
            vehicle_dry_mass_kg: FiniteIntervalV1::singleton(input.physical.vehicle.dry_mass_kg)?,
            initial_fuel_kg: FiniteIntervalV1::singleton(input.physical.vehicle.initial_fuel_kg)?,
            terrain_digests: vec![terrain_digest(&input.physical.terrain)?],
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        self.route_angle_deg.validate()?;
        self.route_radius_m.validate()?;
        self.vehicle_dry_mass_kg.validate()?;
        self.initial_fuel_kg.validate()?;
        if self.horizontal_signs.is_empty()
            || self
                .horizontal_signs
                .iter()
                .any(|sign| !matches!(sign, -1 | 1))
        {
            return Err("domain horizontal_signs must contain only -1 or +1".to_owned());
        }
        if self.waypoint_counts.is_empty() || self.waypoint_counts.iter().any(|count| *count > 2) {
            return Err("domain waypoint_counts must contain values in [0, 2]".to_owned());
        }
        if self
            .terrain_digests
            .iter()
            .any(|digest| digest.trim().is_empty())
        {
            return Err("domain terrain digests must not be empty".to_owned());
        }
        Ok(())
    }

    pub fn contains(&self, input: &RouteCapabilityInputV1) -> Result<bool, String> {
        input.validate()?;
        self.validate()?;
        let physical = &input.physical;
        let terrain = terrain_digest(&physical.terrain)?;
        Ok(self.route_angle_deg.contains(physical.route_angle_deg)
            && self.route_radius_m.contains(physical.route_radius_m)
            && self.horizontal_signs.contains(&physical.horizontal_sign)
            && self.waypoint_counts.contains(&physical.waypoints.len())
            && self
                .vehicle_dry_mass_kg
                .contains(physical.vehicle.dry_mass_kg)
            && self
                .initial_fuel_kg
                .contains(physical.vehicle.initial_fuel_kg)
            && (self.terrain_digests.is_empty() || self.terrain_digests.contains(&terrain)))
    }
}

fn terrain_digest(terrain: &TerrainDefinition) -> Result<String, String> {
    terrain.validate()?;
    canonical_digest(terrain)
}

/// A sealed, model-bearing capability artifact.  The model/config digest
/// covers only behavior-bearing model/configuration; the capability digest
/// additionally covers artifact identity, domain, and ordered evidence IDs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityArtifactV1 {
    pub schema_version: u32,
    pub status: CapabilityArtifactStatus,
    pub invalid_reason: Option<String>,
    pub alternative: String,
    pub model_payload: Value,
    pub configuration: RouteCapabilityConfigurationV1,
    pub declared_domain: RouteCapabilityDomainV1,
    pub training_evidence_digests: Vec<String>,
    pub model_config_digest: RouteCapabilityModelConfigDigest,
    pub capability_artifact_digest: RouteCapabilityArtifactDigest,
}

#[derive(Serialize)]
struct ModelConfigDigestMaterial<'a> {
    model_payload: &'a Value,
    configuration: &'a RouteCapabilityConfigurationV1,
}

#[derive(Serialize)]
struct CapabilityArtifactDigestMaterial<'a> {
    schema_version: u32,
    alternative: &'a str,
    model_payload: &'a Value,
    configuration: &'a RouteCapabilityConfigurationV1,
    declared_domain: &'a RouteCapabilityDomainV1,
    model_config_digest: &'a str,
    training_evidence_digests: &'a [String],
}

#[derive(Serialize)]
struct InvalidCapabilityArtifactDigestMaterial<'a> {
    schema_version: u32,
    status: CapabilityArtifactStatus,
    invalid_reason: &'a Option<String>,
}

impl RouteCapabilityArtifactV1 {
    pub fn new(
        alternative: impl Into<String>,
        model_payload: Value,
        configuration: RouteCapabilityConfigurationV1,
        declared_domain: RouteCapabilityDomainV1,
        training_evidence_digests: Vec<String>,
    ) -> Result<Self, String> {
        Self {
            schema_version: ROUTE_CAPABILITY_ARTIFACT_SCHEMA_VERSION,
            status: CapabilityArtifactStatus::Complete,
            invalid_reason: None,
            alternative: alternative.into(),
            model_payload,
            configuration,
            declared_domain,
            training_evidence_digests,
            model_config_digest: String::new(),
            capability_artifact_digest: String::new(),
        }
        .seal()
    }

    pub fn invalid(reason: impl Into<String>) -> Result<Self, String> {
        let reason = reason.into();
        validate_reason_code(&reason)?;
        if !reason.starts_with(REASON_INVALID_ARTIFACT) {
            return Err(
                "invalid capability artifacts require invalid/artifact/* reason".to_owned(),
            );
        }
        let mut artifact = Self {
            schema_version: ROUTE_CAPABILITY_ARTIFACT_SCHEMA_VERSION,
            status: CapabilityArtifactStatus::Invalid,
            invalid_reason: Some(reason),
            alternative: String::new(),
            model_payload: Value::Null,
            configuration: RouteCapabilityConfigurationV1::default(),
            declared_domain: RouteCapabilityDomainV1::default(),
            training_evidence_digests: Vec::new(),
            model_config_digest: String::new(),
            capability_artifact_digest: String::new(),
        };
        artifact.capability_artifact_digest = artifact.capability_artifact_digest()?;
        Ok(artifact)
    }

    pub fn model_config_digest(&self) -> Result<RouteCapabilityModelConfigDigest, String> {
        self.configuration.validate()?;
        canonical_digest(&ModelConfigDigestMaterial {
            model_payload: &self.model_payload,
            configuration: &self.configuration,
        })
    }

    pub fn capability_artifact_digest(&self) -> Result<RouteCapabilityArtifactDigest, String> {
        if self.status == CapabilityArtifactStatus::Invalid {
            return canonical_digest(&InvalidCapabilityArtifactDigestMaterial {
                schema_version: self.schema_version,
                status: self.status,
                invalid_reason: &self.invalid_reason,
            });
        }
        canonical_digest(&CapabilityArtifactDigestMaterial {
            schema_version: self.schema_version,
            alternative: &self.alternative,
            model_payload: &self.model_payload,
            configuration: &self.configuration,
            declared_domain: &self.declared_domain,
            model_config_digest: &self.model_config_digest,
            training_evidence_digests: &self.training_evidence_digests,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != ROUTE_CAPABILITY_ARTIFACT_SCHEMA_VERSION {
            return Err(format!(
                "capability schema_version must equal {ROUTE_CAPABILITY_ARTIFACT_SCHEMA_VERSION}"
            ));
        }
        if self.status == CapabilityArtifactStatus::Invalid {
            let reason = self
                .invalid_reason
                .as_deref()
                .ok_or_else(|| "invalid capability artifact requires a reason".to_owned())?;
            validate_reason_code(reason)?;
            if !reason.starts_with(REASON_INVALID_ARTIFACT) {
                return Err(
                    "invalid capability artifact reason must use invalid/artifact/*".to_owned(),
                );
            }
            let expected = self.capability_artifact_digest()?;
            if self.capability_artifact_digest != expected {
                return Err(format!(
                    "invalid capability_artifact_digest does not match canonical invalid artifact: expected {expected}"
                ));
            }
            return Ok(());
        }
        if self.invalid_reason.is_some() || self.alternative.trim().is_empty() {
            return Err("complete capability artifact has invalid identity fields".to_owned());
        }
        self.configuration.validate()?;
        self.declared_domain.validate()?;
        for digest in &self.training_evidence_digests {
            if digest.trim().is_empty() {
                return Err("training evidence digests must not be empty".to_owned());
            }
        }
        let expected_model = self.model_config_digest()?;
        if self.model_config_digest != expected_model {
            return Err(format!(
                "model_config_digest does not match canonical model/config: expected {expected_model}"
            ));
        }
        let expected_artifact = self.capability_artifact_digest()?;
        if self.capability_artifact_digest != expected_artifact {
            return Err(format!(
                "capability_artifact_digest does not match canonical artifact: expected {expected_artifact}"
            ));
        }
        Ok(())
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.schema_version = ROUTE_CAPABILITY_ARTIFACT_SCHEMA_VERSION;
        self.status = CapabilityArtifactStatus::Complete;
        self.invalid_reason = None;
        if self.alternative.trim().is_empty() {
            return Err("capability alternative must not be empty".to_owned());
        }
        self.configuration.validate()?;
        self.declared_domain.validate()?;
        if self
            .training_evidence_digests
            .iter()
            .any(|digest| digest.trim().is_empty())
        {
            return Err("training evidence digests must not be empty".to_owned());
        }
        self.model_config_digest = self.model_config_digest()?;
        self.capability_artifact_digest = self.capability_artifact_digest()?;
        self.validate()?;
        Ok(self)
    }
}

pub fn capability_model_config_digest(
    artifact: &RouteCapabilityArtifactV1,
) -> Result<RouteCapabilityModelConfigDigest, String> {
    artifact.model_config_digest()
}

pub fn capability_artifact_digest(
    artifact: &RouteCapabilityArtifactV1,
) -> Result<RouteCapabilityArtifactDigest, String> {
    artifact.capability_artifact_digest()
}

/// Fixed ordered phase identity for route composition.  There is no target
/// landing phase in this contract.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RouteCapabilityPhaseV1 {
    InitialState,
    PadDeparture,
    Acquisition,
    RouteLeg { leg_index: usize },
    Handoff { waypoint_index: usize },
}

impl RouteCapabilityPhaseV1 {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InitialState => "initial_state",
            Self::PadDeparture => "pad_departure",
            Self::Acquisition => "acquisition",
            Self::RouteLeg { .. } => "route_leg",
            Self::Handoff { .. } => "handoff",
        }
    }

    pub fn reason_prefix(&self) -> String {
        match self {
            Self::InitialState => "initial_state".to_owned(),
            Self::PadDeparture => "pad_departure".to_owned(),
            Self::Acquisition => "acquisition".to_owned(),
            Self::RouteLeg { leg_index } => format!("route_leg_{leg_index}"),
            Self::Handoff { waypoint_index } => format!("handoff_{waypoint_index}"),
        }
    }
}

/// One phase result and, when applicable, its outbound boundary-containment
/// result.  Composition checks the phase first and containment second.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityPhaseDecisionV1 {
    pub phase: RouteCapabilityPhaseV1,
    pub decision: CapabilityDecision,
    pub reason: Option<String>,
    pub terminal_state_set: Option<PhaseStateSetV1>,
    pub containment: Option<StateSetContainmentDecisionV1>,
}

impl RouteCapabilityPhaseDecisionV1 {
    pub fn new(
        phase: RouteCapabilityPhaseV1,
        decision: CapabilityDecision,
        reason: Option<String>,
        terminal_state_set: Option<PhaseStateSetV1>,
        containment: Option<StateSetContainmentDecisionV1>,
    ) -> Result<Self, String> {
        let value = Self {
            phase,
            decision,
            reason,
            terminal_state_set,
            containment,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn supported(
        phase: RouteCapabilityPhaseV1,
        terminal_state_set: Option<PhaseStateSetV1>,
    ) -> Result<Self, String> {
        Self::new(
            phase,
            CapabilityDecision::Supported,
            None,
            terminal_state_set,
            None,
        )
    }

    pub fn unsupported(
        phase: RouteCapabilityPhaseV1,
        reason: impl Into<String>,
    ) -> Result<Self, String> {
        Self::new(
            phase,
            CapabilityDecision::Unsupported,
            Some(reason.into()),
            None,
            None,
        )
    }

    pub fn unknown(
        phase: RouteCapabilityPhaseV1,
        reason: impl Into<String>,
    ) -> Result<Self, String> {
        Self::new(
            phase,
            CapabilityDecision::Unknown,
            Some(reason.into()),
            None,
            None,
        )
    }

    pub fn with_containment(
        mut self,
        containment: StateSetContainmentDecisionV1,
    ) -> Result<Self, String> {
        containment.validate()?;
        self.containment = Some(containment);
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_optional_reason(self.decision, self.reason.as_deref())?;
        if self.decision == CapabilityDecision::Supported && self.terminal_state_set.is_none() {
            return Err("supported phase must carry a terminal state set".to_owned());
        }
        if let Some(state_set) = &self.terminal_state_set {
            state_set.validate()?;
        }
        if let Some(containment) = &self.containment {
            containment.validate()?;
        }
        Ok(())
    }
}

/// Result of the fixed-order composition.  `evaluated_phases` is the prefix
/// through the first decisive phase/containment; later diagnostics are not
/// allowed to replace `first_decisive_reason`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteCapabilityCompositionV1 {
    pub decision: CapabilityDecision,
    pub first_decisive_reason: Option<String>,
    pub evaluated_phases: Vec<RouteCapabilityPhaseDecisionV1>,
}

impl RouteCapabilityCompositionV1 {
    pub fn validate(&self) -> Result<(), String> {
        for phase in &self.evaluated_phases {
            phase.validate()?;
        }
        match (self.decision, self.first_decisive_reason.as_deref()) {
            (CapabilityDecision::Supported, None) => Ok(()),
            (CapabilityDecision::Supported, Some(_)) => {
                Err("supported composition cannot carry a decisive reason".to_owned())
            }
            (_, Some(reason)) => validate_reason_code(reason),
            (_, None) => Err("non-supported composition requires first decisive reason".to_owned()),
        }
    }
}

/// Compose phases in order and stop at the first non-supported phase or
/// containment.  A phase's own decision is authoritative before its
/// containment; `unsupported` and `unknown` are otherwise preserved exactly.
pub fn compose_route_capability(
    phases: &[RouteCapabilityPhaseDecisionV1],
) -> Result<RouteCapabilityCompositionV1, String> {
    let mut evaluated = Vec::new();
    for phase in phases {
        phase.validate()?;
        evaluated.push(phase.clone());
        if phase.decision != CapabilityDecision::Supported {
            let reason = phase.reason.clone().ok_or_else(|| {
                "non-supported phase must carry a first decisive reason".to_owned()
            })?;
            let result = RouteCapabilityCompositionV1 {
                decision: phase.decision,
                first_decisive_reason: Some(reason),
                evaluated_phases: evaluated,
            };
            result.validate()?;
            return Ok(result);
        }
        if let Some(containment) = &phase.containment
            && containment.decision != CapabilityDecision::Supported
        {
            let reason = containment.reason.clone().ok_or_else(|| {
                "non-supported containment must carry a first decisive reason".to_owned()
            })?;
            let result = RouteCapabilityCompositionV1 {
                decision: containment.decision,
                first_decisive_reason: Some(reason),
                evaluated_phases: evaluated,
            };
            result.validate()?;
            return Ok(result);
        }
    }
    let result = RouteCapabilityCompositionV1 {
        decision: CapabilityDecision::Supported,
        first_decisive_reason: None,
        evaluated_phases: evaluated,
    };
    result.validate()?;
    Ok(result)
}

/// Validate the exact fixed phase sequence and its conservative proof
/// payload.  Non-supported phases or containments may end the prefix, but no
/// later phase may be smuggled into a prediction artifact.
pub fn validate_route_capability_phase_sequence(
    phases: &[RouteCapabilityPhaseDecisionV1],
    topology: RouteTopology,
    waypoint_count: usize,
) -> Result<(), String> {
    if (topology == RouteTopology::Direct && waypoint_count != 0)
        || (topology == RouteTopology::Waypoint && waypoint_count == 0)
    {
        return Err("topology and waypoint_count are inconsistent".to_owned());
    }
    let expected = ordered_route_phases(waypoint_count);
    if topology == RouteTopology::Direct {
        if phases.len() != 1
            || phases[0].phase != RouteCapabilityPhaseV1::InitialState
            || phases[0].decision != CapabilityDecision::Unknown
            || phases[0].reason.as_deref() != Some(REASON_UNKNOWN_SCOPE_DIRECT_ROUTE)
            || phases[0].terminal_state_set.is_some()
            || phases[0].containment.is_some()
        {
            return Err(
                "direct-route scope prediction must be one unknown initial-state phase".to_owned(),
            );
        }
        phases[0].validate()?;
        return Ok(());
    }
    if phases.is_empty() || phases.len() > expected.len() {
        return Err("prediction phase sequence has the wrong length".to_owned());
    }
    for (index, phase) in phases.iter().enumerate() {
        if phase.phase != expected[index] {
            return Err(format!(
                "prediction phase {index} is out of order: expected {:?}, found {:?}",
                expected[index], phase.phase
            ));
        }
        phase.validate()?;
        if phase.decision != CapabilityDecision::Supported {
            if index + 1 != phases.len() {
                return Err("non-supported phase must terminate the prediction prefix".to_owned());
            }
            return Ok(());
        }
        let is_final = index + 1 == expected.len();
        match (&phase.containment, is_final) {
            (None, false) => {
                return Err(
                    "supported phase with a downstream phase requires containment".to_owned(),
                );
            }
            (Some(containment), _) if containment.decision != CapabilityDecision::Supported => {
                if index + 1 != phases.len() {
                    return Err(
                        "non-supported containment must terminate the prediction prefix".to_owned(),
                    );
                }
                return Ok(());
            }
            _ => {}
        }
    }
    if phases.len() != expected.len() {
        return Err("supported prediction must carry the complete phase sequence".to_owned());
    }
    Ok(())
}

/// Return the exact required phase order through the final emitted waypoint.
pub fn ordered_route_phases(waypoint_count: usize) -> Vec<RouteCapabilityPhaseV1> {
    let mut phases = vec![
        RouteCapabilityPhaseV1::InitialState,
        RouteCapabilityPhaseV1::PadDeparture,
        RouteCapabilityPhaseV1::Acquisition,
    ];
    for waypoint_index in 0..waypoint_count {
        phases.push(RouteCapabilityPhaseV1::RouteLeg {
            leg_index: waypoint_index,
        });
        phases.push(RouteCapabilityPhaseV1::Handoff { waypoint_index });
    }
    phases
}

/// Common sealed prediction artifact.  It contains no observation, outcome,
/// run, controller, or scenario field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteExecutionPredictionV1 {
    pub schema_version: u32,
    pub status: CapabilityArtifactStatus,
    pub invalid_reason: Option<String>,
    pub input_digest: RouteCapabilityInputDigest,
    pub model_config_digest: RouteCapabilityModelConfigDigest,
    pub capability_artifact_digest: RouteCapabilityArtifactDigest,
    pub topology: RouteTopology,
    pub waypoint_count: usize,
    pub decision: Option<CapabilityDecision>,
    pub first_decisive_reason: Option<String>,
    pub phases: Vec<RouteCapabilityPhaseDecisionV1>,
    pub prediction_digest: RouteExecutionPredictionDigest,
}

#[derive(Serialize)]
struct PredictionDigestMaterial<'a> {
    schema_version: u32,
    status: CapabilityArtifactStatus,
    invalid_reason: &'a Option<String>,
    input_digest: &'a str,
    model_config_digest: &'a str,
    capability_artifact_digest: &'a str,
    topology: RouteTopology,
    waypoint_count: usize,
    decision: &'a Option<CapabilityDecision>,
    first_decisive_reason: &'a Option<String>,
    phases: &'a [RouteCapabilityPhaseDecisionV1],
}

impl RouteExecutionPredictionV1 {
    pub fn complete(
        input: &RouteCapabilityInputV1,
        artifact: &RouteCapabilityArtifactV1,
        phases: Vec<RouteCapabilityPhaseDecisionV1>,
    ) -> Result<Self, String> {
        input.validate()?;
        artifact.validate()?;
        validate_route_capability_phase_sequence(
            &phases,
            input.physical.topology,
            input.physical.waypoints.len(),
        )?;
        let composition = compose_route_capability(&phases)?;
        let mut prediction = Self {
            schema_version: ROUTE_EXECUTION_PREDICTION_SCHEMA_VERSION,
            status: CapabilityArtifactStatus::Complete,
            invalid_reason: None,
            input_digest: input.input_digest.clone(),
            model_config_digest: artifact.model_config_digest.clone(),
            capability_artifact_digest: artifact.capability_artifact_digest.clone(),
            topology: input.physical.topology,
            waypoint_count: input.physical.waypoints.len(),
            decision: Some(composition.decision),
            first_decisive_reason: composition.first_decisive_reason,
            phases,
            prediction_digest: String::new(),
        };
        prediction.prediction_digest = prediction.compute_digest()?;
        prediction.validate_against(input, artifact)?;
        Ok(prediction)
    }

    pub fn unknown_scope_direct_route(
        input: &RouteCapabilityInputV1,
        artifact: &RouteCapabilityArtifactV1,
    ) -> Result<Self, String> {
        input.validate()?;
        if input.physical.topology != RouteTopology::Direct {
            return Err("direct-route prediction helper requires direct topology".to_owned());
        }
        let phases = vec![RouteCapabilityPhaseDecisionV1::unknown(
            RouteCapabilityPhaseV1::InitialState,
            REASON_UNKNOWN_SCOPE_DIRECT_ROUTE,
        )?];
        Self::complete(input, artifact, phases)
    }

    pub fn invalid(
        input_digest: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, String> {
        let reason = reason.into();
        validate_reason_code(&reason)?;
        if !reason.starts_with(REASON_INVALID_ARTIFACT) {
            return Err("invalid predictions require invalid/artifact/* reason".to_owned());
        }
        let mut prediction = Self {
            schema_version: ROUTE_EXECUTION_PREDICTION_SCHEMA_VERSION,
            status: CapabilityArtifactStatus::Invalid,
            invalid_reason: Some(reason),
            input_digest: input_digest.into(),
            model_config_digest: String::new(),
            capability_artifact_digest: String::new(),
            topology: RouteTopology::Direct,
            waypoint_count: 0,
            decision: None,
            first_decisive_reason: None,
            phases: Vec::new(),
            prediction_digest: String::new(),
        };
        prediction.prediction_digest = prediction.compute_digest()?;
        Ok(prediction)
    }

    fn compute_digest(&self) -> Result<String, String> {
        canonical_digest(&PredictionDigestMaterial {
            schema_version: self.schema_version,
            status: self.status,
            invalid_reason: &self.invalid_reason,
            input_digest: &self.input_digest,
            model_config_digest: &self.model_config_digest,
            capability_artifact_digest: &self.capability_artifact_digest,
            topology: self.topology,
            waypoint_count: self.waypoint_count,
            decision: &self.decision,
            first_decisive_reason: &self.first_decisive_reason,
            phases: &self.phases,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != ROUTE_EXECUTION_PREDICTION_SCHEMA_VERSION {
            return Err(format!(
                "prediction schema_version must equal {ROUTE_EXECUTION_PREDICTION_SCHEMA_VERSION}"
            ));
        }
        if self.status == CapabilityArtifactStatus::Invalid {
            if self.decision.is_some() || self.first_decisive_reason.is_some() {
                return Err("invalid prediction cannot carry a decision".to_owned());
            }
            let reason = self
                .invalid_reason
                .as_deref()
                .ok_or_else(|| "invalid prediction requires a reason".to_owned())?;
            validate_reason_code(reason)?;
            if !reason.starts_with(REASON_INVALID_ARTIFACT) {
                return Err("invalid prediction reason must use invalid/artifact/*".to_owned());
            }
        } else {
            if self.invalid_reason.is_some()
                || self.input_digest.trim().is_empty()
                || self.model_config_digest.trim().is_empty()
                || self.capability_artifact_digest.trim().is_empty()
            {
                return Err("complete prediction has incomplete identity fields".to_owned());
            }
            validate_route_capability_phase_sequence(
                &self.phases,
                self.topology,
                self.waypoint_count,
            )?;
            let decision = self
                .decision
                .ok_or_else(|| "complete prediction requires a decision".to_owned())?;
            for phase in &self.phases {
                phase.validate()?;
            }
            let composition = compose_route_capability(&self.phases)?;
            if composition.decision != decision
                || composition.first_decisive_reason != self.first_decisive_reason
            {
                return Err(
                    "prediction decision does not match ordered phase composition".to_owned(),
                );
            }
            if let Some(reason) = &self.first_decisive_reason {
                validate_reason_code(reason)?;
            }
        }
        let expected = self.compute_digest()?;
        if self.prediction_digest != expected {
            return Err(format!(
                "prediction_digest does not match canonical prediction: expected {expected}"
            ));
        }
        Ok(())
    }

    pub fn validate_against(
        &self,
        input: &RouteCapabilityInputV1,
        artifact: &RouteCapabilityArtifactV1,
    ) -> Result<(), String> {
        input.validate()?;
        artifact.validate()?;
        self.validate()?;
        if self.status != CapabilityArtifactStatus::Complete
            || self.input_digest != input.input_digest
            || self.model_config_digest != artifact.model_config_digest
            || self.capability_artifact_digest != artifact.capability_artifact_digest
            || self.topology != input.physical.topology
            || self.waypoint_count != input.physical.waypoints.len()
        {
            return Err("prediction identity does not match input/capability artifact".to_owned());
        }
        Ok(())
    }
}

pub fn prediction_digest(
    prediction: &RouteExecutionPredictionV1,
) -> Result<RouteExecutionPredictionDigest, String> {
    prediction.compute_digest()
}

/// Development-only outcome overlay.  These labels are comparison inputs and
/// are never available to the predictor or capability fitting API.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentScopedOutcomeV1 {
    MaintainedSuccess,
    DiagnosticPass,
    DiagnosticFailure,
}

impl DevelopmentScopedOutcomeV1 {
    pub const fn is_maintained_success(self) -> bool {
        matches!(self, Self::MaintainedSuccess)
    }

    pub const fn is_diagnostic_pass(self) -> bool {
        matches!(self, Self::DiagnosticPass)
    }

    pub const fn is_diagnostic_failure(self) -> bool {
        matches!(self, Self::DiagnosticFailure)
    }
}

/// A single sealed prediction joined to neutral D0 evidence and an outcome
/// overlay.  No comparison field is admitted to `RouteCapabilityInputV1`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentComparisonRowV1 {
    pub row_id: String,
    pub input_digest: RouteCapabilityInputDigest,
    pub evidence_digest: String,
    pub input_status: CapabilityArtifactStatus,
    pub evidence_status: CapabilityArtifactStatus,
    pub capability_status: CapabilityArtifactStatus,
    pub prediction: RouteExecutionPredictionV1,
    pub outcome: DevelopmentScopedOutcomeV1,
    pub topology: RouteTopology,
    pub vehicle_physics_digest: String,
}

impl DevelopmentComparisonRowV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        row_id: impl Into<String>,
        input_digest: impl Into<String>,
        evidence_digest: impl Into<String>,
        prediction: RouteExecutionPredictionV1,
        outcome: DevelopmentScopedOutcomeV1,
        topology: RouteTopology,
        vehicle_physics_digest: impl Into<String>,
    ) -> Result<Self, String> {
        let row = Self {
            row_id: row_id.into(),
            input_digest: input_digest.into(),
            evidence_digest: evidence_digest.into(),
            input_status: CapabilityArtifactStatus::Complete,
            evidence_status: CapabilityArtifactStatus::Complete,
            capability_status: CapabilityArtifactStatus::Complete,
            prediction,
            outcome,
            topology,
            vehicle_physics_digest: vehicle_physics_digest.into(),
        };
        row.validate()?;
        Ok(row)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.row_id.trim().is_empty() {
            return Err("comparison row_id must not be empty".to_owned());
        }
        if self.input_digest.trim().is_empty() || self.evidence_digest.trim().is_empty() {
            return Err("comparison input/evidence digests must not be empty".to_owned());
        }
        if self.vehicle_physics_digest.trim().is_empty() {
            return Err("comparison vehicle_physics_digest must not be empty".to_owned());
        }
        self.prediction.validate()?;
        if self.prediction.input_digest != self.input_digest {
            return Err("comparison input_digest does not match prediction".to_owned());
        }
        if self.prediction.topology != self.topology {
            return Err("comparison topology does not match prediction".to_owned());
        }
        Ok(())
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self.input_status, CapabilityArtifactStatus::Invalid)
            || matches!(self.evidence_status, CapabilityArtifactStatus::Invalid)
            || matches!(self.capability_status, CapabilityArtifactStatus::Invalid)
            || matches!(self.prediction.status, CapabilityArtifactStatus::Invalid)
    }
}

impl InputDigestSource for DevelopmentComparisonRowV1 {
    fn input_digest(&self) -> &str {
        &self.input_digest
    }
}

/// Sealed development comparison report.  Its digest covers the rows and
/// schema but no mutable report rendering metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentComparisonV1 {
    pub schema_version: u32,
    pub rows: Vec<DevelopmentComparisonRowV1>,
    pub comparison_digest: DevelopmentComparisonDigest,
}

#[derive(Serialize)]
struct ComparisonDigestMaterial<'a> {
    schema_version: u32,
    rows: &'a [DevelopmentComparisonRowV1],
}

impl DevelopmentComparisonV1 {
    pub fn new(rows: Vec<DevelopmentComparisonRowV1>) -> Result<Self, String> {
        let report = Self {
            schema_version: DEVELOPMENT_COMPARISON_SCHEMA_VERSION,
            rows,
            comparison_digest: String::new(),
        };
        report.seal()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != DEVELOPMENT_COMPARISON_SCHEMA_VERSION {
            return Err(format!(
                "comparison schema_version must equal {DEVELOPMENT_COMPARISON_SCHEMA_VERSION}"
            ));
        }
        for row in &self.rows {
            row.validate()?;
        }
        let expected = self.compute_digest()?;
        if self.comparison_digest != expected {
            return Err(format!(
                "comparison_digest does not match canonical rows: expected {expected}"
            ));
        }
        Ok(())
    }

    fn compute_digest(&self) -> Result<String, String> {
        canonical_digest(&ComparisonDigestMaterial {
            schema_version: self.schema_version,
            rows: &self.rows,
        })
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.schema_version = DEVELOPMENT_COMPARISON_SCHEMA_VERSION;
        for row in &self.rows {
            row.validate()?;
        }
        self.comparison_digest = self.compute_digest()?;
        self.validate()?;
        Ok(self)
    }

    pub fn evaluate_gate(&self) -> DevelopmentGateReportV1 {
        evaluate_development_gate(self)
    }

    pub fn build_folds(&self) -> Result<DigestGroupedFoldsV1, String> {
        build_digest_grouped_folds(&self.rows)
    }
}

pub fn comparison_digest(
    comparison: &DevelopmentComparisonV1,
) -> Result<DevelopmentComparisonDigest, String> {
    comparison.compute_digest()
}

/// Default finite-corpus advancement criteria from the locked D1a contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentGateCriteriaV1 {
    pub maintained_successes: usize,
    pub diagnostic_failures: usize,
    pub diagnostic_passes: usize,
    pub minimum_supported_diagnostic_passes: usize,
}

impl Default for DevelopmentGateCriteriaV1 {
    fn default() -> Self {
        Self {
            maintained_successes: 36,
            diagnostic_failures: 14,
            diagnostic_passes: 10,
            minimum_supported_diagnostic_passes: 5,
        }
    }
}

/// Per-topology/vehicle diagnostic-pass coverage reported by the gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentStratumGateV1 {
    pub topology: RouteTopology,
    pub vehicle_physics_digest: String,
    pub diagnostic_passes: usize,
    pub supported_diagnostic_passes: usize,
    pub passed: bool,
}

/// Machine-readable development advancement gate result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentGateReportV1 {
    pub criteria: DevelopmentGateCriteriaV1,
    pub passed: bool,
    pub invalid_inputs: usize,
    pub invalid_evidence: usize,
    pub invalid_capabilities: usize,
    pub invalid_predictions: usize,
    pub maintained_successes: usize,
    pub maintained_supported: usize,
    pub diagnostic_failures: usize,
    pub diagnostic_failure_supported: usize,
    pub diagnostic_passes: usize,
    pub diagnostic_pass_supported: usize,
    pub strata: Vec<DevelopmentStratumGateV1>,
    pub failure_reasons: Vec<String>,
}

impl DevelopmentGateReportV1 {
    pub fn digest(&self) -> Result<String, String> {
        canonical_digest(self)
    }
}

/// A small trait lets callers evaluate either a comparison report or a bare
/// row slice without duplicating gate logic.
pub trait DevelopmentComparisonRows {
    fn comparison_rows(&self) -> &[DevelopmentComparisonRowV1];
}

impl DevelopmentComparisonRows for DevelopmentComparisonV1 {
    fn comparison_rows(&self) -> &[DevelopmentComparisonRowV1] {
        &self.rows
    }
}

impl DevelopmentComparisonRows for [DevelopmentComparisonRowV1] {
    fn comparison_rows(&self) -> &[DevelopmentComparisonRowV1] {
        self
    }
}

impl DevelopmentComparisonRows for Vec<DevelopmentComparisonRowV1> {
    fn comparison_rows(&self) -> &[DevelopmentComparisonRowV1] {
        self.as_slice()
    }
}

impl<const N: usize> DevelopmentComparisonRows for [DevelopmentComparisonRowV1; N] {
    fn comparison_rows(&self) -> &[DevelopmentComparisonRowV1] {
        self.as_slice()
    }
}

/// Evaluate the exact locked advancement gate.  Invalid rows never count as
/// support, and all-unknown reports fail both maintained-success and useful
/// diagnostic-pass criteria.
pub fn evaluate_development_gate<R: DevelopmentComparisonRows + ?Sized>(
    rows: &R,
) -> DevelopmentGateReportV1 {
    evaluate_development_gate_with_criteria(rows, DevelopmentGateCriteriaV1::default())
}

pub fn evaluate_development_gate_with_criteria<R: DevelopmentComparisonRows + ?Sized>(
    rows: &R,
    criteria: DevelopmentGateCriteriaV1,
) -> DevelopmentGateReportV1 {
    let rows = rows.comparison_rows();
    let mut report = DevelopmentGateReportV1 {
        criteria,
        passed: false,
        invalid_inputs: 0,
        invalid_evidence: 0,
        invalid_capabilities: 0,
        invalid_predictions: 0,
        maintained_successes: 0,
        maintained_supported: 0,
        diagnostic_failures: 0,
        diagnostic_failure_supported: 0,
        diagnostic_passes: 0,
        diagnostic_pass_supported: 0,
        strata: Vec::new(),
        failure_reasons: Vec::new(),
    };
    let mut strata: BTreeMap<(u8, String), (RouteTopology, usize, usize)> = BTreeMap::new();
    for row in rows {
        if row.input_status == CapabilityArtifactStatus::Invalid {
            report.invalid_inputs += 1;
        }
        if row.evidence_status == CapabilityArtifactStatus::Invalid {
            report.invalid_evidence += 1;
        }
        if row.capability_status == CapabilityArtifactStatus::Invalid {
            report.invalid_capabilities += 1;
        }
        let prediction_already_invalid = row.prediction.status == CapabilityArtifactStatus::Invalid;
        if prediction_already_invalid {
            report.invalid_predictions += 1;
        }
        if row.validate().is_err() {
            if !prediction_already_invalid {
                report.invalid_predictions += 1;
            }
            continue;
        }
        if row.is_invalid() {
            continue;
        }
        let supported = row.prediction.decision == Some(CapabilityDecision::Supported);
        if row.outcome.is_maintained_success() {
            report.maintained_successes += 1;
            if supported {
                report.maintained_supported += 1;
            }
        } else if row.outcome.is_diagnostic_failure() {
            report.diagnostic_failures += 1;
            if supported {
                report.diagnostic_failure_supported += 1;
            }
        } else if row.outcome.is_diagnostic_pass() {
            report.diagnostic_passes += 1;
            let key = (
                topology_rank(row.topology),
                row.vehicle_physics_digest.clone(),
            );
            let entry = strata.entry(key).or_insert((row.topology, 0, 0));
            entry.1 += 1;
            if supported {
                report.diagnostic_pass_supported += 1;
                entry.2 += 1;
            }
        }
    }
    for ((_, vehicle_physics_digest), (topology, diagnostic_passes, supported_diagnostic_passes)) in
        strata
    {
        report.strata.push(DevelopmentStratumGateV1 {
            topology,
            vehicle_physics_digest,
            diagnostic_passes,
            supported_diagnostic_passes,
            passed: supported_diagnostic_passes > 0,
        });
    }
    report.strata.sort_by(|lhs, rhs| {
        topology_rank(lhs.topology)
            .cmp(&topology_rank(rhs.topology))
            .then_with(|| lhs.vehicle_physics_digest.cmp(&rhs.vehicle_physics_digest))
    });
    let mut failures = Vec::new();
    if report.invalid_inputs != 0 {
        failures.push("invalid_inputs".to_owned());
    }
    if report.invalid_evidence != 0 {
        failures.push("invalid_evidence".to_owned());
    }
    if report.invalid_capabilities != 0 {
        failures.push("invalid_capabilities".to_owned());
    }
    if report.invalid_predictions != 0 {
        failures.push("invalid_predictions".to_owned());
    }
    if report.maintained_successes != criteria.maintained_successes {
        failures.push("maintained_success_count".to_owned());
    }
    if report.maintained_supported != criteria.maintained_successes {
        failures.push("maintained_success_support".to_owned());
    }
    if report.diagnostic_failures != criteria.diagnostic_failures {
        failures.push("diagnostic_failure_count".to_owned());
    }
    if report.diagnostic_failure_supported != 0 {
        failures.push("diagnostic_failure_false_accept".to_owned());
    }
    if report.diagnostic_passes != criteria.diagnostic_passes {
        failures.push("diagnostic_pass_count".to_owned());
    }
    if report.diagnostic_pass_supported < criteria.minimum_supported_diagnostic_passes {
        failures.push("diagnostic_pass_support".to_owned());
    }
    if report.strata.iter().any(|stratum| !stratum.passed) {
        failures.push("diagnostic_pass_stratum_coverage".to_owned());
    }
    report.passed = failures.is_empty();
    report.failure_reasons = failures;
    report
}

const fn topology_rank(topology: RouteTopology) -> u8 {
    match topology {
        RouteTopology::Direct => 0,
        RouteTopology::Waypoint => 1,
    }
}

/// Source of the canonical physical input digest used for cross-validation.
pub trait InputDigestSource {
    fn input_digest(&self) -> &str;
}

impl InputDigestSource for RouteCapabilityInputV1 {
    fn input_digest(&self) -> &str {
        &self.input_digest
    }
}

impl InputDigestSource for String {
    fn input_digest(&self) -> &str {
        self
    }
}

impl InputDigestSource for &str {
    fn input_digest(&self) -> &str {
        self
    }
}

impl InputDigestSource for &String {
    fn input_digest(&self) -> &str {
        self.as_str()
    }
}

/// One deterministic leave-one-input-digest-out fold.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DigestGroupedFoldV1 {
    pub excluded_input_digest: String,
    pub fit_row_indices: Vec<usize>,
    pub comparison_row_indices: Vec<usize>,
}

impl DigestGroupedFoldV1 {
    pub fn fit_count(&self) -> usize {
        self.fit_row_indices.len()
    }

    pub fn comparison_count(&self) -> usize {
        self.comparison_row_indices.len()
    }
}

pub const DIGEST_GROUPED_FOLDS_SCHEMA_VERSION: u32 = 1;

/// Complete deterministic fold construction and its uniqueness proof.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DigestGroupedFoldsV1 {
    pub schema_version: u32,
    pub total_row_count: usize,
    pub unique_input_digest_count: usize,
    pub all_input_digests_unique: bool,
    pub row_input_digests: Vec<String>,
    pub folds: Vec<DigestGroupedFoldV1>,
    pub fold_digest: String,
}

impl DigestGroupedFoldsV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != DIGEST_GROUPED_FOLDS_SCHEMA_VERSION {
            return Err(format!(
                "fold schema_version must equal {DIGEST_GROUPED_FOLDS_SCHEMA_VERSION}"
            ));
        }
        if self.total_row_count != self.row_input_digests.len()
            || self.unique_input_digest_count != self.folds.len()
        {
            return Err("fold unique digest count does not match fold count".to_owned());
        }
        if self
            .row_input_digests
            .iter()
            .any(|digest| digest.trim().is_empty())
        {
            return Err("fold row input digests must not be empty".to_owned());
        }
        let mut seen = BTreeSet::new();
        let mut compared = BTreeSet::new();
        for fold in &self.folds {
            if fold.excluded_input_digest.trim().is_empty()
                || !seen.insert(fold.excluded_input_digest.as_str())
            {
                return Err("fold excluded digests must be non-empty and unique".to_owned());
            }
            for index in &fold.comparison_row_indices {
                if *index >= self.total_row_count || !compared.insert(*index) {
                    return Err("comparison fold indices must partition rows".to_owned());
                }
                if self.row_input_digests[*index] != fold.excluded_input_digest {
                    return Err("comparison fold row does not match its excluded digest".to_owned());
                }
                if fold.fit_row_indices.contains(index) {
                    return Err("fit and comparison rows overlap in a fold".to_owned());
                }
            }
            for index in &fold.fit_row_indices {
                if *index >= self.total_row_count {
                    return Err("fit fold index lies outside input rows".to_owned());
                }
                if self.row_input_digests[*index] == fold.excluded_input_digest {
                    return Err("fit fold contains its excluded digest".to_owned());
                }
            }
            let expected_fit = (0..self.total_row_count)
                .filter(|index| !fold.comparison_row_indices.contains(index))
                .collect::<Vec<_>>();
            if fold.fit_row_indices != expected_fit {
                return Err("fit fold indices are not the complement of excluded rows".to_owned());
            }
        }
        if compared.len() != self.total_row_count {
            return Err("comparison fold indices do not cover every row".to_owned());
        }
        let all_unique = self
            .folds
            .iter()
            .all(|fold| fold.comparison_row_indices.len() == 1);
        if self.all_input_digests_unique != all_unique {
            return Err("fold uniqueness flag is inconsistent with memberships".to_owned());
        }
        let mut material = self.clone();
        material.fold_digest.clear();
        if self.fold_digest != canonical_digest(&material)? {
            return Err("fold digest does not match memberships".to_owned());
        }
        Ok(())
    }
}

/// Construct sorted, digest-grouped leave-one-input-digest-out folds.  Every
/// row sharing a physical input digest is excluded together, preventing seed,
/// case-ID, and provenance aliases from crossing the fit/comparison boundary.
pub fn build_digest_grouped_folds<R: InputDigestSource>(
    rows: &[R],
) -> Result<DigestGroupedFoldsV1, String> {
    let mut grouped: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let digest = row.input_digest();
        if digest.trim().is_empty() {
            return Err(format!("row {index} has an empty input digest"));
        }
        grouped.entry(digest.to_owned()).or_default().push(index);
    }
    let mut folds = Vec::with_capacity(grouped.len());
    for (excluded_input_digest, comparison_row_indices) in &grouped {
        let mut fit_row_indices = Vec::with_capacity(rows.len() - comparison_row_indices.len());
        for index in 0..rows.len() {
            if !comparison_row_indices.contains(&index) {
                fit_row_indices.push(index);
            }
        }
        folds.push(DigestGroupedFoldV1 {
            excluded_input_digest: excluded_input_digest.clone(),
            fit_row_indices,
            comparison_row_indices: comparison_row_indices.clone(),
        });
    }
    let row_input_digests = rows
        .iter()
        .map(|row| row.input_digest().to_owned())
        .collect::<Vec<_>>();
    let mut result = DigestGroupedFoldsV1 {
        schema_version: DIGEST_GROUPED_FOLDS_SCHEMA_VERSION,
        total_row_count: rows.len(),
        unique_input_digest_count: grouped.len(),
        all_input_digests_unique: grouped.values().all(|indices| indices.len() == 1),
        row_input_digests,
        folds,
        fold_digest: String::new(),
    };
    result.fold_digest = canonical_digest(&result)?;
    result.validate()?;
    Ok(result)
}

pub fn leave_one_input_digest_out<R: InputDigestSource>(
    rows: &[R],
) -> Result<DigestGroupedFoldsV1, String> {
    build_digest_grouped_folds(rows)
}

impl InputDigestSource for DigestGroupedFoldV1 {
    fn input_digest(&self) -> &str {
        &self.excluded_input_digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pd_core::{
        LandingPadSpec, RoutePlanDiagnostics, RoutePlanningPolicy, RouteTopology,
        TransferRouteSpec, WorldSpec,
    };

    fn request_and_plan() -> (RoutePlanningRequest, RoutePlan) {
        let request = RoutePlanningRequest {
            world: WorldSpec {
                gravity_mps2: 9.81,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-100.0, 0.0), Vec2::new(200.0, 0.0)],
                },
                landing_pads: vec![
                    LandingPadSpec {
                        id: "source".to_owned(),
                        center_x_m: 0.0,
                        surface_y_m: 0.0,
                        width_m: 20.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: 100.0,
                        surface_y_m: 0.0,
                        width_m: 20.0,
                    },
                ],
            },
            vehicle: VehicleSpec {
                geometry: pd_core::VehicleGeometry {
                    hull_width_m: 8.0,
                    hull_height_m: 10.0,
                    touchdown_half_span_m: 4.0,
                    touchdown_base_offset_m: 5.0,
                },
                dry_mass_kg: 7200.0,
                initial_fuel_kg: 6300.0,
                max_fuel_kg: 6300.0,
                max_thrust_n: 200_000.0,
                max_fuel_burn_kgps: 50.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 5.0,
                safe_touchdown_tangential_speed_mps: 5.0,
                safe_touchdown_attitude_error_rad: 0.3,
                safe_touchdown_angular_rate_radps: 0.5,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(0.0, 5.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            policy: RoutePlanningPolicy::default(),
        };
        let geometry = pd_core::normalized_geometry(&request).expect("test geometry");
        let plan = RoutePlan {
            algorithm_id: "test-planner".to_owned(),
            policy: request.policy.clone(),
            request_digest: "request-a".to_owned(),
            plan_digest: "plan-a".to_owned(),
            topology: RouteTopology::Direct,
            route: TransferRouteSpec {
                source_pad_id: "source".to_owned(),
                target_pad_id: "target".to_owned(),
                route_angle_deg: 0.0,
                route_radius_m: 100.0,
                waypoints: Vec::new(),
            },
            normalized_geometry: geometry,
            diagnostics: RoutePlanDiagnostics {
                direct_path_clear: true,
                direct_path_clearance: None,
                route_length_m: 100.0,
                direct_distance_m: 100.0,
                excess_length_m: 0.0,
                peak_extra_loft_m: 0.0,
                minimum_planned_clearance_m: 1.0,
                leg_diagnostics: Vec::new(),
                selected_node_ids: vec!["diagnostic-only".to_owned()],
                safe_profile_points_m: vec![Vec2::new(1.0, 1.0)],
                selected_centerline_m: Vec::new(),
                waypoint_authority: Vec::new(),
            },
        };
        (request, plan)
    }

    fn waypoint_request_and_plan() -> (RoutePlanningRequest, RoutePlan) {
        let (request, mut plan) = request_and_plan();
        plan.topology = RouteTopology::Waypoint;
        plan.route.waypoints = vec![TransferWaypointSpec {
            id: "waypoint-0".to_owned(),
            position_m: Vec2::new(50.0, 5.0),
            handoff_tangent_unit: Some(Vec2::new(1.0, 0.0)),
            capture_radius_m: 5.0,
            max_cross_track_m: 4.0,
            max_outbound_heading_error_rad: 0.3,
            min_outbound_progress_mps: 1.0,
            max_outbound_cross_speed_mps: Some(4.0),
            min_speed_mps: 0.0,
            max_speed_mps: 100.0,
            min_vertical_speed_mps: Some(-5.0),
            max_vertical_speed_mps: Some(5.0),
        }];
        plan.route.route_radius_m = 110.0;
        plan.diagnostics.selected_centerline_m.clear();
        (request, plan)
    }

    fn artifact(input: &RouteCapabilityInputV1) -> RouteCapabilityArtifactV1 {
        RouteCapabilityArtifactV1::new(
            "test-alternative",
            serde_json::json!({"kind": "fixture"}),
            RouteCapabilityConfigurationV1::default(),
            RouteCapabilityDomainV1::for_input(input).expect("test domain"),
            vec!["evidence-a".to_owned()],
        )
        .expect("test artifact")
    }

    fn interval(value: f64) -> FiniteIntervalV1 {
        FiniteIntervalV1::singleton(value).expect("finite interval")
    }

    fn state_set(identity: &str) -> PhaseStateSetV1 {
        PhaseStateSetV1::new(
            identity,
            interval(0.0),
            interval(0.0),
            interval(0.0),
            interval(0.0),
            interval(0.0),
            CircularAttitudeIntervalV1::new(0.0, 0.0).expect("attitude"),
            interval(0.0),
            interval(13_500.0),
            interval(6_300.0),
            None,
        )
        .expect("state set")
    }

    fn supported_phases() -> Vec<RouteCapabilityPhaseDecisionV1> {
        let expected = ordered_route_phases(1);
        let expected_len = expected.len();
        expected
            .into_iter()
            .enumerate()
            .map(|(index, phase)| {
                let decision = RouteCapabilityPhaseDecisionV1::supported(
                    phase,
                    Some(state_set(&format!("terminal-{index}"))),
                )
                .expect("supported phase");
                if index + 1 < expected_len {
                    decision
                        .with_containment(
                            StateSetContainmentDecisionV1::new(
                                format!("containment-{index}"),
                                CapabilityDecision::Supported,
                                None,
                            )
                            .expect("supported containment"),
                        )
                        .expect("containment attached")
                } else {
                    decision
                }
            })
            .collect()
    }

    #[test]
    fn canonical_input_ignores_provenance_and_planner_diagnostics() {
        let (request, mut plan) = request_and_plan();
        let first = build_route_capability_input(&request, &plan).expect("input");
        let first_bytes = first.canonical_physical_bytes().expect("bytes");
        let mut second = first.clone();
        second.provenance.request_digest = "request-b".to_owned();
        second.provenance.route_plan_digest = "plan-b".to_owned();
        second.provenance.source_pad_id = "renamed-source".to_owned();
        plan.algorithm_id = "other-planner".to_owned();
        plan.diagnostics.direct_path_clear = false;
        plan.diagnostics.route_length_m = 999.0;
        plan.diagnostics.selected_node_ids = vec!["changed-ranking".to_owned()];
        plan.diagnostics.safe_profile_points_m = vec![Vec2::new(99.0, 99.0)];
        plan.diagnostics.waypoint_authority = vec![];
        let rebuilt = build_route_capability_input(&request, &plan).expect("rebuilt input");
        assert_eq!(
            first_bytes,
            rebuilt.canonical_physical_bytes().expect("bytes")
        );
        assert_eq!(first.input_digest, rebuilt.input_digest);
        assert_eq!(
            first.input_digest,
            second.physical_digest().expect("digest")
        );

        let mut renamed_request = request.clone();
        renamed_request.source_pad_id = "renamed-source".to_owned();
        renamed_request.world.landing_pads[0].id = "renamed-source".to_owned();
        plan.route.source_pad_id = "renamed-source".to_owned();
        plan.request_digest = "request-c".to_owned();
        plan.plan_digest = "plan-c".to_owned();
        let renamed = build_route_capability_input(&renamed_request, &plan).expect("renamed input");
        assert_eq!(
            first.input_digest,
            renamed.physical_digest().expect("renamed physical digest")
        );
    }

    #[test]
    fn waypoint_ids_are_provenance_only_when_geometry_is_unchanged() {
        let (request, plan) = waypoint_request_and_plan();
        let first = build_route_capability_input(&request, &plan).expect("waypoint input");
        let mut renamed_request = request.clone();
        renamed_request.source_pad_id = "renamed-source".to_owned();
        renamed_request.world.landing_pads[0].id = "renamed-source".to_owned();
        let mut renamed_plan = plan.clone();
        renamed_plan.route.source_pad_id = "renamed-source".to_owned();
        renamed_plan.route.waypoints[0].id = "renamed-waypoint".to_owned();
        renamed_plan.request_digest = "request-renamed".to_owned();
        renamed_plan.plan_digest = "plan-renamed".to_owned();
        let renamed =
            build_route_capability_input(&renamed_request, &renamed_plan).expect("renamed input");
        assert_eq!(
            first.input_digest,
            renamed.physical_digest().expect("renamed physical digest")
        );
        assert_ne!(
            first.provenance.waypoint_ids,
            renamed.provenance.waypoint_ids
        );
    }

    #[test]
    fn physical_mutation_changes_input_digest() {
        let (request, plan) = request_and_plan();
        let mut input = build_route_capability_input(&request, &plan).expect("input");
        let digest = input.input_digest.clone();
        input.physical.gravity_mps2 += 0.01;
        assert_ne!(digest, input.physical_digest().expect("digest"));
    }

    #[test]
    fn physical_policy_dto_excludes_policy_version_identity() {
        let (request, _) = request_and_plan();
        let policy =
            RouteCapabilityPhysicalPolicyV1::from_policy(&request.policy).expect("physical policy");
        let bytes = canonical_serialized_bytes(&policy).expect("policy bytes");
        assert!(
            !String::from_utf8(bytes)
                .expect("policy json")
                .contains("policy_version")
        );
        let first_digest = canonical_digest(&policy).expect("policy digest");
        let equivalent = RouteCapabilityPhysicalPolicyV1 {
            max_waypoints: policy.max_waypoints,
            flight_clearance_margin_m: policy.flight_clearance_margin_m,
            endpoint_transition_m: policy.endpoint_transition_m,
            max_extra_loft_ratio: policy.max_extra_loft_ratio,
            max_continuation_ratio: policy.max_continuation_ratio,
            max_handoff_speed_mps: policy.max_handoff_speed_mps,
            min_handoff_speed_mps: policy.min_handoff_speed_mps,
            min_outbound_progress_mps: policy.min_outbound_progress_mps,
            max_outbound_heading_error_rad: policy.max_outbound_heading_error_rad,
            max_outbound_cross_speed_mps: policy.max_outbound_cross_speed_mps,
        };
        assert_eq!(
            first_digest,
            canonical_digest(&equivalent).expect("policy digest")
        );
    }

    #[test]
    fn negative_vertical_speed_lower_bound_is_valid() {
        let waypoint = RouteCapabilityWaypointV1 {
            position_m: Vec2::new(1.0, 2.0),
            handoff_tangent_unit: Some(Vec2::new(1.0, 0.0)),
            capture_radius_m: 1.0,
            max_cross_track_m: 1.0,
            max_outbound_heading_error_rad: 0.1,
            min_outbound_progress_mps: 1.0,
            max_outbound_cross_speed_mps: None,
            min_speed_mps: 0.0,
            max_speed_mps: 1.0,
            min_vertical_speed_mps: Some(-2.0),
            max_vertical_speed_mps: Some(-1.0),
        };
        assert!(waypoint.validate(0).is_ok());
    }

    #[test]
    fn direct_route_prediction_is_complete_unknown_with_scope_reason() {
        let (request, plan) = request_and_plan();
        let input = build_route_capability_input(&request, &plan).expect("input");
        let artifact = artifact(&input);
        let prediction = RouteExecutionPredictionV1::unknown_scope_direct_route(&input, &artifact)
            .expect("prediction");
        prediction
            .validate_against(&input, &artifact)
            .expect("valid");
        assert_eq!(prediction.status, CapabilityArtifactStatus::Complete);
        assert_eq!(prediction.decision, Some(CapabilityDecision::Unknown));
        assert_eq!(
            prediction.first_decisive_reason.as_deref(),
            Some(REASON_UNKNOWN_SCOPE_DIRECT_ROUTE)
        );
    }

    #[test]
    fn invalid_artifact_digest_tampering_is_rejected() {
        let invalid = RouteCapabilityArtifactV1::invalid("invalid/artifact/malformed_model")
            .expect("invalid artifact");
        invalid.validate().expect("sealed invalid artifact");
        let mut tampered = invalid;
        tampered.capability_artifact_digest.push('x');
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn invalid_prediction_rejects_non_artifact_reason_after_recomputed_digest() {
        let mut prediction =
            RouteExecutionPredictionV1::invalid("input", "invalid/artifact/malformed_prediction")
                .expect("invalid prediction");
        prediction.invalid_reason = Some("unknown/coverage/not_an_invalid_artifact".to_owned());
        prediction.prediction_digest = prediction.compute_digest().expect("prediction digest");
        let encoded = serde_json::to_string(&prediction).expect("prediction json");
        let decoded: RouteExecutionPredictionV1 =
            serde_json::from_str(&encoded).expect("prediction json round trip");
        assert!(decoded.validate().is_err());
    }

    #[test]
    fn supported_phase_proofs_require_reason_free_state_and_containment() {
        let supported = RouteCapabilityPhaseDecisionV1::supported(
            RouteCapabilityPhaseV1::InitialState,
            Some(state_set("proof")),
        )
        .expect("supported phase");
        let with_reason = RouteCapabilityPhaseDecisionV1::new(
            RouteCapabilityPhaseV1::InitialState,
            CapabilityDecision::Supported,
            Some("unknown/coverage/should_not_be_here".to_owned()),
            Some(state_set("proof")),
            None,
        );
        assert!(with_reason.is_err());

        let supported_containment = StateSetContainmentDecisionV1::new(
            "proof-containment",
            CapabilityDecision::Supported,
            Some("unknown/coverage/should_not_be_here".to_owned()),
        );
        assert!(supported_containment.is_err());

        let mut missing_state = supported.clone();
        missing_state.terminal_state_set = None;
        assert!(missing_state.validate().is_err());

        let (request, plan) = waypoint_request_and_plan();
        let input = build_route_capability_input(&request, &plan).expect("waypoint input");
        let artifact = artifact(&input);
        let mut missing_containment = supported_phases();
        missing_containment[0].containment = None;
        assert!(
            RouteExecutionPredictionV1::complete(&input, &artifact, missing_containment).is_err()
        );
    }

    #[test]
    fn prediction_rejects_wrong_out_of_order_and_incomplete_supported_sequences() {
        let (request, plan) = waypoint_request_and_plan();
        let input = build_route_capability_input(&request, &plan).expect("waypoint input");
        let artifact = artifact(&input);
        let full = supported_phases();
        RouteExecutionPredictionV1::complete(&input, &artifact, full.clone())
            .expect("complete sequence");

        let mut out_of_order = full.clone();
        out_of_order.swap(1, 2);
        assert!(RouteExecutionPredictionV1::complete(&input, &artifact, out_of_order).is_err());

        let incomplete = full[..full.len() - 1].to_vec();
        assert!(RouteExecutionPredictionV1::complete(&input, &artifact, incomplete).is_err());

        let mut late_phase = full[..3].to_vec();
        late_phase[2] = RouteCapabilityPhaseDecisionV1::unknown(
            RouteCapabilityPhaseV1::Acquisition,
            "unknown/coverage/missing_training",
        )
        .expect("unknown phase");
        late_phase.push(full[3].clone());
        assert!(RouteExecutionPredictionV1::complete(&input, &artifact, late_phase).is_err());
    }

    #[test]
    fn intervals_and_state_sets_reject_nonfinite_or_reversed_values() {
        assert!(FiniteIntervalV1::new(f64::NAN, 1.0).is_err());
        assert!(FiniteIntervalV1::new(2.0, 1.0).is_err());
        assert!(CircularAttitudeIntervalV1::new(0.0, PI + 0.1).is_err());
        let mut set = state_set("malformed");
        set.mass_kg = FiniteIntervalV1 {
            lower: f64::NEG_INFINITY,
            upper: 1.0,
        };
        assert!(set.validate().is_err());
    }

    #[test]
    fn model_digest_ignores_training_provenance_but_artifact_digest_does_not() {
        let (request, plan) = request_and_plan();
        let input = build_route_capability_input(&request, &plan).expect("input");
        let first = artifact(&input);
        let mut second = first.clone();
        second
            .training_evidence_digests
            .push("evidence-b".to_owned());
        second = second.seal().expect("resealed artifact");
        assert_eq!(first.model_config_digest, second.model_config_digest);
        assert_ne!(
            first.capability_artifact_digest,
            second.capability_artifact_digest
        );
    }

    #[test]
    fn ordered_composition_stops_at_first_non_supported_phase_or_containment() {
        let mut phases = supported_phases();
        phases[2] = RouteCapabilityPhaseDecisionV1::unknown(
            RouteCapabilityPhaseV1::Acquisition,
            "unknown/coverage/missing_training",
        )
        .expect("unknown phase");
        phases[3] = RouteCapabilityPhaseDecisionV1::unsupported(
            RouteCapabilityPhaseV1::RouteLeg { leg_index: 0 },
            "unsupported/physics/late_failure",
        )
        .expect("unsupported phase");
        let result = compose_route_capability(&phases).expect("composition");
        assert_eq!(result.decision, CapabilityDecision::Unknown);
        assert_eq!(result.evaluated_phases.len(), 3);
        assert_eq!(
            result.first_decisive_reason.as_deref(),
            Some("unknown/coverage/missing_training")
        );

        let containment = StateSetContainmentDecisionV1::new(
            "handoff-0",
            CapabilityDecision::Unsupported,
            Some("unsupported/containment/disjoint".to_owned()),
        )
        .expect("containment");
        let phase = RouteCapabilityPhaseDecisionV1::supported(
            RouteCapabilityPhaseV1::Handoff { waypoint_index: 0 },
            Some(state_set("handoff-terminal")),
        )
        .expect("phase")
        .with_containment(containment)
        .expect("containment attached");
        let result = compose_route_capability(&[phase]).expect("composition");
        assert_eq!(result.decision, CapabilityDecision::Unsupported);
        assert_eq!(result.evaluated_phases.len(), 1);
    }

    fn comparison_row(
        input_digest: &str,
        outcome: DevelopmentScopedOutcomeV1,
        decision: CapabilityDecision,
        topology: RouteTopology,
        vehicle_physics_digest: &str,
    ) -> DevelopmentComparisonRowV1 {
        let phases = match decision {
            CapabilityDecision::Supported if topology == RouteTopology::Waypoint => {
                supported_phases()
            }
            CapabilityDecision::Supported => vec![
                RouteCapabilityPhaseDecisionV1::supported(
                    RouteCapabilityPhaseV1::InitialState,
                    Some(state_set("direct-terminal")),
                )
                .expect("supported phase"),
            ],
            CapabilityDecision::Unsupported => RouteCapabilityPhaseDecisionV1::unsupported(
                RouteCapabilityPhaseV1::InitialState,
                "unsupported/physics/test_negative",
            )
            .map(|phase| vec![phase])
            .expect("unsupported phase"),
            CapabilityDecision::Unknown => RouteCapabilityPhaseDecisionV1::unknown(
                RouteCapabilityPhaseV1::InitialState,
                "unknown/coverage/test_unknown",
            )
            .map(|phase| vec![phase])
            .expect("unknown phase"),
        };
        let prediction = RouteExecutionPredictionV1 {
            schema_version: ROUTE_EXECUTION_PREDICTION_SCHEMA_VERSION,
            status: CapabilityArtifactStatus::Complete,
            invalid_reason: None,
            input_digest: input_digest.to_owned(),
            model_config_digest: "model".to_owned(),
            capability_artifact_digest: "artifact".to_owned(),
            topology,
            waypoint_count: match topology {
                RouteTopology::Direct => 0,
                RouteTopology::Waypoint => 1,
            },
            decision: Some(decision),
            first_decisive_reason: match decision {
                CapabilityDecision::Supported => None,
                CapabilityDecision::Unsupported => {
                    Some("unsupported/physics/test_negative".to_owned())
                }
                CapabilityDecision::Unknown => Some("unknown/coverage/test_unknown".to_owned()),
            },
            phases,
            prediction_digest: String::new(),
        };
        let mut prediction = prediction;
        prediction.prediction_digest = prediction.compute_digest().expect("prediction digest");
        DevelopmentComparisonRowV1 {
            row_id: format!("row-{input_digest}"),
            input_digest: input_digest.to_owned(),
            evidence_digest: format!("evidence-{input_digest}"),
            input_status: CapabilityArtifactStatus::Complete,
            evidence_status: CapabilityArtifactStatus::Complete,
            capability_status: CapabilityArtifactStatus::Complete,
            prediction,
            outcome,
            topology,
            vehicle_physics_digest: vehicle_physics_digest.to_owned(),
        }
    }

    #[test]
    fn grouped_folds_keep_digest_aliases_together() {
        let digests = vec!["a".to_owned(), "a".to_owned(), "b".to_owned()];
        let folds = build_digest_grouped_folds(&digests).expect("folds");
        assert_eq!(folds.unique_input_digest_count, 2);
        assert!(!folds.all_input_digests_unique);
        let first = folds
            .folds
            .iter()
            .find(|fold| fold.excluded_input_digest == "a")
            .expect("a fold");
        assert_eq!(first.comparison_row_indices, vec![0, 1]);
        assert!(!first.fit_row_indices.contains(&0));
        assert!(!first.fit_row_indices.contains(&1));
        let mut tampered = folds.clone();
        tampered.row_input_digests[0] = "b".to_owned();
        assert!(tampered.validate().is_err());
        let mut tampered = folds;
        tampered.fold_digest.push('0');
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn all_unknown_and_false_accept_gate_results_fail() {
        let mut rows = Vec::new();
        for index in 0..36 {
            rows.push(comparison_row(
                &format!("m{index}"),
                DevelopmentScopedOutcomeV1::MaintainedSuccess,
                CapabilityDecision::Unknown,
                RouteTopology::Waypoint,
                "light",
            ));
        }
        for index in 0..10 {
            rows.push(comparison_row(
                &format!("p{index}"),
                DevelopmentScopedOutcomeV1::DiagnosticPass,
                CapabilityDecision::Unknown,
                RouteTopology::Waypoint,
                "light",
            ));
        }
        for index in 0..14 {
            rows.push(comparison_row(
                &format!("f{index}"),
                DevelopmentScopedOutcomeV1::DiagnosticFailure,
                CapabilityDecision::Unknown,
                RouteTopology::Waypoint,
                "light",
            ));
        }
        let report = evaluate_development_gate(&rows);
        assert!(!report.passed);
        assert!(
            report
                .failure_reasons
                .contains(&"maintained_success_support".to_owned())
        );
        assert!(
            report
                .failure_reasons
                .contains(&"diagnostic_pass_support".to_owned())
        );

        let false_accept = comparison_row(
            "f0",
            DevelopmentScopedOutcomeV1::DiagnosticFailure,
            CapabilityDecision::Supported,
            RouteTopology::Waypoint,
            "light",
        );
        rows[46].prediction = false_accept.prediction;
        let report = evaluate_development_gate(&rows);
        assert!(!report.passed);
        assert!(
            report
                .failure_reasons
                .contains(&"diagnostic_failure_false_accept".to_owned())
        );
    }

    #[test]
    fn invalid_prediction_is_counted_once() {
        let mut row = comparison_row(
            "invalid-row",
            DevelopmentScopedOutcomeV1::MaintainedSuccess,
            CapabilityDecision::Unknown,
            RouteTopology::Waypoint,
            "light",
        );
        row.prediction.status = CapabilityArtifactStatus::Invalid;
        row.prediction.invalid_reason = Some("invalid/artifact/tampered".to_owned());
        row.prediction.prediction_digest = "tampered".to_owned();
        let report = evaluate_development_gate(&[row]);
        assert_eq!(report.invalid_predictions, 1);
    }

    #[test]
    fn comparison_rejects_topology_disagreement_with_prediction() {
        let mut row = comparison_row(
            "topology-row",
            DevelopmentScopedOutcomeV1::DiagnosticPass,
            CapabilityDecision::Unknown,
            RouteTopology::Waypoint,
            "light",
        );
        row.topology = RouteTopology::Direct;
        assert!(row.validate().is_err());
    }

    #[test]
    fn conforming_synthetic_gate_passes_and_strata_are_required() {
        let mut rows = Vec::new();
        for index in 0..36 {
            rows.push(comparison_row(
                &format!("m{index}"),
                DevelopmentScopedOutcomeV1::MaintainedSuccess,
                CapabilityDecision::Supported,
                RouteTopology::Waypoint,
                "light",
            ));
        }
        for index in 0..5 {
            rows.push(comparison_row(
                &format!("p{index}"),
                DevelopmentScopedOutcomeV1::DiagnosticPass,
                CapabilityDecision::Supported,
                RouteTopology::Waypoint,
                if index == 0 { "light" } else { "heavy" },
            ));
        }
        for index in 5..10 {
            rows.push(comparison_row(
                &format!("p{index}"),
                DevelopmentScopedOutcomeV1::DiagnosticPass,
                CapabilityDecision::Unknown,
                RouteTopology::Waypoint,
                "heavy",
            ));
        }
        for index in 0..14 {
            rows.push(comparison_row(
                &format!("f{index}"),
                DevelopmentScopedOutcomeV1::DiagnosticFailure,
                CapabilityDecision::Unknown,
                RouteTopology::Waypoint,
                "light",
            ));
        }
        let report = evaluate_development_gate(&rows);
        assert!(report.passed, "gate failures: {:?}", report.failure_reasons);
        assert_eq!(report.diagnostic_pass_supported, 5);
        assert!(report.strata.iter().all(|stratum| stratum.passed));
    }
}
