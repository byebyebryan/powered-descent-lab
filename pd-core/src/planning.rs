//! Neutral, serializable contracts shared by waypoint planning and its
//! consumers.
//!
//! The deterministic search itself lives in `pd-plan`.  This module owns the
//! request/policy/result vocabulary, stable rejection mapping, and the exact
//! route-property validator used by generated routes.

use std::{f64::consts::PI, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{
    math::Vec2,
    model::{
        LandingPadSpec, TransferRouteSpec, TransferWaypointSpec, VehicleInitialState, VehicleSpec,
        WorldSpec,
    },
    terrain::{CorridorClearance, CorridorEnvelope, TerrainQueryError},
};

pub const HEIGHTFIELD_VISIBILITY_ALGORITHM_ID: &str = "heightfield_visibility_v1";
pub const ROUTE_PLANNING_POLICY_VERSION: &str = "heightfield_visibility_policy_v1";

/// Setup-time planner compute evidence.  `wall_time_us` is elapsed monotonic
/// wall-clock time spent inside the deterministic `pd_plan::plan` call for a
/// single resolved request; it excludes request construction, simulation,
/// artifact I/O, and report rendering.  This is observational evidence only,
/// so it must not participate in request/plan/cache identity digests.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannerComputeEvidence {
    pub wall_time_us: u64,
}

/// Explicit planner policy.  Defaults are the V1 values in
/// `docs/waypoint_planning.md`; callers persist the complete resolved value in
/// every request and plan artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutePlanningPolicy {
    pub policy_version: String,
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

impl Default for RoutePlanningPolicy {
    fn default() -> Self {
        Self::v1()
    }
}

