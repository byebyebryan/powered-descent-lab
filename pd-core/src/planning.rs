//! Read-only, serializable route-planning contracts retained for historical
//! V1 descriptors and saved reports.
//!
//! This module no longer implements or validates live routes. The serialized
//! policy and result shapes remain available for reading existing artifacts.

use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{math::Vec2, model::TransferRouteSpec, terrain::CorridorClearance};

pub const HEIGHTFIELD_VISIBILITY_ALGORITHM_ID: &str = "heightfield_visibility_v1";
pub const ROUTE_PLANNING_POLICY_VERSION: &str = "heightfield_visibility_policy_v1";

/// Historical setup-time planner timing retained in saved descriptors.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannerComputeEvidence {
    pub wall_time_us: u64,
}

/// Serialized V1 policy snapshot retained on historical route plans.
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
}

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

/// Deserialize the historical positive-infinity sentinel encoded as JSON null.
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

    pub fn waypoints(&self) -> &[crate::model::TransferWaypointSpec] {
        &self.route.waypoints
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        math::Vec2,
        model::{TransferRouteSpec, TransferWaypointSpec},
        terrain::{CorridorClearance, CorridorResidual},
    };

    use super::*;

    fn saved_route_plan_fixture() -> RoutePlan {
        let clearance = CorridorClearance {
            clear: true,
            minimum_clearance_m: 1.0,
            worst_residual: CorridorResidual {
                residual_m: -1.0,
                centerline_position_m: Vec2::new(0.0, 2.0),
                terrain_position_m: Vec2::new(0.0, 0.0),
                terrain_segment_index: 0,
                required_envelope_y_m: 1.0,
                centerline_y_m: 2.0,
                vertical_extent_m: 1.0,
            },
        };
        RoutePlan {
            algorithm_id: HEIGHTFIELD_VISIBILITY_ALGORITHM_ID.to_owned(),
            policy: RoutePlanningPolicy::default(),
            request_digest: "request".to_owned(),
            plan_digest: "plan".to_owned(),
            topology: RouteTopology::Waypoint,
            route: TransferRouteSpec {
                source_pad_id: "source".to_owned(),
                target_pad_id: "target".to_owned(),
                route_angle_deg: 0.0,
                route_radius_m: 100.0,
                waypoints: vec![TransferWaypointSpec {
                    id: "w0".to_owned(),
                    position_m: Vec2::new(50.0, 20.0),
                    handoff_tangent_unit: Some(Vec2::new(1.0, 0.0)),
                    capture_radius_m: 2.0,
                    max_cross_track_m: 3.0,
                    max_outbound_heading_error_rad: 0.35,
                    min_outbound_progress_mps: 8.0,
                    max_outbound_cross_speed_mps: Some(20.0),
                    min_speed_mps: 10.0,
                    max_speed_mps: 130.0,
                    min_vertical_speed_mps: None,
                    max_vertical_speed_mps: None,
                }],
            },
            normalized_geometry: NormalizedRouteGeometry {
                horizontal_sign: 1,
                direct_horizontal_span_m: 100.0,
                direct_distance_m: 100.0,
                route_angle_rad: 0.0,
                route_angle_deg: 0.0,
            },
            diagnostics: RoutePlanDiagnostics {
                direct_path_clear: false,
                direct_path_clearance: Some(clearance.clone()),
                route_length_m: 110.0,
                direct_distance_m: 100.0,
                excess_length_m: 10.0,
                peak_extra_loft_m: 5.0,
                minimum_planned_clearance_m: 1.0,
                leg_diagnostics: vec![
                    RouteLegDiagnostics {
                        leg_index: 0,
                        start_m: Vec2::new(0.0, 5.0),
                        end_m: Vec2::new(50.0, 20.0),
                        route_length_m: 52.0,
                        minimum_clearance_m: 1.0,
                        clearance: clearance.clone(),
                        stopping_speed_cap_mps: Some(12.0),
                        turn_speed_cap_mps: Some(f64::INFINITY),
                        handoff_speed_cap_mps: Some(10.0),
                        available_distance_m: Some(40.0),
                        stopping_ratio_at_handoff: Some(0.5),
                        turn_ratio_at_handoff: Some(0.0),
                    },
                    RouteLegDiagnostics {
                        leg_index: 1,
                        start_m: Vec2::new(50.0, 20.0),
                        end_m: Vec2::new(100.0, 5.0),
                        route_length_m: 52.0,
                        minimum_clearance_m: 1.0,
                        clearance,
                        stopping_speed_cap_mps: None,
                        turn_speed_cap_mps: None,
                        handoff_speed_cap_mps: None,
                        available_distance_m: None,
                        stopping_ratio_at_handoff: None,
                        turn_ratio_at_handoff: None,
                    },
                ],
                selected_node_ids: vec!["w0".to_owned()],
                safe_profile_points_m: vec![Vec2::new(10.0, 8.0), Vec2::new(90.0, 8.0)],
                selected_centerline_m: vec![
                    Vec2::new(0.0, 5.0),
                    Vec2::new(50.0, 20.0),
                    Vec2::new(100.0, 5.0),
                ],
                waypoint_authority: vec![WaypointAuthorityDiagnostics {
                    waypoint_index: 0,
                    available_inbound_distance_m: 40.0,
                    available_outbound_distance_m: 40.0,
                    inbound_stopping_speed_cap_mps: 12.0,
                    outbound_stopping_speed_cap_mps: 12.0,
                    inbound_turn_speed_cap_mps: f64::INFINITY,
                    outbound_turn_speed_cap_mps: f64::INFINITY,
                    handoff_speed_cap_mps: 10.0,
                    inbound_stopping_ratio_at_handoff: 0.5,
                    outbound_stopping_ratio_at_handoff: 0.5,
                    inbound_turn_ratio_at_handoff: 0.0,
                    outbound_turn_ratio_at_handoff: 0.0,
                }],
            },
        }
    }

    #[test]
    fn saved_route_diagnostics_round_trip_null_infinity_and_none() {
        let plan = saved_route_plan_fixture();
        let bytes = serde_json::to_vec(&plan).unwrap();
        let encoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(encoded["diagnostics"]["leg_diagnostics"][0]["turn_speed_cap_mps"].is_null());
        assert!(
            encoded["diagnostics"]["waypoint_authority"][0]["inbound_turn_speed_cap_mps"].is_null()
        );
        assert!(encoded["diagnostics"]["leg_diagnostics"][1]["turn_speed_cap_mps"].is_null());

        let decoded: RoutePlan = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, plan);
        assert!(
            decoded.diagnostics.leg_diagnostics[0]
                .turn_speed_cap_mps
                .unwrap()
                .is_infinite()
        );
        assert_eq!(
            decoded.diagnostics.leg_diagnostics[1].turn_speed_cap_mps,
            None
        );
        assert!(
            decoded.diagnostics.waypoint_authority[0]
                .inbound_turn_speed_cap_mps
                .is_infinite()
        );
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);

        let mut invalid_authority =
            serde_json::to_value(&plan.diagnostics.waypoint_authority[0]).unwrap();
        invalid_authority["inbound_turn_speed_cap_mps"] = serde_json::json!(-1.0);
        assert!(serde_json::from_value::<WaypointAuthorityDiagnostics>(invalid_authority).is_err());
    }
}