impl RoutePlanningPolicy {
    pub fn v1() -> Self {
        Self {
            policy_version: ROUTE_PLANNING_POLICY_VERSION.to_owned(),
            max_waypoints: 2,
            flight_clearance_margin_m: 24.0,
            endpoint_transition_m: 96.0,
            max_extra_loft_ratio: 0.45,
            max_continuation_ratio: 0.75,
            max_handoff_speed_mps: 130.0,
            min_handoff_speed_mps: 10.0,
            min_outbound_progress_mps: 8.0,
            max_outbound_heading_error_rad: 0.35,
            max_outbound_cross_speed_mps: 20.0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.policy_version != ROUTE_PLANNING_POLICY_VERSION {
            return Err(format!(
                "policy_version must equal {ROUTE_PLANNING_POLICY_VERSION}"
            ));
        }
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
        if self.endpoint_transition_m <= 0.0 {
            return Err("endpoint_transition_m must be positive".to_owned());
        }
        if self.endpoint_transition_m > 96.0 {
            return Err("endpoint_transition_m must be <= 96m for planner V1".to_owned());
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

/// Owned setup-time snapshot.  It intentionally carries no controller config,
/// scenario labels, random seed behavior, or simulation-time state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutePlanningRequest {
    pub world: WorldSpec,
    pub vehicle: VehicleSpec,
    pub initial_state: VehicleInitialState,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub policy: RoutePlanningPolicy,
}

impl RoutePlanningRequest {
    pub fn validate(&self) -> Result<(), PlanningValidationError> {
        self.world
            .validate()
            .map_err(PlanningValidationError::InvalidRequest)?;
        self.vehicle
            .validate()
            .map_err(PlanningValidationError::InvalidRequest)?;
        self.initial_state
            .validate()
            .map_err(PlanningValidationError::InvalidRequest)?;
        self.policy
            .validate()
            .map_err(PlanningValidationError::InvalidRequest)?;
        if self.source_pad_id.trim().is_empty() || self.target_pad_id.trim().is_empty() {
            return Err(PlanningValidationError::InvalidRequest(
                "source_pad_id and target_pad_id must not be empty".to_owned(),
            ));
        }
        if self.source_pad_id == self.target_pad_id {
            return Err(PlanningValidationError::InvalidRequest(
                "source_pad_id and target_pad_id must differ".to_owned(),
            ));
        }
        if self.world.landing_pad(&self.source_pad_id).is_none()
            || self.world.landing_pad(&self.target_pad_id).is_none()
        {
            return Err(PlanningValidationError::InvalidRequest(
                "source and target pad IDs must resolve in world.landing_pads".to_owned(),
            ));
        }
        validate_request_geometry(self)?;
        Ok(())
    }

    pub fn source_pad(&self) -> Option<&LandingPadSpec> {
        self.world.landing_pad(&self.source_pad_id)
    }

    pub fn target_pad(&self) -> Option<&LandingPadSpec> {
        self.world.landing_pad(&self.target_pad_id)
    }
}

// Serialized world coordinates must agree to nanometre-scale tolerance at
// pad contact.  This is intentionally tighter than planner clearance
// margins: it only guards canonical pose/surface identity, not flight safety.
const PLANNING_GEOMETRY_TOLERANCE_M: f64 = 1.0e-9;

fn validate_request_geometry(
    request: &RoutePlanningRequest,
) -> Result<(), PlanningValidationError> {
    let terrain = &request.world.terrain;
    let points = terrain.points();
    let domain_min_x_m = points[0].x;
    let domain_max_x_m = points[points.len() - 1].x;
    for pad in &request.world.landing_pads {
        let left = pad.center_x_m - pad.half_width_m();
        let right = pad.center_x_m + pad.half_width_m();
        if left < domain_min_x_m - PLANNING_GEOMETRY_TOLERANCE_M
            || right > domain_max_x_m + PLANNING_GEOMETRY_TOLERANCE_M
        {
            return Err(PlanningValidationError::InvalidRequest(format!(
                "landing pad '{}' footprint lies outside terrain domain",
                pad.id
            )));
        }
        for x_m in [left, right] {
            let terrain_y = terrain
                .sample_height_strict(x_m)
                .map_err(|error| PlanningValidationError::InvalidRequest(error.to_string()))?;
            if (terrain_y - pad.surface_y_m).abs() > PLANNING_GEOMETRY_TOLERANCE_M {
                return Err(PlanningValidationError::InvalidRequest(format!(
                    "landing pad '{}' is not on a flat terrain surface",
                    pad.id
                )));
            }
        }
        for point in points {
            if point.x >= left - PLANNING_GEOMETRY_TOLERANCE_M
                && point.x <= right + PLANNING_GEOMETRY_TOLERANCE_M
                && (point.y - pad.surface_y_m).abs() > PLANNING_GEOMETRY_TOLERANCE_M
            {
                return Err(PlanningValidationError::InvalidRequest(format!(
                    "landing pad '{}' footprint contains non-flat terrain",
                    pad.id
                )));
            }
        }
    }
    let source = request.source_pad().expect("source pad validated");
    let expected_source_position = Vec2::new(
        source.center_x_m,
        source.surface_y_m + request.vehicle.geometry.touchdown_base_offset_m,
    );
    if (request.initial_state.position_m - expected_source_position).length()
        > PLANNING_GEOMETRY_TOLERANCE_M
    {
        return Err(PlanningValidationError::InvalidRequest(
            "initial state must match the source pad touchdown reference".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningValidationError {
    InvalidRequest(String),
    UnsupportedGeometry(String),
}

impl fmt::Display for PlanningValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::UnsupportedGeometry(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for PlanningValidationError {}

/// Stable planner rejection vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningRejectionCode {
    InvalidRequest,
    UnsupportedGeometry,
    LoftLimitExceeded,
    RouteComplexityExceeded,
    InsufficientAuthority,
    NoRouteWithinPolicy,
}

impl PlanningRejectionCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::UnsupportedGeometry => "unsupported_geometry",
            Self::LoftLimitExceeded => "loft_limit_exceeded",
            Self::RouteComplexityExceeded => "route_complexity_exceeded",
            Self::InsufficientAuthority => "insufficient_authority",
            Self::NoRouteWithinPolicy => "no_route_within_policy",
        }
    }
}

impl fmt::Display for PlanningRejectionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A typed, serializable planner rejection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanningRejection {
    pub code: PlanningRejectionCode,
    pub message: String,
}

impl PlanningRejection {
    pub fn new(code: PlanningRejectionCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub const fn rejection_code(&self) -> PlanningRejectionCode {
        self.code
    }
}

impl fmt::Display for PlanningRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PlanningRejection {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteTopology {
    Direct,
    Waypoint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRouteGeometry {
    pub horizontal_sign: i8,
    pub direct_horizontal_span_m: f64,
    pub direct_distance_m: f64,
    pub route_angle_rad: f64,
    pub route_angle_deg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RouteLegDiagnostics {
    pub leg_index: usize,
    pub start_m: Vec2,
    pub end_m: Vec2,
    pub route_length_m: f64,
    pub minimum_clearance_m: f64,
    pub clearance: CorridorClearance,
    pub stopping_speed_cap_mps: Option<f64>,
    pub turn_speed_cap_mps: Option<f64>,
    pub handoff_speed_cap_mps: Option<f64>,
    pub available_distance_m: Option<f64>,
    pub stopping_ratio_at_handoff: Option<f64>,
    pub turn_ratio_at_handoff: Option<f64>,
}

#[derive(Deserialize)]
struct RouteLegDiagnosticsFields {
    leg_index: usize,
    start_m: Vec2,
    end_m: Vec2,
    route_length_m: f64,
    minimum_clearance_m: f64,
    clearance: CorridorClearance,
    stopping_speed_cap_mps: Option<f64>,
    turn_speed_cap_mps: Option<f64>,
    handoff_speed_cap_mps: Option<f64>,
    available_distance_m: Option<f64>,
    stopping_ratio_at_handoff: Option<f64>,
    turn_ratio_at_handoff: Option<f64>,
}

impl<'de> Deserialize<'de> for RouteLegDiagnostics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = RouteLegDiagnosticsFields::deserialize(deserializer)?;
        // `serde_json` represents `Some(+infinity)` as null. A leg with
        // waypoint authority has an available-distance and turn-ratio value;
        // in that context null is the canonical unbounded turn-speed cap.
        // Legs without waypoint authority retain the ordinary None meaning.
        let turn_speed_cap_mps = match fields.turn_speed_cap_mps {
            Some(value) => Some(value),
            None if fields.available_distance_m.is_some()
                && fields.turn_ratio_at_handoff.is_some() =>
            {
                Some(f64::INFINITY)
            }
            None => None,
        };
        Ok(Self {
            leg_index: fields.leg_index,
            start_m: fields.start_m,
            end_m: fields.end_m,
            route_length_m: fields.route_length_m,
            minimum_clearance_m: fields.minimum_clearance_m,
            clearance: fields.clearance,
            stopping_speed_cap_mps: fields.stopping_speed_cap_mps,
            turn_speed_cap_mps,
            handoff_speed_cap_mps: fields.handoff_speed_cap_mps,
            available_distance_m: fields.available_distance_m,
            stopping_ratio_at_handoff: fields.stopping_ratio_at_handoff,
            turn_ratio_at_handoff: fields.turn_ratio_at_handoff,
        })
    }
}

/// The exact planner authority calculation uses positive infinity when a
/// straight handoff has no turn-speed bound.  JSON has no representation for
/// infinity, so serde_json emits that value as `null`.  Keep serialization
/// unchanged while accepting that one intentional sentinel on input.
fn deserialize_turn_speed_cap<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<f64>::deserialize(deserializer)?;
    match value {
        None => Ok(f64::INFINITY),
        Some(value) if value.is_finite() && value > 0.0 => Ok(value),
        Some(value) => Err(de::Error::custom(format!(
            "turn speed cap must be positive finite or null, got {value}"
        ))),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointAuthorityDiagnostics {
    pub waypoint_index: usize,
    pub available_inbound_distance_m: f64,
    pub available_outbound_distance_m: f64,
    pub inbound_stopping_speed_cap_mps: f64,
    pub outbound_stopping_speed_cap_mps: f64,
    #[serde(deserialize_with = "deserialize_turn_speed_cap")]
    pub inbound_turn_speed_cap_mps: f64,
    #[serde(deserialize_with = "deserialize_turn_speed_cap")]
    pub outbound_turn_speed_cap_mps: f64,
    pub handoff_speed_cap_mps: f64,
    pub inbound_stopping_ratio_at_handoff: f64,
    pub outbound_stopping_ratio_at_handoff: f64,
    pub inbound_turn_ratio_at_handoff: f64,
    pub outbound_turn_ratio_at_handoff: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutePlanDiagnostics {
    pub direct_path_clear: bool,
    pub direct_path_clearance: Option<CorridorClearance>,
    pub route_length_m: f64,
    pub direct_distance_m: f64,
    pub excess_length_m: f64,
    pub peak_extra_loft_m: f64,
    pub minimum_planned_clearance_m: f64,
    pub leg_diagnostics: Vec<RouteLegDiagnostics>,
    pub selected_node_ids: Vec<String>,
    pub safe_profile_points_m: Vec<Vec2>,
    pub selected_centerline_m: Vec<Vec2>,
    pub waypoint_authority: Vec<WaypointAuthorityDiagnostics>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutePlan {
    pub algorithm_id: String,
    pub policy: RoutePlanningPolicy,
    pub request_digest: String,
    pub plan_digest: String,
    pub topology: RouteTopology,
    pub route: TransferRouteSpec,
    pub normalized_geometry: NormalizedRouteGeometry,
    pub diagnostics: RoutePlanDiagnostics,
}

impl RoutePlan {
    pub fn transfer_route(&self) -> &TransferRouteSpec {
        &self.route
    }

    pub fn waypoints(&self) -> &[TransferWaypointSpec] {
        &self.route.waypoints
    }
}

/// Result of validating a concrete route against a planning request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteValidation {
    pub topology: RouteTopology,
    pub route_length_m: f64,
    pub peak_extra_loft_m: f64,
    pub minimum_planned_clearance_m: f64,
    pub leg_diagnostics: Vec<RouteLegDiagnostics>,
    pub selected_centerline_m: Vec<Vec2>,
    pub waypoint_authority: Vec<WaypointAuthorityDiagnostics>,
}

/// Errors emitted by the shared route validator.  Planner code maps these to
/// stable rejection codes; callers can also use this contract directly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RouteValidationError {
    InvalidRequest(String),
    UnsupportedGeometry(String),
    TerrainQuery(String),
    LoftLimitExceeded(String),
    RouteComplexityExceeded(String),
    InsufficientAuthority(String),
    NoRouteWithinPolicy(String),
}

impl fmt::Display for RouteValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidRequest(message)
            | Self::UnsupportedGeometry(message)
            | Self::TerrainQuery(message)
            | Self::LoftLimitExceeded(message)
            | Self::RouteComplexityExceeded(message)
            | Self::InsufficientAuthority(message)
            | Self::NoRouteWithinPolicy(message) => message,
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RouteValidationError {}

impl RouteValidationError {
    pub const fn rejection_code(&self) -> PlanningRejectionCode {
        match self {
            Self::InvalidRequest(_) | Self::TerrainQuery(_) => {
                PlanningRejectionCode::InvalidRequest
            }
            Self::UnsupportedGeometry(_) => PlanningRejectionCode::UnsupportedGeometry,
            Self::LoftLimitExceeded(_) => PlanningRejectionCode::LoftLimitExceeded,
            Self::RouteComplexityExceeded(_) => PlanningRejectionCode::RouteComplexityExceeded,
            Self::InsufficientAuthority(_) => PlanningRejectionCode::InsufficientAuthority,
            Self::NoRouteWithinPolicy(_) => PlanningRejectionCode::NoRouteWithinPolicy,
        }
    }
}

pub fn compute_waypoint_authority(
    request: &RoutePlanningRequest,
    previous: Vec2,
    waypoint: Vec2,
    next: Vec2,
    capture_radius_m: f64,
) -> Result<WaypointAuthorityDiagnostics, RouteValidationError> {
    if [previous, waypoint, next]
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err(RouteValidationError::InvalidRequest(
            "waypoint authority points must be finite".to_owned(),
        ));
    }
    if !capture_radius_m.is_finite() || capture_radius_m <= 0.0 {
        return Err(RouteValidationError::InvalidRequest(
            "waypoint capture radius must be positive and finite".to_owned(),
        ));
    }
    let initial_mass = request.vehicle.dry_mass_kg + request.vehicle.initial_fuel_kg;
    let net_acceleration = request.vehicle.max_thrust_n / initial_mass - request.world.gravity_mps2;
    if !net_acceleration.is_finite() || net_acceleration <= 0.0 {
        return Err(RouteValidationError::InsufficientAuthority(
            "gravity-taxed initial authority is not positive".to_owned(),
        ));
    }
    let inbound_distance = (waypoint - previous).length();
    let outbound_distance = (next - waypoint).length();
    if inbound_distance <= 1.0e-12 || outbound_distance <= 1.0e-12 {
        return Err(RouteValidationError::UnsupportedGeometry(
            "waypoint authority legs must have non-zero length".to_owned(),
        ));
    }
    let available_inbound = inbound_distance - capture_radius_m;
    let available_outbound = outbound_distance - capture_radius_m;
    if available_inbound <= 0.0 || available_outbound <= 0.0 {
        return Err(RouteValidationError::InsufficientAuthority(
            "waypoint capture radius consumes available authority distance".to_owned(),
        ));
    }
    let inbound_unit = (waypoint - previous) * (1.0 / inbound_distance);
    let outbound_unit = (next - waypoint) * (1.0 / outbound_distance);
    let deflection = inbound_unit
        .x
        .mul_add(outbound_unit.x, inbound_unit.y * outbound_unit.y)
        .clamp(-1.0, 1.0)
        .acos();
    let stopping_cap = |available: f64| {
        (2.0 * net_acceleration * request.policy.max_continuation_ratio * available).sqrt()
    };
    let turn_cap = |available: f64| {
        let sin_half = (deflection * 0.5).sin();
        if sin_half <= 1.0e-12 {
            f64::INFINITY
        } else {
            (request.policy.max_continuation_ratio * net_acceleration * available
                / (2.0 * sin_half))
                .sqrt()
        }
    };
    let inbound_stopping = stopping_cap(available_inbound);
    let outbound_stopping = stopping_cap(available_outbound);
    let inbound_turn = turn_cap(available_inbound);
    let outbound_turn = turn_cap(available_outbound);
    let handoff_cap = request
        .policy
        .max_handoff_speed_mps
        .min(inbound_stopping)
        .min(outbound_stopping)
        .min(inbound_turn)
        .min(outbound_turn);
    if !handoff_cap.is_finite() || handoff_cap < request.policy.min_handoff_speed_mps {
        return Err(RouteValidationError::InsufficientAuthority(
            "no handoff speed satisfies conservative stopping and turn authority".to_owned(),
        ));
    }
    let stopping_ratio =
        |speed: f64, available: f64| speed * speed / (2.0 * net_acceleration * available);
    let turn_ratio = |speed: f64, available: f64| {
        2.0 * speed * speed * (deflection * 0.5).sin() / (net_acceleration * available)
    };
    Ok(WaypointAuthorityDiagnostics {
        waypoint_index: 0,
        available_inbound_distance_m: available_inbound,
        available_outbound_distance_m: available_outbound,
        inbound_stopping_speed_cap_mps: inbound_stopping,
        outbound_stopping_speed_cap_mps: outbound_stopping,
        inbound_turn_speed_cap_mps: inbound_turn,
        outbound_turn_speed_cap_mps: outbound_turn,
        handoff_speed_cap_mps: handoff_cap,
        inbound_stopping_ratio_at_handoff: stopping_ratio(handoff_cap, available_inbound),
        outbound_stopping_ratio_at_handoff: stopping_ratio(handoff_cap, available_outbound),
        inbound_turn_ratio_at_handoff: turn_ratio(handoff_cap, available_inbound),
        outbound_turn_ratio_at_handoff: turn_ratio(handoff_cap, available_outbound),
    })
}

/// Validate route shape, exact terrain corridor, loft, and conservative
/// gravity-taxed authority policy properties for every leg and waypoint.
pub fn validate_route(
    request: &RoutePlanningRequest,
    route: &TransferRouteSpec,
) -> Result<RouteValidation, RouteValidationError> {
    request.validate().map_err(|error| match error {
        PlanningValidationError::InvalidRequest(message) => {
            RouteValidationError::InvalidRequest(message)
        }
        PlanningValidationError::UnsupportedGeometry(message) => {
            RouteValidationError::UnsupportedGeometry(message)
        }
    })?;
    route
        .validate()
        .map_err(RouteValidationError::InvalidRequest)?;
    if route.source_pad_id != request.source_pad_id || route.target_pad_id != request.target_pad_id
    {
        return Err(RouteValidationError::InvalidRequest(
            "route source/target IDs must match planning request".to_owned(),
        ));
    }
    let geometry = normalized_geometry(request)?;
    let source = request
        .source_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("source pad not found".to_owned()))?;
    let target = request
        .target_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("target pad not found".to_owned()))?;
    let max_waypoints = usize::from(request.policy.max_waypoints);
    if route.waypoints.len() > max_waypoints {
        return Err(RouteValidationError::RouteComplexityExceeded(format!(
            "route has {} waypoints but policy allows {max_waypoints}",
            route.waypoints.len()
        )));
    }
    for waypoint in &route.waypoints {
        if waypoint.max_outbound_heading_error_rad
            > request.policy.max_outbound_heading_error_rad + 1.0e-9
        {
            return Err(RouteValidationError::InvalidRequest(
                "waypoint heading envelope exceeds planning policy".to_owned(),
            ));
        }
        if waypoint.min_outbound_progress_mps + 1.0e-9 < request.policy.min_outbound_progress_mps {
            return Err(RouteValidationError::InvalidRequest(
                "waypoint outbound-progress envelope is weaker than planning policy".to_owned(),
            ));
        }
        if waypoint
            .max_outbound_cross_speed_mps
            .is_none_or(|limit| limit > request.policy.max_outbound_cross_speed_mps + 1.0e-9)
        {
            return Err(RouteValidationError::InvalidRequest(
                "waypoint cross-speed envelope must be explicit and within planning policy"
                    .to_owned(),
            ));
        }
        if waypoint.min_speed_mps + 1.0e-9 < request.policy.min_handoff_speed_mps
            || waypoint.max_speed_mps > request.policy.max_handoff_speed_mps + 1.0e-9
        {
            return Err(RouteValidationError::InvalidRequest(
                "waypoint speed envelope is outside planning policy".to_owned(),
            ));
        }
    }
    let expected_angle = geometry.route_angle_deg;
    let expected_radius = geometry.direct_distance_m;
    if (route.route_angle_deg - expected_angle).abs() > 1.0e-9
        || (route.route_radius_m - expected_radius).abs() > 1.0e-9
    {
        return Err(RouteValidationError::InvalidRequest(
            "route angle/radius must be derived from pad centers".to_owned(),
        ));
    }

    let direction = f64::from(geometry.horizontal_sign);
    let source_point = Vec2::new(
        source.center_x_m,
        source.surface_y_m + request.vehicle.geometry.touchdown_base_offset_m,
    );
    let target_point = Vec2::new(
        target.center_x_m,
        target.surface_y_m + request.vehicle.geometry.touchdown_base_offset_m,
    );
    let mut world_points = Vec::with_capacity(route.waypoints.len() + 2);
    world_points.push(source_point);
    world_points.extend(route.waypoints.iter().map(|waypoint| waypoint.position_m));
    world_points.push(target_point);
    let raw_normalized_points: Vec<Vec2> = world_points
        .iter()
        .map(|point| Vec2::new(direction * (point.x - source.center_x_m), point.y))
        .collect();
    for pair in raw_normalized_points.windows(2) {
        if pair[1].x <= pair[0].x {
            return Err(RouteValidationError::UnsupportedGeometry(
                "route legs must strictly increase normalized horizontal progress".to_owned(),
            ));
        }
    }

    let (profile, free_span) = build_endpoint_profile(request, geometry.direct_horizontal_span_m)?;
    if free_span <= 0.0 {
        return Err(RouteValidationError::UnsupportedGeometry(
            "source and target pad footprints overlap".to_owned(),
        ));
    }
    let shaped_points = endpoint_shaped_centerline(
        request,
        &geometry,
        &profile,
        &raw_normalized_points[1..raw_normalized_points.len() - 1],
    )?;
    let waypoint_shape_indices = route
        .waypoints
        .iter()
        .map(|waypoint| {
            let normalized = Vec2::new(
                direction * (waypoint.position_m.x - source.center_x_m),
                waypoint.position_m.y,
            );
            shaped_points
                .iter()
                .position(|point| {
                    (point.x - normalized.x).abs() <= 1.0e-9
                        && (point.y - normalized.y).abs() <= 1.0e-9
                })
                .ok_or_else(|| {
                    RouteValidationError::UnsupportedGeometry(
                        "waypoint was not retained in shaped centerline".to_owned(),
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (waypoint_index, waypoint) in route.waypoints.iter().enumerate() {
        let shaped_index = waypoint_shape_indices[waypoint_index];
        let inbound = unit_vector(shaped_points[shaped_index] - shaped_points[shaped_index - 1])
            .ok_or_else(|| {
                RouteValidationError::UnsupportedGeometry(format!(
                    "waypoint {waypoint_index} inbound leg has zero length"
                ))
            })?;
        let outbound = unit_vector(shaped_points[shaped_index + 1] - shaped_points[shaped_index])
            .ok_or_else(|| {
            RouteValidationError::UnsupportedGeometry(format!(
                "waypoint {waypoint_index} outbound leg has zero length"
            ))
        })?;
        let bisector = unit_vector(inbound + outbound).unwrap_or(outbound);
        let tangent = waypoint.handoff_tangent_unit.ok_or_else(|| {
            RouteValidationError::InvalidRequest(format!(
                "waypoint {waypoint_index} handoff tangent must be explicit"
            ))
        })?;
        let normalized_tangent = Vec2::new(direction * tangent.x, tangent.y);
        if (normalized_tangent - bisector).length() > 1.0e-6 {
            return Err(RouteValidationError::InvalidRequest(format!(
                "waypoint {waypoint_index} handoff tangent must match the route bisector"
            )));
        }
    }
    let loft_limit = request.policy.max_extra_loft_ratio * geometry.direct_distance_m;
    let mut route_length_m = 0.0;
    let mut peak_extra_loft_m: f64 = 0.0;
    let mut minimum_clearance_m = f64::INFINITY;
    let mut leg_diagnostics = Vec::with_capacity(shaped_points.len() - 1);
    let mut waypoint_capture = Vec::with_capacity(route.waypoints.len());
    let mut waypoint_authority = Vec::with_capacity(route.waypoints.len());

    for waypoint in &route.waypoints {
        waypoint_capture.push((waypoint.position_m, waypoint.capture_radius_m));
    }
    for pair in waypoint_capture.windows(2) {
        if (pair[1].0 - pair[0].0).length() <= pair[0].1 + pair[1].1 {
            return Err(RouteValidationError::NoRouteWithinPolicy(
                "waypoint capture regions overlap".to_owned(),
            ));
        }
    }

    for (waypoint_index, waypoint) in route.waypoints.iter().enumerate() {
        let shaped_index = waypoint_shape_indices[waypoint_index];
        let mut authority = compute_waypoint_authority(
            request,
            shaped_points[shaped_index - 1],
            shaped_points[shaped_index],
            shaped_points[shaped_index + 1],
            waypoint.capture_radius_m,
        )?;
        authority.waypoint_index = waypoint_index;
        if waypoint.max_speed_mps > authority.handoff_speed_cap_mps + 1.0e-9 {
            return Err(RouteValidationError::InsufficientAuthority(format!(
                "waypoint {waypoint_index} max speed exceeds conservative authority cap"
            )));
        }
        waypoint_authority.push(authority);
    }

    for (leg_index, pair) in shaped_points.windows(2).enumerate() {
        let start = pair[0];
        let end = pair[1];
        let leg_length = (end - start).length();
        route_length_m += leg_length;
        let leg_loft = peak_extra_loft(
            start,
            end,
            source_point.y,
            target_point.y,
            geometry.direct_horizontal_span_m,
        );
        peak_extra_loft_m = peak_extra_loft_m.max(leg_loft);
        if leg_loft > loft_limit {
            return Err(RouteValidationError::LoftLimitExceeded(format!(
                "leg {leg_index} exceeds extra-loft limit {loft_limit}m"
            )));
        }

        let mut clearance = None;
        for window in profile.breakpoints_between(start.x, end.x) {
            let t0 = if (end.x - start.x).abs() <= f64::EPSILON {
                0.0
            } else {
                (window.0 - start.x) / (end.x - start.x)
            };
            let t1 = if (end.x - start.x).abs() <= f64::EPSILON {
                1.0
            } else {
                (window.1 - start.x) / (end.x - start.x)
            };
            let a = t0.clamp(0.0, 1.0);
            let b = t1.clamp(0.0, 1.0);
            if b - a <= 1.0e-12 {
                continue;
            }
            let segment_start = interpolate(start, end, a);
            let segment_end = interpolate(start, end, b);
            let (start_envelope, end_envelope) = (
                profile.envelope_at(segment_start.x),
                profile.envelope_at(segment_end.x),
            );
            let segment_clearance = request
                .world
                .terrain
                .exact_corridor_clearance(
                    denormalize_point(segment_start, source.center_x_m, geometry.horizontal_sign),
                    denormalize_point(segment_end, source.center_x_m, geometry.horizontal_sign),
                    start_envelope,
                    end_envelope,
                )
                .map_err(|error| match error {
                    TerrainQueryError::DomainOverrun { .. } => {
                        RouteValidationError::UnsupportedGeometry(error.to_string())
                    }
                    _ => RouteValidationError::TerrainQuery(error.to_string()),
                })?;
            minimum_clearance_m = minimum_clearance_m.min(segment_clearance.minimum_clearance_m);
            if clearance
                .as_ref()
                .is_none_or(|current: &CorridorClearance| {
                    segment_clearance.worst_residual.residual_m > current.worst_residual.residual_m
                })
            {
                clearance = Some(segment_clearance);
            }
        }
        let clearance = clearance.ok_or_else(|| {
            RouteValidationError::TerrainQuery("route leg has no profile interval".to_owned())
        })?;
        if !clearance.clear {
            return Err(RouteValidationError::NoRouteWithinPolicy(format!(
                "route leg {leg_index} intersects terrain at x={}m",
                clearance.worst_residual.terrain_position_m.x
            )));
        }
        let inbound_authority = waypoint_shape_indices
            .iter()
            .position(|index| *index == leg_index + 1)
            .and_then(|index| waypoint_authority.get(index));
        let outbound_authority = waypoint_shape_indices
            .iter()
            .position(|index| *index == leg_index)
            .and_then(|index| waypoint_authority.get(index));
        let stopping_speed_cap_mps = match (inbound_authority, outbound_authority) {
            (Some(inbound), Some(outbound)) => Some(
                inbound
                    .inbound_stopping_speed_cap_mps
                    .min(outbound.outbound_stopping_speed_cap_mps),
            ),
            (Some(inbound), None) => Some(inbound.inbound_stopping_speed_cap_mps),
            (None, Some(outbound)) => Some(outbound.outbound_stopping_speed_cap_mps),
            (None, None) => None,
        };
        let turn_speed_cap_mps = match (inbound_authority, outbound_authority) {
            (Some(inbound), Some(outbound)) => Some(
                inbound
                    .inbound_turn_speed_cap_mps
                    .min(outbound.outbound_turn_speed_cap_mps),
            ),
            (Some(inbound), None) => Some(inbound.inbound_turn_speed_cap_mps),
            (None, Some(outbound)) => Some(outbound.outbound_turn_speed_cap_mps),
            (None, None) => None,
        };
        let handoff_speed_cap_mps = match (inbound_authority, outbound_authority) {
            (Some(inbound), Some(outbound)) => Some(
                inbound
                    .handoff_speed_cap_mps
                    .min(outbound.handoff_speed_cap_mps),
            ),
            (Some(inbound), None) => Some(inbound.handoff_speed_cap_mps),
            (None, Some(outbound)) => Some(outbound.handoff_speed_cap_mps),
            (None, None) => None,
        };
        let stopping_ratio_at_handoff = match (inbound_authority, outbound_authority) {
            (Some(inbound), Some(outbound)) => Some(
                inbound
                    .inbound_stopping_ratio_at_handoff
                    .max(outbound.outbound_stopping_ratio_at_handoff),
            ),
            (Some(inbound), None) => Some(inbound.inbound_stopping_ratio_at_handoff),
            (None, Some(outbound)) => Some(outbound.outbound_stopping_ratio_at_handoff),
            (None, None) => None,
        };
        let turn_ratio_at_handoff = match (inbound_authority, outbound_authority) {
            (Some(inbound), Some(outbound)) => Some(
                inbound
                    .inbound_turn_ratio_at_handoff
                    .max(outbound.outbound_turn_ratio_at_handoff),
            ),
            (Some(inbound), None) => Some(inbound.inbound_turn_ratio_at_handoff),
            (None, Some(outbound)) => Some(outbound.outbound_turn_ratio_at_handoff),
            (None, None) => None,
        };
        let available_distance_m = match (inbound_authority, outbound_authority) {
            (Some(inbound), Some(outbound)) => Some(
                inbound
                    .available_inbound_distance_m
                    .min(outbound.available_outbound_distance_m),
            ),
            (Some(inbound), None) => Some(inbound.available_inbound_distance_m),
            (None, Some(outbound)) => Some(outbound.available_outbound_distance_m),
            (None, None) => None,
        };
        leg_diagnostics.push(RouteLegDiagnostics {
            leg_index,
            start_m: denormalize_point(start, source.center_x_m, geometry.horizontal_sign),
            end_m: denormalize_point(end, source.center_x_m, geometry.horizontal_sign),
            route_length_m: leg_length,
            minimum_clearance_m: clearance.minimum_clearance_m,
            clearance,
            stopping_speed_cap_mps,
            turn_speed_cap_mps,
            handoff_speed_cap_mps,
            available_distance_m,
            stopping_ratio_at_handoff,
            turn_ratio_at_handoff,
        });
    }

    let selected_centerline_m = shaped_points
        .iter()
        .copied()
        .map(|point| denormalize_point(point, source.center_x_m, geometry.horizontal_sign))
        .collect();
    Ok(RouteValidation {
        topology: if route.waypoints.is_empty() {
            RouteTopology::Direct
        } else {
            RouteTopology::Waypoint
        },
        route_length_m,
        peak_extra_loft_m,
        minimum_planned_clearance_m: minimum_clearance_m,
        leg_diagnostics,
        selected_centerline_m,
        waypoint_authority,
    })
}

/// Build the normalized safety profile used by both validator and planner.
/// This is public so pd-plan can construct candidate nodes without duplicating
/// endpoint taper semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct SafetyProfile {
    /// The centerline x at which the contact footprint has cleared the source
    /// pad and source taper may begin.
    pub source_transition_start_m: f64,
    /// The centerline x at which the source taper reaches the full envelope.
    pub source_transition_end_m: f64,
    /// The centerline x at which the target taper leaves the full envelope.
    pub target_transition_start_m: f64,
    /// The centerline x at which the contact footprint reaches the target pad
    /// and the target taper is complete.
    pub target_transition_end_m: f64,
    pub horizontal_span_m: f64,
    pub full_envelope: CorridorEnvelope,
    pub contact_envelope: CorridorEnvelope,
}

impl SafetyProfile {
    pub fn breakpoints(&self) -> [f64; 6] {
        [
            0.0,
            self.source_transition_start_m,
            self.source_transition_end_m,
            self.target_transition_start_m,
            self.target_transition_end_m,
            self.horizontal_span_m,
        ]
    }

    pub fn breakpoints_between(&self, start_m: f64, end_m: f64) -> Vec<(f64, f64)> {
        let low = start_m.min(end_m);
        let high = start_m.max(end_m);
        let mut cuts = vec![low, high];
        for breakpoint in self.breakpoints() {
            if breakpoint > low + 1.0e-12 && breakpoint < high - 1.0e-12 {
                cuts.push(breakpoint);
            }
        }
        cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        cuts.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-12);
        cuts.windows(2).map(|pair| (pair[0], pair[1])).collect()
    }

    pub fn envelope_at(&self, normalized_x_m: f64) -> CorridorEnvelope {
        let x = normalized_x_m.clamp(0.0, self.horizontal_span_m);
        if x <= self.source_transition_start_m {
            return self.contact_envelope;
        }
        if x < self.source_transition_end_m {
            return taper(
                self.contact_envelope,
                self.full_envelope,
                fraction(
                    x,
                    self.source_transition_start_m,
                    self.source_transition_end_m,
                ),
            );
        }
        if x <= self.target_transition_start_m {
            return self.full_envelope;
        }
        if x < self.target_transition_end_m {
            return taper(
                self.full_envelope,
                self.contact_envelope,
                fraction(
                    x,
                    self.target_transition_start_m,
                    self.target_transition_end_m,
                ),
            );
        }
        self.contact_envelope
    }
}

pub fn normalized_geometry(
    request: &RoutePlanningRequest,
) -> Result<NormalizedRouteGeometry, RouteValidationError> {
    let source = request
        .source_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("source pad not found".to_owned()))?;
    let target = request
        .target_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("target pad not found".to_owned()))?;
    let dx = target.center_x_m - source.center_x_m;
    let dy = target.surface_y_m - source.surface_y_m;
    if dx.abs() <= 1.0e-9 {
        return Err(RouteValidationError::UnsupportedGeometry(
            "source and target pads have insufficient horizontal separation".to_owned(),
        ));
    }
    let horizontal_sign = if dx.is_sign_positive() { 1 } else { -1 };
    let direct_horizontal_span_m = dx.abs();
    let direct_distance_m = dx.hypot(dy);
    Ok(NormalizedRouteGeometry {
        horizontal_sign,
        direct_horizontal_span_m,
        direct_distance_m,
        route_angle_rad: dy.atan2(dx.abs()),
        route_angle_deg: dy.atan2(dx.abs()).to_degrees(),
    })
}

pub fn build_endpoint_profile(
    request: &RoutePlanningRequest,
    horizontal_span_m: f64,
) -> Result<(SafetyProfile, f64), RouteValidationError> {
    let source = request
        .source_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("source pad not found".to_owned()))?;
    let target = request
        .target_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("target pad not found".to_owned()))?;
    let full_extent = request
        .vehicle
        .geometry
        .hull_width_m
        .hypot(request.vehicle.geometry.hull_height_m)
        * 0.5
        + request.policy.flight_clearance_margin_m;
    let contact = CorridorEnvelope::new(
        request.vehicle.geometry.touchdown_half_span_m,
        request.vehicle.geometry.touchdown_base_offset_m,
    );
    // Keep the contact envelope until its horizontal footprint has cleared
    // each pad.  The usable span is therefore between these expanded
    // contact-footprint boundaries, not merely between pad edges.
    let source_transition_start = source.half_width_m() + contact.horizontal_extent_m;
    let target_transition_end =
        horizontal_span_m - target.half_width_m() - contact.horizontal_extent_m;
    let free_span = target_transition_end - source_transition_start;
    if free_span <= 0.0 {
        return Err(RouteValidationError::UnsupportedGeometry(
            "source and target contact footprints leave no usable transition span".to_owned(),
        ));
    }
    let transition = request.policy.endpoint_transition_m.min(free_span * 0.25);
    let full = CorridorEnvelope::new(full_extent, full_extent);
    Ok((
        SafetyProfile {
            source_transition_start_m: source_transition_start,
            source_transition_end_m: source_transition_start + transition,
            target_transition_start_m: target_transition_end - transition,
            target_transition_end_m: target_transition_end,
            horizontal_span_m,
            full_envelope: full,
            contact_envelope: contact,
        },
        free_span,
    ))
}

/// Build the canonical normalized centerline used for exact endpoint safety.
///
/// The implicit egress/ingress pieces keep the vehicle at each pad's contact
/// reference until its contact footprint has cleared that pad, then blend to
/// the original pad-center touchdown chord through the full-envelope middle.
/// Emitted waypoint nodes must therefore lie strictly inside the full-envelope
/// interval; the implicit points are diagnostics, not route waypoints.
pub fn endpoint_shaped_centerline(
    request: &RoutePlanningRequest,
    geometry: &NormalizedRouteGeometry,
    profile: &SafetyProfile,
    waypoints_normalized: &[Vec2],
) -> Result<Vec<Vec2>, RouteValidationError> {
    let source = request
        .source_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("source pad not found".to_owned()))?;
    let target = request
        .target_pad()
        .ok_or_else(|| RouteValidationError::InvalidRequest("target pad not found".to_owned()))?;
    let source_y = source.surface_y_m + request.vehicle.geometry.touchdown_base_offset_m;
    let target_y = target.surface_y_m + request.vehicle.geometry.touchdown_base_offset_m;
    let chord_y = |x: f64| lerp(source_y, target_y, x / geometry.direct_horizontal_span_m);
    let mut points = vec![
        Vec2::new(0.0, source_y),
        Vec2::new(profile.source_transition_start_m, source_y),
        Vec2::new(
            profile.source_transition_end_m,
            chord_y(profile.source_transition_end_m),
        ),
    ];
    for (index, waypoint) in waypoints_normalized.iter().enumerate() {
        if !waypoint.x.is_finite() || !waypoint.y.is_finite() {
            return Err(RouteValidationError::InvalidRequest(format!(
                "waypoint {index} must be finite"
            )));
        }
        if waypoint.x <= profile.source_transition_end_m + 1.0e-9
            || waypoint.x >= profile.target_transition_start_m - 1.0e-9
        {
            return Err(RouteValidationError::UnsupportedGeometry(format!(
                "waypoint {index} must lie strictly inside full-envelope route progress"
            )));
        }
        points.push(*waypoint);
    }
    points.extend([
        Vec2::new(
            profile.target_transition_start_m,
            chord_y(profile.target_transition_start_m),
        ),
        Vec2::new(profile.target_transition_end_m, target_y),
        Vec2::new(profile.horizontal_span_m, target_y),
    ]);
    points.sort_by(|lhs, rhs| lhs.x.total_cmp(&rhs.x));
    points.dedup_by(|lhs, rhs| (lhs.x - rhs.x).abs() <= 1.0e-9 && (lhs.y - rhs.y).abs() <= 1.0e-9);
    Ok(points)
}

fn taper(start: CorridorEnvelope, end: CorridorEnvelope, t: f64) -> CorridorEnvelope {
    CorridorEnvelope::new(
        lerp(start.horizontal_extent_m, end.horizontal_extent_m, t),
        lerp(start.vertical_extent_m, end.vertical_extent_m, t),
    )
}

fn fraction(value: f64, start: f64, end: f64) -> f64 {
    if (end - start).abs() <= f64::EPSILON {
        1.0
    } else {
        ((value - start) / (end - start)).clamp(0.0, 1.0)
    }
}

fn interpolate(start: Vec2, end: Vec2, t: f64) -> Vec2 {
    Vec2::new(lerp(start.x, end.x, t), lerp(start.y, end.y, t))
}

fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + ((end - start) * t)
}

fn denormalize_point(point: Vec2, source_x_m: f64, horizontal_sign: i8) -> Vec2 {
    Vec2::new(source_x_m + (point.x * f64::from(horizontal_sign)), point.y)
}

fn unit_vector(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-12).then(|| vector * (1.0 / length))
}

fn peak_extra_loft(
    start: Vec2,
    end: Vec2,
    source_y: f64,
    target_y: f64,
    direct_horizontal_span_m: f64,
) -> f64 {
    // The route and direct chord are linear on each leg, so the difference is
    // affine and its maximum is attained at a leg endpoint.
    let direct_y_start = lerp(source_y, target_y, start.x / direct_horizontal_span_m);
    let direct_y_end = lerp(source_y, target_y, end.x / direct_horizontal_span_m);
    (start.y - direct_y_start)
        .max(end.y - direct_y_end)
        .max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        math::Vec2,
        model::{LandingPadSpec, VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec},
        terrain::{CorridorResidual, TerrainDefinition},
    };

    fn request(source_x_m: f64, target_x_m: f64) -> RoutePlanningRequest {
        RoutePlanningRequest {
            world: WorldSpec {
                gravity_mps2: 9.8,
                terrain: TerrainDefinition::Heightfield {
                    points_m: vec![Vec2::new(-200.0, 100.0), Vec2::new(400.0, 100.0)],
                },
                landing_pads: vec![
                    LandingPadSpec {
                        id: "source".to_owned(),
                        center_x_m: source_x_m,
                        surface_y_m: 100.0,
                        width_m: 20.0,
                    },
                    LandingPadSpec {
                        id: "target".to_owned(),
                        center_x_m: target_x_m,
                        surface_y_m: 100.0,
                        width_m: 20.0,
                    },
                ],
            },
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 10.0,
                    hull_height_m: 10.0,
                    touchdown_half_span_m: 5.0,
                    touchdown_base_offset_m: 5.0,
                },
                dry_mass_kg: 1.0,
                initial_fuel_kg: 1.0,
                max_fuel_kg: 2.0,
                max_thrust_n: 100.0,
                max_fuel_burn_kgps: 1.0,
                min_throttle_frac: 0.0,
                max_rotation_rate_radps: 1.0,
                safe_touchdown_normal_speed_mps: 1.0,
                safe_touchdown_tangential_speed_mps: 1.0,
                safe_touchdown_attitude_error_rad: 1.0,
                safe_touchdown_angular_rate_radps: 1.0,
            },
            initial_state: VehicleInitialState {
                position_m: Vec2::new(source_x_m, 105.0),
                velocity_mps: Vec2::new(0.0, 0.0),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
            },
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            policy: RoutePlanningPolicy::default(),
        }
    }

    fn one_waypoint_route(
        request: &RoutePlanningRequest,
        position: Vec2,
        _tangent: Vec2,
    ) -> TransferRouteSpec {
        let geometry = normalized_geometry(request).unwrap();
        let (profile, _) =
            build_endpoint_profile(request, geometry.direct_horizontal_span_m).unwrap();
        let normalized_position = Vec2::new(
            f64::from(geometry.horizontal_sign)
                * (position.x - request.source_pad().unwrap().center_x_m),
            position.y,
        );
        let shaped =
            endpoint_shaped_centerline(request, &geometry, &profile, &[normalized_position])
                .unwrap();
        let index = shaped
            .iter()
            .position(|point| *point == normalized_position)
            .unwrap();
        let normalized_tangent =
            unit(unit(shaped[index] - shaped[index - 1]) + unit(shaped[index + 1] - shaped[index]));
        let tangent = Vec2::new(
            f64::from(geometry.horizontal_sign) * normalized_tangent.x,
            normalized_tangent.y,
        );
        TransferRouteSpec {
            source_pad_id: request.source_pad_id.clone(),
            target_pad_id: request.target_pad_id.clone(),
            route_angle_deg: geometry.route_angle_deg,
            route_radius_m: geometry.direct_distance_m,
            waypoints: vec![TransferWaypointSpec {
                id: "w0".to_owned(),
                position_m: position,
                handoff_tangent_unit: Some(tangent),
                capture_radius_m: 35.0,
                max_cross_track_m: 35.0,
                max_outbound_heading_error_rad: 0.35,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: Some(20.0),
                min_speed_mps: 10.0,
                max_speed_mps: 10.0,
                min_vertical_speed_mps: None,
                max_vertical_speed_mps: None,
            }],
        }
    }

    fn unit(vector: Vec2) -> Vec2 {
        vector * (1.0 / vector.length())
    }

    fn set_canonical_tangents(request: &RoutePlanningRequest, route: &mut TransferRouteSpec) {
        let geometry = normalized_geometry(request).unwrap();
        let (profile, _) =
            build_endpoint_profile(request, geometry.direct_horizontal_span_m).unwrap();
        let normalized = route
            .waypoints
            .iter()
            .map(|waypoint| {
                Vec2::new(
                    f64::from(geometry.horizontal_sign)
                        * (waypoint.position_m.x - request.source_pad().unwrap().center_x_m),
                    waypoint.position_m.y,
                )
            })
            .collect::<Vec<_>>();
        let shaped = endpoint_shaped_centerline(request, &geometry, &profile, &normalized).unwrap();
        for (index, waypoint) in route.waypoints.iter_mut().enumerate() {
            let shape_index = shaped
                .iter()
                .position(|point| *point == normalized[index])
                .unwrap();
            let normalized_tangent = unit(
                unit(shaped[shape_index] - shaped[shape_index - 1])
                    + unit(shaped[shape_index + 1] - shaped[shape_index]),
            );
            waypoint.handoff_tangent_unit = Some(Vec2::new(
                f64::from(geometry.horizontal_sign) * normalized_tangent.x,
                normalized_tangent.y,
            ));
        }
    }

    #[test]
    fn endpoint_taper_is_contact_full_contact() {
        let request = request(0.0, 300.0);
        let geometry = normalized_geometry(&request).unwrap();
        let (profile, free_span) =
            build_endpoint_profile(&request, geometry.direct_horizontal_span_m).unwrap();
        assert_eq!(profile.envelope_at(0.0), profile.contact_envelope);
        assert_eq!(
            profile.envelope_at(profile.source_transition_start_m),
            profile.contact_envelope
        );
        assert_eq!(
            profile.envelope_at(profile.source_transition_end_m),
            profile.full_envelope
        );
        assert_eq!(
            profile.envelope_at(profile.target_transition_start_m),
            profile.full_envelope
        );
        assert_eq!(
            profile.envelope_at(profile.target_transition_end_m),
            profile.contact_envelope
        );
        assert_eq!(
            profile.envelope_at(geometry.direct_horizontal_span_m),
            profile.contact_envelope
        );
        assert_eq!(
            profile.source_transition_start_m,
            request.world.landing_pads[0].half_width_m()
                + request.vehicle.geometry.touchdown_half_span_m
        );
        assert_eq!(
            profile.target_transition_end_m,
            geometry.direct_horizontal_span_m
                - request.world.landing_pads[1].half_width_m()
                - request.vehicle.geometry.touchdown_half_span_m
        );
        let expected_transition = free_span.min(96.0).min(free_span * 0.25);
        assert!(
            (profile.source_transition_end_m
                - profile.source_transition_start_m
                - expected_transition)
                .abs()
                < 1.0e-9
        );
    }

    #[test]
    fn expanded_contact_footprints_can_leave_no_transition_span() {
        let request = request(0.0, 30.0);
        assert!(matches!(
            build_endpoint_profile(&request, 30.0),
            Err(RouteValidationError::UnsupportedGeometry(_))
        ));
    }

    #[test]
    fn reverse_geometry_normalizes_leftward_route() {
        let request = request(300.0, 0.0);
        let geometry = normalized_geometry(&request).unwrap();
        assert_eq!(geometry.horizontal_sign, -1);
        assert_eq!(geometry.direct_horizontal_span_m, 300.0);
        assert_eq!(geometry.route_angle_deg, 0.0);
    }

    #[test]
    fn zero_and_reversed_route_legs_are_rejected() {
        let request = request(0.0, 300.0);
        let route = TransferRouteSpec {
            source_pad_id: "source".to_owned(),
            target_pad_id: "target".to_owned(),
            route_angle_deg: 0.0,
            route_radius_m: 300.0,
            waypoints: vec![TransferWaypointSpec {
                id: "w0".to_owned(),
                position_m: Vec2::new(0.0, 150.0),
                handoff_tangent_unit: None,
                capture_radius_m: 35.0,
                max_cross_track_m: 35.0,
                max_outbound_heading_error_rad: 0.35,
                min_outbound_progress_mps: 8.0,
                max_outbound_cross_speed_mps: Some(20.0),
                min_speed_mps: 10.0,
                max_speed_mps: 100.0,
                min_vertical_speed_mps: None,
                max_vertical_speed_mps: None,
            }],
        };
        assert!(matches!(
            validate_route(&request, &route),
            Err(RouteValidationError::UnsupportedGeometry(_))
        ));
    }

    #[test]
    fn shared_validator_rejects_blocked_source_and_final_legs() {
        let source_request = request(0.0, 300.0);
        let mut source_request = source_request;
        source_request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-200.0, 100.0),
                Vec2::new(40.0, 100.0),
                Vec2::new(50.0, 190.0),
                Vec2::new(60.0, 100.0),
                Vec2::new(400.0, 100.0),
            ],
        };
        let source_blocked = one_waypoint_route(
            &source_request,
            Vec2::new(120.0, 150.0),
            Vec2::new(1.0, 0.0),
        );
        let source_error = validate_route(&source_request, &source_blocked).unwrap_err();
        assert!(
            matches!(source_error, RouteValidationError::NoRouteWithinPolicy(_)),
            "{source_error:?}"
        );
        assert!(source_error.to_string().contains("leg"));

        let mut final_request = request(0.0, 300.0);
        final_request.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-200.0, 100.0),
                Vec2::new(10.0, 100.0),
                Vec2::new(200.0, 130.0),
                Vec2::new(210.0, 100.0),
                Vec2::new(290.0, 100.0),
                Vec2::new(400.0, 100.0),
            ],
        };
        let previous = Vec2::new(0.0, 105.0);
        let waypoint = Vec2::new(100.0, 161.0);
        let next = Vec2::new(300.0, 105.0);
        let tangent = unit(unit(waypoint - previous) + unit(next - waypoint));
        let final_blocked = one_waypoint_route(&final_request, waypoint, tangent);
        let final_error = validate_route(&final_request, &final_blocked).unwrap_err();
        assert!(matches!(
            final_error,
            RouteValidationError::NoRouteWithinPolicy(_)
        ));
        assert!(final_error.to_string().contains("leg"));
    }

    #[test]
    fn authority_reports_gravity_taxed_caps_and_ratios() {
        let request = request(0.0, 300.0);
        let previous = Vec2::new(0.0, 105.0);
        let waypoint = Vec2::new(100.0, 150.0);
        let next = Vec2::new(300.0, 105.0);
        let diagnostics =
            compute_waypoint_authority(&request, previous, waypoint, next, 35.0).unwrap();
        let net_acceleration = 100.0 / 2.0 - 9.8;
        let available = (waypoint - previous).length() - 35.0;
        let expected_stop =
            (2.0 * net_acceleration * request.policy.max_continuation_ratio * available).sqrt();
        assert!((diagnostics.inbound_stopping_speed_cap_mps - expected_stop).abs() < 1.0e-9);
        assert!(diagnostics.inbound_stopping_ratio_at_handoff <= 0.75 + 1.0e-9);
        assert!(diagnostics.inbound_turn_ratio_at_handoff <= 0.75 + 1.0e-9);

        let mut infeasible = request;
        infeasible.vehicle.max_thrust_n = 19.7;
        assert!(matches!(
            compute_waypoint_authority(&infeasible, previous, waypoint, next, 35.0),
            Err(RouteValidationError::InsufficientAuthority(_))
        ));
    }

    #[test]
    fn infinity_turn_authority_json_round_trips_without_byte_change() {
        let request = request(0.0, 300.0);
        let authority = compute_waypoint_authority(
            &request,
            Vec2::new(0.0, 105.0),
            Vec2::new(100.0, 105.0),
            Vec2::new(300.0, 105.0),
            35.0,
        )
        .unwrap();
        assert!(authority.inbound_turn_speed_cap_mps.is_infinite());
        assert!(authority.outbound_turn_speed_cap_mps.is_infinite());
        let bytes = serde_json::to_vec(&authority).unwrap();
        let decoded: WaypointAuthorityDiagnostics = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, authority);
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
        let mut negative = serde_json::to_value(&authority).unwrap();
        negative["inbound_turn_speed_cap_mps"] = serde_json::json!(-1.0);
        assert!(serde_json::from_value::<WaypointAuthorityDiagnostics>(negative).is_err());

        let mut route = one_waypoint_route(&request, Vec2::new(100.0, 150.0), Vec2::new(1.0, 0.0));
        route.waypoints[0].max_speed_mps = 10.0;
        set_canonical_tangents(&request, &mut route);
        let geometry = normalized_geometry(&request).unwrap();
        let clearance = CorridorClearance {
            clear: true,
            minimum_clearance_m: 1.0,
            worst_residual: CorridorResidual {
                residual_m: -1.0,
                centerline_position_m: Vec2::new(0.0, 0.0),
                terrain_position_m: Vec2::new(0.0, 0.0),
                terrain_segment_index: 0,
                required_envelope_y_m: 0.0,
                centerline_y_m: 1.0,
                vertical_extent_m: 1.0,
            },
        };
        let infinity_leg = RouteLegDiagnostics {
            leg_index: 0,
            start_m: Vec2::new(0.0, 1.0),
            end_m: Vec2::new(1.0, 1.0),
            route_length_m: 1.0,
            minimum_clearance_m: 1.0,
            clearance: clearance.clone(),
            stopping_speed_cap_mps: Some(1.0),
            turn_speed_cap_mps: Some(f64::INFINITY),
            handoff_speed_cap_mps: Some(1.0),
            available_distance_m: Some(1.0),
            stopping_ratio_at_handoff: Some(0.0),
            turn_ratio_at_handoff: Some(0.0),
        };
        let ordinary_leg = RouteLegDiagnostics {
            leg_index: 1,
            start_m: Vec2::new(1.0, 1.0),
            end_m: Vec2::new(2.0, 1.0),
            route_length_m: 1.0,
            minimum_clearance_m: 1.0,
            clearance,
            stopping_speed_cap_mps: None,
            turn_speed_cap_mps: None,
            handoff_speed_cap_mps: None,
            available_distance_m: None,
            stopping_ratio_at_handoff: None,
            turn_ratio_at_handoff: None,
        };
        let plan = RoutePlan {
            algorithm_id: HEIGHTFIELD_VISIBILITY_ALGORITHM_ID.to_owned(),
            policy: request.policy.clone(),
            request_digest: "request".to_owned(),
            plan_digest: "plan".to_owned(),
            topology: RouteTopology::Waypoint,
            route,
            normalized_geometry: geometry.clone(),
            diagnostics: RoutePlanDiagnostics {
                direct_path_clear: false,
                direct_path_clearance: None,
                route_length_m: geometry.direct_distance_m,
                direct_distance_m: geometry.direct_distance_m,
                excess_length_m: 0.0,
                peak_extra_loft_m: 0.0,
                minimum_planned_clearance_m: 1.0,
                leg_diagnostics: vec![infinity_leg, ordinary_leg],
                selected_node_ids: vec!["w0".to_owned()],
                safe_profile_points_m: Vec::new(),
                selected_centerline_m: vec![Vec2::new(0.0, 105.0), Vec2::new(300.0, 105.0)],
                waypoint_authority: vec![authority],
            },
        };
        let plan_bytes = serde_json::to_vec(&plan).unwrap();
        let decoded_plan: RoutePlan = serde_json::from_slice(&plan_bytes).unwrap();
        assert_eq!(decoded_plan, plan);
        assert_eq!(
            decoded_plan.diagnostics.leg_diagnostics[0].turn_speed_cap_mps,
            Some(f64::INFINITY)
        );
        assert_eq!(
            decoded_plan.diagnostics.leg_diagnostics[1].turn_speed_cap_mps,
            None
        );
        assert_eq!(serde_json::to_vec(&decoded_plan).unwrap(), plan_bytes);
    }

    #[test]
    fn shared_validator_rejects_capture_overlap_and_weak_handoff_contracts() {
        let request = request(0.0, 300.0);
        let geometry = normalized_geometry(&request).unwrap();
        let mut route = TransferRouteSpec {
            source_pad_id: request.source_pad_id.clone(),
            target_pad_id: request.target_pad_id.clone(),
            route_angle_deg: geometry.route_angle_deg,
            route_radius_m: geometry.direct_distance_m,
            waypoints: vec![
                TransferWaypointSpec {
                    id: "w0".to_owned(),
                    position_m: Vec2::new(100.0, 150.0),
                    handoff_tangent_unit: Some(Vec2::new(0.9775, 0.2108)),
                    capture_radius_m: 35.0,
                    max_cross_track_m: 35.0,
                    max_outbound_heading_error_rad: 0.35,
                    min_outbound_progress_mps: 8.0,
                    max_outbound_cross_speed_mps: Some(20.0),
                    min_speed_mps: 10.0,
                    max_speed_mps: 10.0,
                    min_vertical_speed_mps: None,
                    max_vertical_speed_mps: None,
                },
                TransferWaypointSpec {
                    id: "w1".to_owned(),
                    position_m: Vec2::new(150.0, 150.0),
                    handoff_tangent_unit: Some(Vec2::new(0.9897, -0.1439)),
                    capture_radius_m: 35.0,
                    max_cross_track_m: 35.0,
                    max_outbound_heading_error_rad: 0.35,
                    min_outbound_progress_mps: 8.0,
                    max_outbound_cross_speed_mps: Some(20.0),
                    min_speed_mps: 10.0,
                    max_speed_mps: 10.0,
                    min_vertical_speed_mps: None,
                    max_vertical_speed_mps: None,
                },
            ],
        };
        set_canonical_tangents(&request, &mut route);
        let overlap_error = validate_route(&request, &route).unwrap_err();
        assert!(
            matches!(overlap_error, RouteValidationError::NoRouteWithinPolicy(_)),
            "{overlap_error:?}"
        );

        route.waypoints.truncate(1);
        route.waypoints[0].handoff_tangent_unit = None;
        assert!(matches!(
            validate_route(&request, &route),
            Err(RouteValidationError::InvalidRequest(_))
        ));
        set_canonical_tangents(&request, &mut route);
        route.waypoints[0].max_outbound_cross_speed_mps = None;
        assert!(matches!(
            validate_route(&request, &route),
            Err(RouteValidationError::InvalidRequest(_))
        ));
    }

    #[test]
    fn request_geometry_requires_canonical_touchdown_and_flat_pad_surfaces() {
        let mut misaligned = request(0.0, 300.0);
        misaligned.initial_state.position_m.y += 0.01;
        assert!(matches!(
            misaligned.validate(),
            Err(PlanningValidationError::InvalidRequest(_))
        ));

        let mut sloped_pad = request(0.0, 300.0);
        sloped_pad.world.terrain = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-200.0, 100.0),
                Vec2::new(0.0, 101.0),
                Vec2::new(20.0, 100.0),
                Vec2::new(300.0, 100.0),
                Vec2::new(400.0, 100.0),
            ],
        };
        assert!(matches!(
            sloped_pad.validate(),
            Err(PlanningValidationError::InvalidRequest(_))
        ));
    }
}
