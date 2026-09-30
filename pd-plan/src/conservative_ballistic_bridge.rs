//! Side-by-side V2 ballistic-first analytical certificate.
//!
//! This module deliberately does not alter the production route planner or
//! invoke the controller/simulator.  A candidate consists of one exact
//! semi-implicit ballistic arc, plus two exact discrete powered bridges: one
//! from the supported source pad to an ascending arc sample and one from a
//! descending arc sample to the target touchdown state.  The bridge is a
//! feasibility certificate, not a command stream for a controller.

use pd_core::{
    CorridorEnvelope, LandingPadSpec, RoutePlanningPolicy, RoutePlanningRequest, RouteTopology,
    TerrainDefinition, TransferRouteSpec, TransferWaypointSpec, Vec2, VehicleGeometry,
    VehicleInitialState, VehicleSpec, WaypointAuthorityDiagnostics, WaypointHandoffKinematics,
    WorldSpec, build_endpoint_profile, compute_waypoint_authority, endpoint_shaped_centerline,
    normalized_geometry,
};
use serde::{Deserialize, Serialize};

mod canonical_initial;
pub use canonical_initial::*;

const FIXTURE: &str =
    include_str!("../fixtures/conservative_ballistic_direct_bridge_probes_v2.json");
const HELDOUT_INPUT_MANIFEST_V1: &str =
    include_str!("../fixtures/conservative_ballistic_ridge_heldout_inputs_v1.json");
const FIXTURE_SCHEMA_ID: &str = "conservative_ballistic_direct_bridge_probes_v2";
const FIXTURE_SCHEMA_VERSION: u32 = 2;
const ENDPOINT_TOLERANCE: f64 = 1.0e-8;

/// Schema for the static, controller-free setup report projection.
pub const REPORT_SCHEMA_ID_V2: &str = "conservative_ballistic_direct_bridge_setup_report_v2";
pub const REPORT_SCHEMA_VERSION_V2: u32 = 2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectBridgePolicyV2 {
    pub physics_hz: u32,
    pub gravity_mps2: f64,
    pub duration_multipliers: Vec<f64>,
    pub minimum_clearance_m: f64,
    pub maximum_mission_time_s: f64,
    pub mission_time_reserve_s: f64,
    pub thrust_derate: f64,
    pub declared_robustness_margin: f64,
    pub handoff_interval_s: f64,
    pub bridge_duration_interval_s: f64,
    pub terminal_target_downward_speed_fraction: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleInputV2 {
    pub geometry: VehicleGeometryInputV2,
    pub dry_mass_kg: f64,
    pub initial_fuel_kg: f64,
    pub max_fuel_kg: f64,
    pub max_fuel_burn_kgps: f64,
    pub max_thrust_n: f64,
    pub min_throttle_frac: f64,
    pub max_rotation_rate_radps: f64,
    pub safe_touchdown_normal_speed_mps: f64,
    pub safe_touchdown_tangential_speed_mps: f64,
    pub safe_touchdown_attitude_error_rad: f64,
    pub safe_touchdown_angular_rate_radps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleGeometryInputV2 {
    pub hull_width_m: f64,
    pub hull_height_m: f64,
    pub touchdown_half_span_m: f64,
    pub touchdown_base_offset_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PadInputV2 {
    pub center_x_m: f64,
    pub surface_y_m: f64,
    pub width_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectBridgeProbeV2 {
    pub id: String,
    pub source: PadInputV2,
    pub target: PadInputV2,
    pub terrain_points_m: Vec<Vec2>,
    pub initial_position_m: Vec2,
    pub initial_velocity_mps: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectBridgeFixtureV2 {
    pub schema_id: String,
    pub schema_version: u32,
    pub policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    pub cases: Vec<DirectBridgeProbeV2>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarginV2 {
    pub raw: f64,
    pub normalized: f64,
}

impl MarginV2 {
    fn unbounded() -> Self {
        Self {
            raw: f64::MAX,
            normalized: f64::MAX,
        }
    }

    fn failed() -> Self {
        Self {
            raw: -f64::MAX,
            normalized: -f64::MAX,
        }
    }

    fn upper(limit: f64, value: f64) -> Self {
        let raw = limit - value;
        Self {
            raw,
            normalized: raw / limit.abs().max(f64::EPSILON),
        }
    }

    /// A tight numerical equality bound needs its own scale.  Normalizing it
    /// against one metre/second would turn a correct `1e-8` endpoint into an
    /// artificial robustness failure.
    fn upper_relative(limit: f64, value: f64) -> Self {
        let raw = limit - value;
        Self {
            raw,
            normalized: raw / limit,
        }
    }

    fn lower(value: f64, limit: f64) -> Self {
        let raw = value - limit;
        Self {
            raw,
            normalized: raw / limit.abs().max(f64::EPSILON),
        }
    }

    fn lower_scaled(value: f64, limit: f64, scale: f64) -> Self {
        let raw = value - limit;
        Self {
            raw,
            normalized: raw / scale.abs().max(f64::EPSILON),
        }
    }

    fn passes(self, policy: &DirectBridgePolicyV2) -> bool {
        self.raw >= 0.0 && self.normalized + 1.0e-12 >= policy.declared_robustness_margin
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentMarginsV2 {
    pub coupled_thrust: MarginV2,
    pub minimum_throttle: MarginV2,
    pub clearance: MarginV2,
    pub bridge_endpoint: MarginV2,
    pub powered_slew: MarginV2,
    pub source_attitude: MarginV2,
    pub coast_slew: MarginV2,
    pub fuel: MarginV2,
    pub time: MarginV2,
    pub touchdown_speed: MarginV2,
    pub touchdown_attitude: MarginV2,
    pub touchdown_angular_rate: MarginV2,
}

impl ComponentMarginsV2 {
    fn unbounded() -> Self {
        Self {
            coupled_thrust: MarginV2::unbounded(),
            minimum_throttle: MarginV2::unbounded(),
            clearance: MarginV2::unbounded(),
            bridge_endpoint: MarginV2::unbounded(),
            powered_slew: MarginV2::unbounded(),
            source_attitude: MarginV2::unbounded(),
            coast_slew: MarginV2::unbounded(),
            fuel: MarginV2::unbounded(),
            time: MarginV2::unbounded(),
            touchdown_speed: MarginV2::unbounded(),
            touchdown_attitude: MarginV2::unbounded(),
            touchdown_angular_rate: MarginV2::unbounded(),
        }
    }

    pub fn minimum_normalized(self) -> f64 {
        [
            self.coupled_thrust,
            self.minimum_throttle,
            self.clearance,
            self.bridge_endpoint,
            self.powered_slew,
            self.source_attitude,
            self.coast_slew,
            self.fuel,
            self.time,
            self.touchdown_speed,
            self.touchdown_attitude,
            self.touchdown_angular_rate,
        ]
        .into_iter()
        .map(|margin| margin.normalized)
        .fold(f64::INFINITY, f64::min)
    }

    fn min_with(self, other: Self) -> Self {
        Self {
            coupled_thrust: min_margin(self.coupled_thrust, other.coupled_thrust),
            minimum_throttle: min_margin(self.minimum_throttle, other.minimum_throttle),
            clearance: min_margin(self.clearance, other.clearance),
            bridge_endpoint: min_margin(self.bridge_endpoint, other.bridge_endpoint),
            powered_slew: min_margin(self.powered_slew, other.powered_slew),
            source_attitude: min_margin(self.source_attitude, other.source_attitude),
            coast_slew: min_margin(self.coast_slew, other.coast_slew),
            fuel: min_margin(self.fuel, other.fuel),
            time: min_margin(self.time, other.time),
            touchdown_speed: min_margin(self.touchdown_speed, other.touchdown_speed),
            touchdown_attitude: min_margin(self.touchdown_attitude, other.touchdown_attitude),
            touchdown_angular_rate: min_margin(
                self.touchdown_angular_rate,
                other.touchdown_angular_rate,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectBridgeReasonV2 {
    CoupledThrust,
    MinimumThrottle,
    TerrainClearance,
    BridgeEndpoint,
    PoweredSlew,
    SourceAttitude,
    CoastSlew,
    Fuel,
    MissionTime,
    TouchdownSpeed,
    TouchdownAttitude,
    TouchdownAngularRate,
    NoSourceBridge,
    NoIntermediateBridge,
    NoTerminalBridge,
    NoNonoverlappingPair,
    RouteProgress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertificationV2 {
    Certified,
    /// The declared analytical screens rejected this candidate; this is not
    /// a claim that every unmodeled physical/controller route is impossible.
    NotCertified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeKindV2 {
    Source,
    Intermediate,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct KinematicStateV2 {
    pub position_m: Vec2,
    pub velocity_mps: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VirtualBallisticArcV2 {
    pub start_m: Vec2,
    pub end_m: Vec2,
    pub dt_s: f64,
    pub gravity_mps2: f64,
    pub steps: u64,
    pub duration_s: f64,
    pub departure_velocity_mps: Vec2,
    pub arrival_velocity_mps: Vec2,
    pub apex_step: u64,
    pub apex_position_m: Vec2,
    pub identity: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandoffV2 {
    pub arc_step: u64,
    pub state: KinematicStateV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeEnvironmentEvidenceV2 {
    pub clearance_margin: MarginV2,
    pub pad_corridor_tick_count: usize,
    pub free_flight_tick_count: usize,
    pub first_powered_direction_unit: Option<Vec2>,
    pub final_powered_direction_unit: Option<Vec2>,
    pub initial_attitude_margin: MarginV2,
    pub final_attitude_margin: MarginV2,
    pub final_angular_rate_margin: MarginV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoastEvidenceV2 {
    pub source_handoff: HandoffV2,
    pub terminal_handoff: HandoffV2,
    pub duration_s: f64,
    pub clearance_margin: MarginV2,
    pub slew_angle_rad: f64,
    pub required_slew_time_s: f64,
    pub coast_slew_margin: MarginV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectBridgeCandidateV2 {
    pub duration_multiplier: f64,
    pub virtual_arc: VirtualBallisticArcV2,
    pub source_handoff: Option<HandoffV2>,
    pub terminal_handoff: Option<HandoffV2>,
    pub source_bridge: Option<AnalyticalBridgeV2>,
    pub terminal_bridge: Option<AnalyticalBridgeV2>,
    pub source_environment: Option<BridgeEnvironmentEvidenceV2>,
    pub terminal_environment: Option<BridgeEnvironmentEvidenceV2>,
    pub selected_coast: Option<CoastEvidenceV2>,
    pub source_bridge_attempt_count: usize,
    pub terminal_bridge_attempt_count: usize,
    pub source_bridge_streamed_tick_count: usize,
    pub terminal_bridge_streamed_tick_count: usize,
    pub source_environment_rejection_count: usize,
    pub terminal_environment_rejection_count: usize,
    pub classification: CertificationV2,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub margins: ComponentMarginsV2,
    pub total_fuel_burn_kg: Option<f64>,
    pub total_time_s: Option<f64>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectBridgeProbeResultV2 {
    pub id: String,
    pub release_reference_m: Vec2,
    pub touchdown_reference_m: Vec2,
    pub candidates: Vec<DirectBridgeCandidateV2>,
    pub certified_candidate_count: usize,
    pub selected_candidate_identity: String,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectBridgeEvaluationV2 {
    pub fixture_identity: String,
    pub policy_identity: String,
    pub vehicle_identity: String,
    pub results: Vec<DirectBridgeProbeResultV2>,
    pub ridge_canary: RidgeCanaryEvidenceV2,
    pub identity: String,
}

/// Mission-level status for the declared nominal lane.  This is intentionally
/// separate from [`CertificationV2`]: a red result means that the selected
/// nominal lane was rejected by the bounded analytical policy, not that every
/// physically possible route is impossible.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionStatusV2 {
    Green,
    Red,
}

/// Explicit boundary for the local correction envelope.  The envelope starts
/// after the nominal source bridge has committed the vehicle to coast; it is
/// not a claim about pre-handoff source-bridge replanning.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionCommitmentPhaseV2 {
    PostSourceHandoff,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatTwinEvidenceV2 {
    pub source_case_id: String,
    pub terrain_identity: String,
    pub candidate_duration_multipliers: Vec<f64>,
    pub candidate_classifications: Vec<CertificationV2>,
    pub candidate_identities: Vec<String>,
    pub certified_candidate_count: usize,
    pub nominal_candidate_identity: String,
    pub nominal_duration_multiplier: f64,
    pub nominal_duration_s: f64,
    pub nominal_candidate: DirectBridgeCandidateV2,
    pub status: MissionStatusV2,
    pub identity: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrectionEnvelopeBoundsV2 {
    pub lower_m: Vec2,
    pub upper_m: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrectionEnvelopeSampleV2 {
    pub arc_step: u64,
    pub elapsed_s: f64,
    pub nominal_state: KinematicStateV2,
    /// Maximum additional elapsed time included at this nominal sample.  The
    /// corresponding nominal drift is bounded separately below, rather than
    /// being silently folded into thrust authority.
    pub time_shift_allowance_s: f64,
    pub nominal_time_shift_drift_bounds_m: CorrectionEnvelopeBoundsV2,
    pub correction_time_s: f64,
    pub thrust_displacement_bound_m: f64,
    pub horizontal_displacement_bound_m: f64,
    pub vertical_displacement_bound_m: f64,
    pub bounds: CorrectionEnvelopeBoundsV2,
}

/// Conservative crossing cut through the local-correction tube.  The cut
/// combines every exact physics-tick sample whose horizontal bound reaches
/// the ridge center; it is deliberately not the union of every historical
/// tube sample.  A mesa that covers this cut blocks the declared crossing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorrectionCrossingCutV2 {
    pub feature_center_x_m: f64,
    pub eligible_sample_count: usize,
    pub first_arc_step: Option<u64>,
    pub last_arc_step: Option<u64>,
    pub bounds: CorrectionEnvelopeBoundsV2,
}

/// A controller-neutral, intentionally optimistic bound on local correction
/// after the nominal source bridge has handed off to coast.  The bound uses
/// full derated thrust at worst-case mass in any direction, ignores attitude
/// slew and throttle granularity, and is limited by fuel, mission reserve,
/// and an explicit crossing-time allowance.  It is therefore an over-approx-
/// imation of a simple reactive lane, not a controller replay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalCorrectionEnvelopeV2 {
    pub nominal_candidate_identity: String,
    pub commitment_phase: CorrectionCommitmentPhaseV2,
    pub source_handoff: HandoffV2,
    pub commitment_state: KinematicStateV2,
    pub feature_center_x_m: f64,
    pub nominal_crossing_arc_step: u64,
    pub nominal_crossing_state: KinematicStateV2,
    pub crossing_time_s: f64,
    pub crossing_time_allowance_s: f64,
    pub correction_horizon_s: f64,
    pub mission_horizon_s: f64,
    pub fuel_horizon_s: f64,
    pub available_correction_fuel_kg: f64,
    pub terminal_reserve_fuel_kg: f64,
    pub mission_reserve_fuel_kg: f64,
    pub worst_case_mass_kg: f64,
    pub derated_max_acceleration_mps2: f64,
    pub vehicle_clearance_extent_m: Vec2,
    pub horizontal_displacement_bound_m: f64,
    pub vertical_displacement_bound_m: f64,
    pub bounds: CorrectionEnvelopeBoundsV2,
    pub samples: Vec<CorrectionEnvelopeSampleV2>,
    pub crossing_cut: CorrectionCrossingCutV2,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MesaGeometryV2 {
    pub source_feature_identity: String,
    pub base_y_m: f64,
    pub feature_left_x_m: f64,
    pub feature_right_x_m: f64,
    pub feature_top_y_m: f64,
    pub center_x_m: f64,
    pub base_left_x_m: f64,
    pub top_left_x_m: f64,
    pub top_right_x_m: f64,
    pub base_right_x_m: f64,
    pub top_y_m: f64,
    pub terrain_points_m: Vec<Vec2>,
    pub correction_blocking_margin_m: MarginV2,
    pub blocks_full_correction_corridor: bool,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RidgeDirectDiagnosticV2 {
    pub duration_multiplier: f64,
    pub arc_steps: u64,
    pub duration_s: f64,
    pub classification: CertificationV2,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub minimum_normalized_margin: f64,
    pub candidate_identity: String,
}

/// Ordered segments used to check that a selected waypoint certificate keeps
/// progressing along the source-to-target ground track.  This is intentionally
/// a canary-witness screen, not a new global bridge margin.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteProgressSegmentV2 {
    SourceBridge,
    SourceCoast,
    IntermediateBridge,
    TerminalCoast,
    TerminalBridge,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteProgressSegmentEvidenceV2 {
    pub segment: RouteProgressSegmentV2,
    pub sample_count: usize,
    pub minimum_tangent_velocity_mps: f64,
    pub backtracking_distance_m: f64,
    pub allowed_backtracking_distance_m: f64,
    pub backtracking_margin_m: f64,
    pub passes: bool,
}

/// Controller-neutral route-progress screen for the selected one-waypoint
/// witness. Ground-track tangent deliberately ignores terminal vertical
/// descent. Source and terminal bridges may use the vehicle's touchdown
/// half-span for local pad alignment; coasts and the intermediate bridge may
/// use only the exact endpoint tolerance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteProgressEvidenceV2 {
    pub ground_track_tangent_unit: Vec2,
    pub segments: Vec<RouteProgressSegmentEvidenceV2>,
    pub minimum_tangent_velocity_mps: f64,
    pub total_backtracking_distance_m: f64,
    pub passes: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointCandidateV2 {
    pub waypoint_position_m: Vec2,
    pub source_leg: VirtualBallisticArcV2,
    pub target_leg: VirtualBallisticArcV2,
    pub source_handoff: Option<HandoffV2>,
    pub intermediate_entry_handoff: Option<HandoffV2>,
    pub intermediate_exit_handoff: Option<HandoffV2>,
    pub terminal_handoff: Option<HandoffV2>,
    pub source_bridge: Option<AnalyticalBridgeV2>,
    pub intermediate_bridge: Option<AnalyticalBridgeV2>,
    pub terminal_bridge: Option<AnalyticalBridgeV2>,
    pub source_environment: Option<BridgeEnvironmentEvidenceV2>,
    pub intermediate_environment: Option<BridgeEnvironmentEvidenceV2>,
    pub terminal_environment: Option<BridgeEnvironmentEvidenceV2>,
    pub route_progress: Option<RouteProgressEvidenceV2>,
    pub source_bridge_attempt_count: usize,
    pub intermediate_bridge_attempt_count: usize,
    pub terminal_bridge_attempt_count: usize,
    pub source_bridge_streamed_tick_count: usize,
    pub intermediate_bridge_streamed_tick_count: usize,
    pub terminal_bridge_streamed_tick_count: usize,
    pub source_environment_rejection_count: usize,
    pub intermediate_environment_rejection_count: usize,
    pub terminal_environment_rejection_count: usize,
    pub classification: CertificationV2,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub margins: ComponentMarginsV2,
    pub total_fuel_burn_kg: Option<f64>,
    pub total_time_s: Option<f64>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointSearchEvidenceV2 {
    pub max_candidates: usize,
    pub waypoint_position_count: usize,
    pub leg_duration_pair_count: usize,
    pub candidate_count: usize,
    pub certified_candidate_count: usize,
    pub rejection_counts: Vec<(DirectBridgeReasonV2, usize)>,
    pub selected_candidate_identity: Option<String>,
    pub selected_candidate: Option<WaypointCandidateV2>,
    pub candidates: Vec<WaypointCandidateV2>,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RidgeCanaryEvidenceV2 {
    pub schema_id: String,
    pub schema_version: u32,
    pub source_case_id: String,
    pub flat_control: FlatTwinEvidenceV2,
    pub nominal_candidate_identity: String,
    pub correction_envelope: LocalCorrectionEnvelopeV2,
    pub mesa: MesaGeometryV2,
    /// True only when the canonical envelope passes its structural checks and
    /// the derived mesa clears the declared robust blocking screen.
    pub blocking_lane_valid: bool,
    pub direct_status: MissionStatusV2,
    pub direct_nominal_candidate: RidgeDirectDiagnosticV2,
    pub direct_global_replans: Vec<RidgeDirectDiagnosticV2>,
    pub waypoint_search: WaypointSearchEvidenceV2,
    pub identity: String,
}

/// A self-contained, tamper-recomputable projection of the frozen v2 setup
/// and its analytical certificate evaluation. It does not involve the
/// production planner, controller, or simulator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectBridgeReportV2 {
    pub schema_id: String,
    pub schema_version: u32,
    pub fixture: DirectBridgeFixtureV2,
    pub evaluation: DirectBridgeEvaluationV2,
    pub identity: String,
}

/// A deliberately narrow, non-production decision exposed for the frozen V2
/// ridge experiment.  This is not the production mission planner: `Direct`
/// always means the shortest robust flat-derived nominal lane, while
/// `OneWaypoint` is the finite terrain-derived repair selected only after that
/// same nominal lane is rejected on the derived mesa.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExperimentalRidgeCandidateOutcomeV2 {
    Direct {
        candidate: Box<DirectBridgeCandidateV2>,
    },
    OneWaypoint {
        candidate: Box<WaypointCandidateV2>,
        crossing: Box<ExactIntermediateBridgeCrossingV2>,
    },
    Unsupported {
        reason: ExperimentalRidgeUnsupportedReasonV2,
        rejection_reasons: Vec<DirectBridgeReasonV2>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRidgeUnsupportedReasonV2 {
    NoCertifiedNominalDirect,
    NominalLaneNotRobustlyBlocked,
    FiniteWaypointSearchExhausted,
}

/// Exact analytical crossing selected from the certified intermediate bridge.
/// The virtual anchor is provenance only and is never substituted for a state
/// actually traversed by the discrete bridge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExactIntermediateBridgeCrossingV2 {
    pub selection_rule: String,
    pub candidate_identity: String,
    pub intermediate_bridge_identity: String,
    pub virtual_anchor_m: Vec2,
    pub intermediate_entry_handoff: HandoffV2,
    pub intermediate_exit_handoff: HandoffV2,
    pub intermediate_bridge_total_applied_steps: u64,
    pub certified_prefix_end_applied_steps: u64,
    pub certified_suffix_start_applied_steps: u64,
    pub previous_applied_steps: u64,
    pub selected_applied_steps: u64,
    pub previous_state: KinematicStateV2,
    pub selected_state: KinematicStateV2,
    pub previous_directed_offset_m: f64,
    pub selected_directed_offset_m: f64,
    pub strict_directed_crossing: bool,
}

/// Compact projection consumed by the controller-shadow evaluator.  It keeps
/// the frozen analytical inputs, candidate ordering/provenance, and the two
/// scoped decisions without exposing the complete setup-report artifact.
///
/// This API exists only under the `conservative-ballistic-report` feature and
/// must not be treated as a general or production route-planning interface.
#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentalRidgeCandidateProjectionV2 {
    pub policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    pub case: DirectBridgeProbeV2,
    pub mesa: MesaGeometryV2,
    pub analytical_report_identity: String,
    pub analytical_canary_identity: String,
    pub flat_candidate_identities: Vec<String>,
    pub ridge_direct_candidate_identities: Vec<String>,
    pub waypoint_search_identity: String,
    pub waypoint_candidate_identities: Vec<String>,
    pub flat_control: ExperimentalRidgeCandidateOutcomeV2,
    pub derived_mesa: ExperimentalRidgeCandidateOutcomeV2,
}

/// Versioned, input-only contract for one arbitrary case from the bounded
/// ridge family.  The caller cannot provide any derived mesa, certificate,
/// route, or controller result: every such value is recomputed below.
pub const EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_ID_V1: &str = "experimental_ridge_case_input_v1";
pub const EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeCaseInputV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    /// One and only one raw probe.  A separate fixture or probe selection
    /// field would permit case-ID selection to enter this generic boundary.
    pub probe: DirectBridgeProbeV2,
    pub identity: String,
}

/// Versioned input-only manifest for the two frozen H2 held-out ridge cases.
///
/// The manifest is deliberately separate from [`DirectBridgeFixtureV2`].  It
/// binds the ordered, complete generic inputs used by the held-out experiment
/// without carrying any analytical candidate, runtime route, or controller
/// outcome fields.
pub const EXPERIMENTAL_RIDGE_CASE_MANIFEST_SCHEMA_ID_V1: &str =
    "experimental_ridge_case_manifest_v1";
pub const EXPERIMENTAL_RIDGE_CASE_MANIFEST_SCHEMA_VERSION_V1: u32 = 1;
pub const EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1: [&str; 2] =
    ["ridge_progress_050_probe", "ridge_progress_068_probe"];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRidgeCaseManifestErrorV1 {
    Json,
    SchemaMismatch,
    CaseCountMismatch,
    CaseOrderMismatch,
    DuplicateCaseId,
    InputInvalid,
    IdentityMismatch,
}

impl std::fmt::Display for ExperimentalRidgeCaseManifestErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Json => "experimental ridge case manifest JSON is invalid",
            Self::SchemaMismatch => "experimental ridge case manifest schema is invalid",
            Self::CaseCountMismatch => {
                "experimental ridge case manifest must contain exactly two cases"
            }
            Self::CaseOrderMismatch => {
                "experimental ridge case manifest case IDs are not in the frozen order"
            }
            Self::DuplicateCaseId => "experimental ridge case manifest contains a duplicate ID",
            Self::InputInvalid => "experimental ridge case manifest contains an invalid input",
            Self::IdentityMismatch => {
                "experimental ridge case manifest identity does not bind its ordered inputs"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExperimentalRidgeCaseManifestErrorV1 {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentalRidgeCaseManifestV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub cases: Vec<ExperimentalRidgeCaseInputV1>,
    pub identity: String,
}

/// Compact, recomputable analytical projection for one
/// [`ExperimentalRidgeCaseInputV1`].  This intentionally carries no
/// embedded-report identity: its provenance is the caller-owned input and a
/// freshly evaluated per-case canary only.
pub const EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_ID_V1: &str =
    "experimental_ridge_case_projection_v1";
pub const EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeCaseProjectionV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input_identity: String,
    pub analytical_canary_identity: String,
    pub policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    pub probe: DirectBridgeProbeV2,
    pub mesa: MesaGeometryV2,
    pub flat_candidate_identities: Vec<String>,
    pub ridge_direct_candidate_identities: Vec<String>,
    /// Compact, sample-free evidence for the same-duration direct candidate
    /// evaluated against the derived mesa.  H2 uses this to distinguish the
    /// intended terrain-clearance rejection from a generic red status.
    pub derived_nominal_direct: RidgeDirectDiagnosticV2,
    /// Non-nominal direct candidates remain diagnostic-only. They are exposed
    /// as stopped analytical-result evidence; selection remains the finite
    /// one-waypoint repair.
    pub derived_non_nominal_direct_diagnostics: Vec<RidgeDirectDiagnosticV2>,
    /// This is the complete robust-blocker predicate, not merely the mesa
    /// geometry flag.  It binds the correction-envelope validation as well.
    pub derived_blocker_valid: bool,
    /// The finite search keeps only its first certified witness, so this
    /// explicit count is necessary to verify the one-witness prediction.
    pub waypoint_certified_candidate_count: usize,
    pub waypoint_search_identity: String,
    pub waypoint_candidate_identities: Vec<String>,
    pub flat_control: ExperimentalRidgeCandidateOutcomeV2,
    pub derived_mesa: ExperimentalRidgeCandidateOutcomeV2,
    pub identity: String,
}

/// Schema identity for the planner-owned controller-facing projection.  This
/// remains feature-gated with the experimental V2 candidate API and is not a
/// production `RoutePlan` contract.
pub const EXPERIMENTAL_RIDGE_RUNTIME_PROJECTION_SCHEMA_ID_V2: &str =
    "experimental_ridge_runtime_route_projection_v2";
pub const EXPERIMENTAL_RIDGE_RUNTIME_PROJECTION_SCHEMA_VERSION_V2: u32 = 2;

/// Structural evidence for a route accepted by `TransferRouteSpec::validate`.
/// This intentionally does not contain the result of the ordinary
/// `pd_core::validate_route` full-pad validator; that validator remains a
/// separate evaluator diagnostic for the source-taper canary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeRuntimeRouteStructureV2 {
    pub topology: RouteTopology,
    pub waypoint_count: usize,
    pub transfer_route_valid: bool,
}

/// Planner-neutral copy of the canonical waypoint handoff kinematics.  The
/// core model type is intentionally not serialized, so the feature-gated
/// projection carries this stable evidence shape for downstream evaluators.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeRuntimeWaypointHandoffKinematicsV2 {
    pub distance_m: f64,
    pub cross_track_m: f64,
    pub plane_progress_m: f64,
    pub outbound_heading_error_rad: f64,
    pub outbound_progress_mps: f64,
    pub outbound_cross_speed_mps: f64,
    pub speed_mps: f64,
    pub vertical_speed_mps: f64,
}

impl From<WaypointHandoffKinematics> for ExperimentalRidgeRuntimeWaypointHandoffKinematicsV2 {
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

/// Planner-neutral copy of the canonical waypoint handoff assessment.  The
/// violation strings are the stable names already defined by `pd-core`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeRuntimeWaypointHandoffAssessmentV2 {
    pub triggered: bool,
    pub capture_window_open: bool,
    pub deadline_reached: bool,
    pub spatial_pass: bool,
    pub envelope_pass: bool,
    pub contract_pass: bool,
    pub violations: Vec<String>,
}

/// Controller-facing result of projecting one validated analytical outcome.
/// `Unsupported` is a valid bounded-search result, distinct from an invalid
/// candidate or a failed runtime projection.
// The one-waypoint variant intentionally keeps the complete crossing and
// handoff evidence inline so evaluators can consume it without another
// allocation/dereference boundary. This feature-gated evidence enum is not a
// hot-path production value.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExperimentalRidgeRuntimeOutcomeV2 {
    Direct {
        candidate_identity: String,
        route: TransferRouteSpec,
        structural_validation: ExperimentalRidgeRuntimeRouteStructureV2,
        identity: String,
    },
    OneWaypoint {
        candidate_identity: String,
        route: TransferRouteSpec,
        crossing: ExactIntermediateBridgeCrossingV2,
        authority: WaypointAuthorityDiagnostics,
        handoff_kinematics: ExperimentalRidgeRuntimeWaypointHandoffKinematicsV2,
        handoff_assessment: ExperimentalRidgeRuntimeWaypointHandoffAssessmentV2,
        structural_validation: ExperimentalRidgeRuntimeRouteStructureV2,
        identity: String,
    },
    Unsupported {
        reason: ExperimentalRidgeUnsupportedReasonV2,
        rejection_reasons: Vec<DirectBridgeReasonV2>,
        identity: String,
    },
}

/// Planner-owned runtime projection of both frozen V2 canary decisions.  The
/// evaluator can consume the two route outcomes without reconstructing the
/// analytical certificate or the waypoint route policy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeRuntimeProjectionV2 {
    pub schema_id: String,
    pub schema_version: u32,
    pub analytical_report_identity: String,
    pub analytical_canary_identity: String,
    pub policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    pub case: DirectBridgeProbeV2,
    pub mesa: MesaGeometryV2,
    pub flat_control: ExperimentalRidgeRuntimeOutcomeV2,
    pub derived_mesa: ExperimentalRidgeRuntimeOutcomeV2,
    pub identity: String,
}

/// Schema identity for the runtime counterpart of the generic V1 ridge-case
/// projection.  It deliberately remains distinct from the frozen embedded V2
/// controller-shadow projection.
pub const EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V1: &str =
    "experimental_ridge_case_runtime_projection_v1";
pub const EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeCaseRuntimeProjectionV1 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input_identity: String,
    pub analytical_canary_identity: String,
    pub policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    pub probe: DirectBridgeProbeV2,
    pub mesa: MesaGeometryV2,
    pub flat_control: ExperimentalRidgeRuntimeOutcomeV2,
    pub derived_mesa: ExperimentalRidgeRuntimeOutcomeV2,
    pub identity: String,
}

/// Schema identity for the generic runtime projection with the bounded,
/// controller-free handoff selection diagnostic.  The V1 generic runtime
/// projection intentionally remains crossing-only; this is a new boundary so
/// consumers cannot accidentally reinterpret historical V1 evidence.
pub const EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V2: &str =
    "experimental_ridge_case_runtime_projection_v2";
pub const EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V2: u32 = 2;

/// The only semantic handoff states considered by the generic V2 projector.
/// `PrimaryCrossing` is the existing exact virtual-anchor crossing.  The
/// intermediate bridge exit is considered only when that primary state fails
/// the canonical handoff contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRidgeRuntimeHandoffSelectionKindV2 {
    PrimaryCrossing,
    IntermediateBridgeExit,
}

/// Complete evidence for one exact state considered by the V2 handoff
/// selector.  The route and waypoint are generated from `selected_state` and
/// therefore retain the exact state used for the canonical assessment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeRuntimeHandoffAttemptV2 {
    pub selection_kind: ExperimentalRidgeRuntimeHandoffSelectionKindV2,
    pub applied_steps: u64,
    /// `Some` only for the certified intermediate-bridge fallback.  The
    /// value is the target-leg arc step at which that bridge exits.
    pub target_leg_arc_step: Option<u64>,
    pub selected_state: KinematicStateV2,
    pub route: TransferRouteSpec,
    pub authority: WaypointAuthorityDiagnostics,
    pub handoff_kinematics: ExperimentalRidgeRuntimeWaypointHandoffKinematicsV2,
    pub handoff_assessment: ExperimentalRidgeRuntimeWaypointHandoffAssessmentV2,
    pub structural_validation: ExperimentalRidgeRuntimeRouteStructureV2,
    pub identity: String,
}

/// Structured provenance retained when both semantic handoff states are
/// considered, including the original exact crossing even when the
/// intermediate bridge exit is selected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeRuntimeHandoffSelectionV2 {
    pub candidate_identity: String,
    pub crossing: ExactIntermediateBridgeCrossingV2,
    pub attempts: Vec<ExperimentalRidgeRuntimeHandoffAttemptV2>,
    pub selected_attempt_index: usize,
    pub identity: String,
}

impl ExperimentalRidgeRuntimeHandoffSelectionV2 {
    /// Return the selected attempt, or `None` for a malformed selection
    /// artifact whose index does not address its bounded attempt list.
    pub fn selected_attempt(&self) -> Option<&ExperimentalRidgeRuntimeHandoffAttemptV2> {
        self.attempts.get(self.selected_attempt_index)
    }
}

/// Parallel generic V2 runtime outcome.  It mirrors the stable route outcome
/// fields used by the embedded projection while carrying the new structured
/// handoff selection inline for a one-waypoint result.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExperimentalRidgeCaseRuntimeOutcomeV2 {
    Direct {
        candidate_identity: String,
        route: TransferRouteSpec,
        structural_validation: ExperimentalRidgeRuntimeRouteStructureV2,
        identity: String,
    },
    OneWaypoint {
        candidate_identity: String,
        route: TransferRouteSpec,
        crossing: ExactIntermediateBridgeCrossingV2,
        handoff_selection: ExperimentalRidgeRuntimeHandoffSelectionV2,
        authority: WaypointAuthorityDiagnostics,
        handoff_kinematics: ExperimentalRidgeRuntimeWaypointHandoffKinematicsV2,
        handoff_assessment: ExperimentalRidgeRuntimeWaypointHandoffAssessmentV2,
        structural_validation: ExperimentalRidgeRuntimeRouteStructureV2,
        identity: String,
    },
    Unsupported {
        reason: ExperimentalRidgeUnsupportedReasonV2,
        rejection_reasons: Vec<DirectBridgeReasonV2>,
        identity: String,
    },
}

/// Typed evidence retained at the error boundary when both semantic states
/// fail `TransferWaypointSpec::assess_handoff`.  This keeps failed diagnostic
/// data available without converting a failed contract into a route result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeRuntimeHandoffFailureV2 {
    pub candidate_identity: String,
    pub crossing: ExactIntermediateBridgeCrossingV2,
    pub attempts: Vec<ExperimentalRidgeRuntimeHandoffAttemptV2>,
    pub identity: String,
}

/// Generic V2 runtime projection envelope.  The route outcome keeps the
/// established runtime shape for downstream consumers; one-waypoint handoff
/// selection evidence is carried by its parallel V2 outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExperimentalRidgeCaseRuntimeProjectionV2 {
    pub schema_id: String,
    pub schema_version: u32,
    pub input_identity: String,
    pub analytical_canary_identity: String,
    pub policy: DirectBridgePolicyV2,
    pub vehicle: VehicleInputV2,
    pub probe: DirectBridgeProbeV2,
    pub mesa: MesaGeometryV2,
    pub flat_control: ExperimentalRidgeCaseRuntimeOutcomeV2,
    pub derived_mesa: ExperimentalRidgeCaseRuntimeOutcomeV2,
    pub identity: String,
}

/// Stable typed failures for the generic V2 runtime boundary.  Unlike the
/// historical V1 `HandoffContractFailed` result, a two-attempt contract
/// failure carries every generated attempt and its canonical assessment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExperimentalRidgeCaseRuntimeProjectionErrorV2 {
    CandidateEvidence(ExperimentalRidgeCandidateErrorV2),
    InvalidRuntimeRequest,
    WaypointRouteGeometryInvalid,
    WaypointTangentInvalid,
    WaypointAuthorityInvalid,
    WaypointAuthorityBelowMinimum,
    RouteStructuralValidationFailed,
    HandoffKinematicsInvalid,
    TargetLegAcquisitionApexSeamInvalid,
    HandoffAttemptsFailed(Box<ExperimentalRidgeRuntimeHandoffFailureV2>),
    RuntimeProjectionIdentityMismatch,
}

impl std::fmt::Display for ExperimentalRidgeCaseRuntimeProjectionErrorV2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::CandidateEvidence(error) => {
                return write!(formatter, "candidate evidence: {error}");
            }
            Self::InvalidRuntimeRequest => "runtime projection request is invalid",
            Self::WaypointRouteGeometryInvalid => "waypoint route geometry is invalid",
            Self::WaypointTangentInvalid => "waypoint route tangent is invalid",
            Self::WaypointAuthorityInvalid => "waypoint authority could not be computed",
            Self::WaypointAuthorityBelowMinimum => {
                "waypoint authority cap is below the configured handoff minimum"
            }
            Self::RouteStructuralValidationFailed => "runtime route failed structural validation",
            Self::HandoffKinematicsInvalid => "waypoint handoff kinematics are invalid",
            Self::TargetLegAcquisitionApexSeamInvalid => {
                "intermediate bridge exit is not the certified target-leg acquisition/apex seam"
            }
            Self::HandoffAttemptsFailed(_) => {
                "both semantic waypoint handoff attempts fail the canonical contract"
            }
            Self::RuntimeProjectionIdentityMismatch => {
                "runtime route projection identity does not bind its canonical contents"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExperimentalRidgeCaseRuntimeProjectionErrorV2 {}

/// Stable fail-closed reasons for malformed experimental candidate evidence.
/// A valid but exhausted finite search is represented by
/// [`ExperimentalRidgeCandidateOutcomeV2::Unsupported`], not this error type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRidgeCandidateErrorV2 {
    EmbeddedReportInvalid,
    GenericInputInvalid,
    GenericInputIdentityMismatch,
    GenericProjectionSchemaMismatch,
    GenericProjectionIdentityMismatch,
    GenericProjectionMismatch,
    DerivedMesaGeometryInvalid,
    ProjectionMismatch,
    FlatNominalIdentityMismatch,
    FlatNominalCertificateInvalid,
    DerivedNominalEvidenceMismatch,
    WaypointSelectionIdentityMismatch,
    WaypointCertificateMissing,
    WaypointCertificateInvalid,
    IntermediateBridgeMissing,
    IntermediateHandoffMissing,
    IntermediateBridgeJoinMismatch,
    IntermediateBridgeHasNoSteps,
    IntermediateBridgeStartsAtOrBeyondAnchor,
    IntermediateBridgeDoesNotCrossAnchor,
    IntermediateCrossingMismatch,
}

impl std::fmt::Display for ExperimentalRidgeCandidateErrorV2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmbeddedReportInvalid => "embedded V2 report failed canonical validation",
            Self::GenericInputInvalid => "generic ridge-case input schema or contents are invalid",
            Self::GenericInputIdentityMismatch => {
                "generic ridge-case input identity does not bind its canonical contents"
            }
            Self::GenericProjectionSchemaMismatch => {
                "generic ridge-case projection schema is invalid"
            }
            Self::GenericProjectionIdentityMismatch => {
                "generic ridge-case projection identity does not bind its canonical contents"
            }
            Self::GenericProjectionMismatch => {
                "generic ridge-case projection does not recompute from its input"
            }
            Self::DerivedMesaGeometryInvalid => {
                "derived mesa geometry is invalid for the generic ridge-case input"
            }
            Self::ProjectionMismatch => {
                "experimental ridge candidate projection does not match the embedded V2 evidence"
            }
            Self::FlatNominalIdentityMismatch => {
                "flat nominal candidate identity or deterministic ordering is inconsistent"
            }
            Self::FlatNominalCertificateInvalid => {
                "flat nominal candidate certificate is not valid"
            }
            Self::DerivedNominalEvidenceMismatch => {
                "derived-mesa nominal candidate does not match its analytical diagnostic"
            }
            Self::WaypointSelectionIdentityMismatch => {
                "selected waypoint identity or deterministic ordering is inconsistent"
            }
            Self::WaypointCertificateMissing => {
                "ridge repair decision is missing its selected waypoint certificate"
            }
            Self::WaypointCertificateInvalid => {
                "selected waypoint certificate does not pass every analytical invariant"
            }
            Self::IntermediateBridgeMissing => {
                "selected waypoint certificate has no intermediate bridge"
            }
            Self::IntermediateHandoffMissing => {
                "selected waypoint certificate has incomplete intermediate handoffs"
            }
            Self::IntermediateBridgeJoinMismatch => {
                "selected intermediate bridge does not exactly join its recorded handoffs"
            }
            Self::IntermediateBridgeHasNoSteps => {
                "selected intermediate bridge has no applied-step crossing bracket"
            }
            Self::IntermediateBridgeStartsAtOrBeyondAnchor => {
                "selected intermediate bridge does not begin before the virtual-anchor plane"
            }
            Self::IntermediateBridgeDoesNotCrossAnchor => {
                "selected intermediate bridge never reaches the virtual-anchor plane"
            }
            Self::IntermediateCrossingMismatch => {
                "recorded intermediate crossing is not the first exact directed crossing"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExperimentalRidgeCandidateErrorV2 {}

/// Stable fail-closed errors emitted while projecting validated analytical
/// evidence into a controller-facing route.  Candidate evidence failures are
/// preserved as their original typed reason; unsupported finite search never
/// becomes one of these errors.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentalRidgeRuntimeProjectionErrorV2 {
    CandidateEvidence(ExperimentalRidgeCandidateErrorV2),
    InvalidRuntimeRequest,
    WaypointRouteGeometryInvalid,
    WaypointTangentInvalid,
    WaypointAuthorityInvalid,
    WaypointAuthorityBelowMinimum,
    RouteStructuralValidationFailed,
    HandoffKinematicsInvalid,
    HandoffContractFailed,
    RuntimeProjectionIdentityMismatch,
}

impl std::fmt::Display for ExperimentalRidgeRuntimeProjectionErrorV2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::CandidateEvidence(error) => {
                return write!(formatter, "candidate evidence: {error}");
            }
            Self::InvalidRuntimeRequest => "runtime projection request is invalid",
            Self::WaypointRouteGeometryInvalid => "waypoint route geometry is invalid",
            Self::WaypointTangentInvalid => "waypoint route tangent is invalid",
            Self::WaypointAuthorityInvalid => "waypoint authority could not be computed",
            Self::WaypointAuthorityBelowMinimum => {
                "waypoint authority cap is below the configured handoff minimum"
            }
            Self::RouteStructuralValidationFailed => "runtime route failed structural validation",
            Self::HandoffKinematicsInvalid => "waypoint handoff kinematics are invalid",
            Self::HandoffContractFailed => {
                "waypoint handoff does not satisfy the canonical contract"
            }
            Self::RuntimeProjectionIdentityMismatch => {
                "runtime route projection identity does not bind its canonical contents"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExperimentalRidgeRuntimeProjectionErrorV2 {}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeSampleV2 {
    /// Zero-based acceleration sample index.  `state_m` is the exact state
    /// after applying this acceleration for one physics tick.
    pub tick: u64,
    pub state_m: KinematicStateV2,
    pub net_acceleration_mps2: Vec2,
    pub thrust_acceleration_mps2: Vec2,
    pub throttle_fraction: f64,
    pub thrust_direction_unit: Option<Vec2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalyticalBridgeV2 {
    pub kind: BridgeKindV2,
    pub start_state: KinematicStateV2,
    pub end_state: KinematicStateV2,
    pub steps: u64,
    pub duration_s: f64,
    pub initial_net_acceleration_mps2: Vec2,
    pub net_acceleration_step_mps2: Vec2,
    /// Optional runtime materialization of the exact affine bridge. Samples
    /// are derived from the compact coefficients and are intentionally not
    /// serialized into setup-report artifacts.
    #[serde(default, skip_serializing)]
    pub samples: Vec<BridgeSampleV2>,
    pub endpoint_position_error_m: f64,
    pub endpoint_velocity_error_mps: f64,
    pub fuel_burn_kg: f64,
    pub classification: CertificationV2,
    pub reasons: Vec<DirectBridgeReasonV2>,
    pub margins: ComponentMarginsV2,
    pub identity: String,
}

impl AnalyticalBridgeV2 {
    /// Return the exact state after `applied_steps` semi-implicit updates.
    pub fn state_at(&self, applied_steps: u64) -> KinematicStateV2 {
        assert!(
            applied_steps <= self.steps,
            "powered-bridge sample lies within bridge"
        );
        exact_affine_bridge_state(
            self.start_state,
            self.initial_net_acceleration_mps2,
            self.net_acceleration_step_mps2,
            self.duration_s / self.steps as f64,
            applied_steps,
        )
    }
}

impl DirectBridgePolicyV2 {
    fn dt_s(&self) -> f64 {
        1.0 / f64::from(self.physics_hz)
    }

    fn interval_ticks(&self, seconds: f64, label: &str) -> Result<u64, String> {
        let ticks = seconds * f64::from(self.physics_hz);
        let rounded = ticks.round();
        if (ticks - rounded).abs() > 1.0e-9 || rounded < 1.0 {
            return Err(format!(
                "{label} must convert to a positive whole physics tick count"
            ));
        }
        Ok(rounded as u64)
    }

    pub fn handoff_interval_ticks(&self) -> u64 {
        self.interval_ticks(self.handoff_interval_s, "handoff_interval_s")
            .expect("validated policy")
    }

    pub fn bridge_interval_ticks(&self) -> u64 {
        self.interval_ticks(
            self.bridge_duration_interval_s,
            "bridge_duration_interval_s",
        )
        .expect("validated policy")
    }

    pub fn mission_budget_s(&self) -> f64 {
        self.maximum_mission_time_s - self.mission_time_reserve_s
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.physics_hz == 0 {
            return Err("physics_hz must be positive".to_owned());
        }
        if self.duration_multipliers.len() != 4
            || self
                .duration_multipliers
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err("duration_multipliers must contain four positive finite values".to_owned());
        }
        let mut multipliers = self.duration_multipliers.clone();
        multipliers.sort_by(f64::total_cmp);
        multipliers.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-12);
        if multipliers.len() != 4 {
            return Err("duration_multipliers must be unique".to_owned());
        }
        for (label, value) in [
            ("gravity_mps2", self.gravity_mps2),
            ("minimum_clearance_m", self.minimum_clearance_m),
            ("maximum_mission_time_s", self.maximum_mission_time_s),
            ("mission_time_reserve_s", self.mission_time_reserve_s),
            ("handoff_interval_s", self.handoff_interval_s),
            (
                "bridge_duration_interval_s",
                self.bridge_duration_interval_s,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{label} must be positive and finite"));
            }
        }
        if !(0.0..=1.0).contains(&self.thrust_derate)
            || !(0.0..1.0).contains(&self.declared_robustness_margin)
            || !(0.0..=1.0).contains(&self.terminal_target_downward_speed_fraction)
            || self.mission_time_reserve_s >= self.maximum_mission_time_s
        {
            return Err("policy fractions or time reserve are invalid".to_owned());
        }
        self.interval_ticks(self.handoff_interval_s, "handoff_interval_s")?;
        if self.interval_ticks(
            self.bridge_duration_interval_s,
            "bridge_duration_interval_s",
        )? < 2
        {
            return Err("bridge_duration_interval_s must be at least two physics ticks".to_owned());
        }
        Ok(())
    }
}

impl VehicleInputV2 {
    fn worst_case_mass_kg(&self) -> f64 {
        self.dry_mass_kg + self.max_fuel_kg
    }

    fn max_acceleration_mps2(&self) -> f64 {
        self.max_thrust_n / self.worst_case_mass_kg()
    }

    fn derated_max_acceleration_mps2(&self, policy: &DirectBridgePolicyV2) -> f64 {
        policy.thrust_derate * self.max_acceleration_mps2()
    }

    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in [
            ("hull_width_m", self.geometry.hull_width_m),
            ("hull_height_m", self.geometry.hull_height_m),
            ("touchdown_half_span_m", self.geometry.touchdown_half_span_m),
            (
                "touchdown_base_offset_m",
                self.geometry.touchdown_base_offset_m,
            ),
            ("dry_mass_kg", self.dry_mass_kg),
            ("initial_fuel_kg", self.initial_fuel_kg),
            ("max_fuel_kg", self.max_fuel_kg),
            ("max_fuel_burn_kgps", self.max_fuel_burn_kgps),
            ("max_thrust_n", self.max_thrust_n),
            ("max_rotation_rate_radps", self.max_rotation_rate_radps),
            (
                "safe_touchdown_normal_speed_mps",
                self.safe_touchdown_normal_speed_mps,
            ),
            (
                "safe_touchdown_tangential_speed_mps",
                self.safe_touchdown_tangential_speed_mps,
            ),
            (
                "safe_touchdown_attitude_error_rad",
                self.safe_touchdown_attitude_error_rad,
            ),
            (
                "safe_touchdown_angular_rate_radps",
                self.safe_touchdown_angular_rate_radps,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{label} must be positive and finite"));
            }
        }
        if !(0.0..=1.0).contains(&self.min_throttle_frac) || self.initial_fuel_kg > self.max_fuel_kg
        {
            return Err("vehicle throttle or fuel fields are invalid".to_owned());
        }
        Ok(())
    }
}

impl DirectBridgeFixtureV2 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_id != FIXTURE_SCHEMA_ID || self.schema_version != FIXTURE_SCHEMA_VERSION {
            return Err("unexpected direct bridge fixture schema".to_owned());
        }
        self.policy.validate()?;
        self.vehicle.validate()?;
        if self.vehicle.derated_max_acceleration_mps2(&self.policy) <= 0.0 {
            return Err("derated thrust authority must be positive".to_owned());
        }
        let expected = [
            "clear_direct_probe",
            "long_span_probe",
            "ridge_probe",
            "long_range_probe",
        ];
        let mut ids = std::collections::BTreeSet::new();
        for case in &self.cases {
            if !ids.insert(case.id.clone()) {
                return Err(format!("duplicate probe id {}", case.id));
            }
            case.validate(&self.policy, &self.vehicle)?;
        }
        if self.cases.len() != expected.len() || expected.iter().any(|id| !ids.contains(*id)) {
            return Err("fixture must contain the four v2 direct probes".to_owned());
        }
        Ok(())
    }
}

impl DirectBridgeProbeV2 {
    pub fn validate(
        &self,
        policy: &DirectBridgePolicyV2,
        vehicle: &VehicleInputV2,
    ) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("probe id must not be empty".to_owned());
        }
        for (label, value) in [
            ("source center", self.source.center_x_m),
            ("source surface", self.source.surface_y_m),
            ("source width", self.source.width_m),
            ("target center", self.target.center_x_m),
            ("target surface", self.target.surface_y_m),
            ("target width", self.target.width_m),
            ("initial position x", self.initial_position_m.x),
            ("initial position y", self.initial_position_m.y),
            ("initial velocity x", self.initial_velocity_mps.x),
            ("initial velocity y", self.initial_velocity_mps.y),
        ] {
            if !value.is_finite() {
                return Err(format!("{label} must be finite"));
            }
        }
        if self.source.width_m <= 0.0
            || self.target.width_m <= 0.0
            || self.target.center_x_m <= self.source.center_x_m
        {
            return Err("probe pad geometry must have positive forward span".to_owned());
        }
        let terrain = terrain(self);
        terrain.validate()?;
        for (label, pad) in [("source", &self.source), ("target", &self.target)] {
            let left = pad.center_x_m - pad.width_m * 0.5;
            let right = pad.center_x_m + pad.width_m * 0.5;
            for x in [left, right] {
                let y = terrain
                    .sample_height_strict(x)
                    .map_err(|error| error.to_string())?;
                if (y - pad.surface_y_m).abs() > 1.0e-9 {
                    return Err(format!("{label} pad must be flat at its declared surface"));
                }
            }
            if self.terrain_points_m.iter().any(|point| {
                point.x > left && point.x < right && (point.y - pad.surface_y_m).abs() > 1.0e-9
            }) {
                return Err(format!(
                    "{label} pad terrain footprint must be flat at its declared surface"
                ));
            }
        }
        let expected_source = Vec2::new(
            self.source.center_x_m,
            self.source.surface_y_m + vehicle.geometry.touchdown_base_offset_m,
        );
        if distance(self.initial_position_m, expected_source) > ENDPOINT_TOLERANCE
            || self.initial_velocity_mps.length() > ENDPOINT_TOLERANCE
        {
            return Err("probe initial state must be a supported source-pad rest state".to_owned());
        }
        let release_y = self.source.surface_y_m
            + vehicle.geometry.touchdown_base_offset_m
            + policy.minimum_clearance_m;
        if release_y <= self.initial_position_m.y {
            return Err("release reference must be above supported source state".to_owned());
        }
        Ok(())
    }
}

/// Load and validate the frozen, side-by-side v2 probe fixture.
pub fn load_embedded_fixture_v2() -> DirectBridgeFixtureV2 {
    let fixture: DirectBridgeFixtureV2 =
        serde_json::from_str(FIXTURE).expect("valid v2 direct bridge fixture JSON");
    fixture
        .validate()
        .expect("valid v2 direct bridge fixture contract");
    fixture
}

fn digest<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("v2 direct bridge identity serialization");
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn terrain(case: &DirectBridgeProbeV2) -> TerrainDefinition {
    TerrainDefinition::Heightfield {
        points_m: case.terrain_points_m.clone(),
    }
}

fn distance(left: Vec2, right: Vec2) -> f64 {
    (left - right).length()
}

fn min_margin(left: MarginV2, right: MarginV2) -> MarginV2 {
    if left.normalized.total_cmp(&right.normalized).is_gt() {
        right
    } else {
        left
    }
}

fn normalize(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-12).then(|| vector * (1.0 / length))
}

fn angle_between(left: Vec2, right: Vec2) -> f64 {
    match (normalize(left), normalize(right)) {
        (Some(left), Some(right)) => {
            let dot = (left.x * right.x) + (left.y * right.y);
            dot.clamp(-1.0, 1.0).acos()
        }
        // A zero-thrust direction is not an attitude command.  The caller's
        // minimum-throttle/required-direction screen owns that rejection;
        // returning zero here avoids turning an otherwise explicit failure
        // into a NaN-dependent comparison.
        _ => 0.0,
    }
}

impl VirtualBallisticArcV2 {
    pub fn new(policy: &DirectBridgePolicyV2, start_m: Vec2, end_m: Vec2, steps: u64) -> Self {
        assert!(steps > 0, "ballistic arcs need at least one tick");
        let dt_s = policy.dt_s();
        let n = steps as f64;
        let departure_velocity_mps = Vec2::new(
            (end_m.x - start_m.x) / (n * dt_s),
            (end_m.y - start_m.y + policy.gravity_mps2 * dt_s * dt_s * n * (n + 1.0) * 0.5)
                / (n * dt_s),
        );
        let arrival_velocity_mps = Vec2::new(
            departure_velocity_mps.x,
            departure_velocity_mps.y - policy.gravity_mps2 * n * dt_s,
        );
        let apex_step = (0..=steps)
            .max_by(|left, right| {
                Self::state_at_raw(
                    start_m,
                    departure_velocity_mps,
                    dt_s,
                    policy.gravity_mps2,
                    *left,
                )
                .position_m
                .y
                .total_cmp(
                    &Self::state_at_raw(
                        start_m,
                        departure_velocity_mps,
                        dt_s,
                        policy.gravity_mps2,
                        *right,
                    )
                    .position_m
                    .y,
                )
            })
            .expect("non-empty ballistic index range");
        let mut arc = Self {
            start_m,
            end_m,
            dt_s,
            gravity_mps2: policy.gravity_mps2,
            steps,
            duration_s: n * dt_s,
            departure_velocity_mps,
            arrival_velocity_mps,
            apex_step,
            apex_position_m: Self::state_at_raw(
                start_m,
                departure_velocity_mps,
                dt_s,
                policy.gravity_mps2,
                apex_step,
            )
            .position_m,
            identity: String::new(),
        };
        arc.identity = digest(&BallisticIdentity {
            start_m: arc.start_m,
            end_m: arc.end_m,
            dt_s: arc.dt_s,
            gravity_mps2: arc.gravity_mps2,
            steps: arc.steps,
            departure_velocity_mps: arc.departure_velocity_mps,
            arrival_velocity_mps: arc.arrival_velocity_mps,
            apex_step: arc.apex_step,
            apex_position_m: arc.apex_position_m,
        });
        arc
    }

    fn state_at_raw(
        start_m: Vec2,
        departure_velocity_mps: Vec2,
        dt_s: f64,
        gravity_mps2: f64,
        step: u64,
    ) -> KinematicStateV2 {
        let k = step as f64;
        let gravity = Vec2::new(0.0, -gravity_mps2);
        KinematicStateV2 {
            position_m: start_m
                + departure_velocity_mps * (k * dt_s)
                + gravity * (dt_s * dt_s * k * (k + 1.0) * 0.5),
            velocity_mps: departure_velocity_mps + gravity * (k * dt_s),
        }
    }

    pub fn state_at(&self, step: u64) -> KinematicStateV2 {
        assert!(step <= self.steps, "ballistic sample lies within arc");
        Self::state_at_raw(
            self.start_m,
            self.departure_velocity_mps,
            self.dt_s,
            self.gravity_mps2,
            step,
        )
    }
}

#[derive(Serialize)]
struct BallisticIdentity {
    start_m: Vec2,
    end_m: Vec2,
    dt_s: f64,
    gravity_mps2: f64,
    steps: u64,
    departure_velocity_mps: Vec2,
    arrival_velocity_mps: Vec2,
    apex_step: u64,
    apex_position_m: Vec2,
}

/// Construct an exact semi-implicit powered bridge with an affine total-net
/// acceleration sequence.  This is intentionally controller-neutral: it
/// proves that the discrete plant can join two state vectors under the
/// vehicle's thrust bounds, but it does not produce control commands.
pub fn exact_discrete_bridge_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    kind: BridgeKindV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    steps: u64,
) -> Result<AnalyticalBridgeV2, String> {
    policy.validate()?;
    vehicle.validate()?;
    if steps < 2 {
        return Err("an exact bridge needs at least two physics ticks".to_owned());
    }
    if [
        start_state.position_m.x,
        start_state.position_m.y,
        start_state.velocity_mps.x,
        start_state.velocity_mps.y,
        end_state.position_m.x,
        end_state.position_m.y,
        end_state.velocity_mps.x,
        end_state.velocity_mps.y,
    ]
    .into_iter()
    .any(|value| !value.is_finite())
    {
        return Err("bridge state must be finite".to_owned());
    }

    let dt_s = policy.dt_s();
    let n = steps as f64;
    let (initial_net_acceleration_mps2, net_acceleration_step_mps2) =
        bridge_coefficients(policy, start_state, end_state, steps);

    let max_acceleration = vehicle.max_acceleration_mps2();
    let derated_max_acceleration = vehicle.derated_max_acceleration_mps2(policy);
    let mut samples = Vec::with_capacity(steps as usize);
    let mut max_required_thrust_acceleration = 0.0_f64;
    let mut minimum_nonzero_throttle = f64::INFINITY;
    let mut fuel_burn_kg = 0.0;
    let mut previous_direction: Option<(u64, Vec2)> = None;
    let mut maximum_powered_rotation_rate = 0.0_f64;

    for tick in 0..steps {
        let net_acceleration =
            initial_net_acceleration_mps2 + net_acceleration_step_mps2 * (tick as f64);
        let thrust_acceleration = net_acceleration + Vec2::new(0.0, policy.gravity_mps2);
        let thrust_magnitude = thrust_acceleration.length();
        let throttle_fraction = thrust_magnitude / max_acceleration;
        let thrust_direction_unit = normalize(thrust_acceleration);
        if thrust_magnitude > 1.0e-12 {
            max_required_thrust_acceleration =
                max_required_thrust_acceleration.max(thrust_magnitude);
            minimum_nonzero_throttle = minimum_nonzero_throttle.min(throttle_fraction);
            fuel_burn_kg += throttle_fraction * vehicle.max_fuel_burn_kgps * dt_s;
            if let (Some((previous_tick, previous)), Some(direction)) =
                (previous_direction, thrust_direction_unit)
            {
                let elapsed = (tick - previous_tick) as f64 * dt_s;
                maximum_powered_rotation_rate = maximum_powered_rotation_rate
                    .max(angle_between(previous, direction) / elapsed.max(dt_s));
            }
            previous_direction = thrust_direction_unit.map(|direction| (tick, direction));
        }

        let state = exact_affine_bridge_state(
            start_state,
            initial_net_acceleration_mps2,
            net_acceleration_step_mps2,
            dt_s,
            tick + 1,
        );
        samples.push(BridgeSampleV2 {
            tick,
            state_m: state,
            net_acceleration_mps2: net_acceleration,
            thrust_acceleration_mps2: thrust_acceleration,
            throttle_fraction,
            thrust_direction_unit,
        });
    }

    let final_state = exact_affine_bridge_state(
        start_state,
        initial_net_acceleration_mps2,
        net_acceleration_step_mps2,
        dt_s,
        steps,
    );
    let endpoint_position_error_m = distance(final_state.position_m, end_state.position_m);
    let endpoint_velocity_error_mps = distance(final_state.velocity_mps, end_state.velocity_mps);
    let endpoint_error = endpoint_position_error_m.max(endpoint_velocity_error_mps);
    let mut margins = ComponentMarginsV2::unbounded();
    margins.coupled_thrust =
        MarginV2::upper(derated_max_acceleration, max_required_thrust_acceleration);
    margins.minimum_throttle = if minimum_nonzero_throttle.is_finite() {
        MarginV2::lower(minimum_nonzero_throttle, vehicle.min_throttle_frac)
    } else {
        MarginV2::failed()
    };
    margins.bridge_endpoint = MarginV2::upper_relative(ENDPOINT_TOLERANCE, endpoint_error);
    margins.powered_slew = MarginV2::upper(
        vehicle.max_rotation_rate_radps,
        maximum_powered_rotation_rate,
    );

    let mut reasons = Vec::new();
    if !margins.coupled_thrust.passes(policy) {
        reasons.push(DirectBridgeReasonV2::CoupledThrust);
    }
    if !margins.minimum_throttle.passes(policy) {
        reasons.push(DirectBridgeReasonV2::MinimumThrottle);
    }
    if !margins.bridge_endpoint.passes(policy) {
        reasons.push(DirectBridgeReasonV2::BridgeEndpoint);
    }
    if !margins.powered_slew.passes(policy) {
        reasons.push(DirectBridgeReasonV2::PoweredSlew);
    }
    reasons.sort_unstable();
    reasons.dedup();
    let classification = if reasons.is_empty() {
        CertificationV2::Certified
    } else {
        CertificationV2::NotCertified
    };
    let mut bridge = AnalyticalBridgeV2 {
        kind,
        start_state,
        end_state,
        steps,
        duration_s: n * dt_s,
        initial_net_acceleration_mps2,
        net_acceleration_step_mps2,
        samples,
        endpoint_position_error_m,
        endpoint_velocity_error_mps,
        fuel_burn_kg,
        classification,
        reasons,
        margins,
        identity: String::new(),
    };
    bridge.identity = digest(&BridgeIdentity {
        kind: bridge.kind,
        start_state: bridge.start_state,
        end_state: bridge.end_state,
        steps: bridge.steps,
        initial_net_acceleration_mps2: bridge.initial_net_acceleration_mps2,
        net_acceleration_step_mps2: bridge.net_acceleration_step_mps2,
        endpoint_position_error_m: bridge.endpoint_position_error_m,
        endpoint_velocity_error_mps: bridge.endpoint_velocity_error_mps,
        fuel_burn_kg: bridge.fuel_burn_kg,
        classification: bridge.classification,
        reasons: &bridge.reasons,
        margins: bridge.margins,
    });
    Ok(bridge)
}

#[derive(Serialize)]
struct BridgeIdentity<'a> {
    kind: BridgeKindV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    steps: u64,
    initial_net_acceleration_mps2: Vec2,
    net_acceleration_step_mps2: Vec2,
    endpoint_position_error_m: f64,
    endpoint_velocity_error_mps: f64,
    fuel_burn_kg: f64,
    classification: CertificationV2,
    reasons: &'a [DirectBridgeReasonV2],
    margins: ComponentMarginsV2,
}

#[derive(Default)]
struct AttemptSummaryV2 {
    count: usize,
    streamed_tick_count: usize,
    environment_rejection_count: usize,
    reasons: std::collections::BTreeSet<DirectBridgeReasonV2>,
}

#[derive(Clone)]
struct CompactBridgeAssessmentV2 {
    kind: BridgeKindV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    steps: u64,
    duration_s: f64,
    initial_net_acceleration_mps2: Vec2,
    net_acceleration_step_mps2: Vec2,
    endpoint_position_error_m: f64,
    endpoint_velocity_error_mps: f64,
    fuel_burn_kg: f64,
    classification: CertificationV2,
    reasons: Vec<DirectBridgeReasonV2>,
    margins: ComponentMarginsV2,
    environment: BridgeEnvironmentEvidenceV2,
}

#[derive(Clone)]
struct BridgeOptionV2 {
    handoff: HandoffV2,
    bridge: CompactBridgeAssessmentV2,
}

struct PairSelectionV2 {
    source_option_index: usize,
    terminal_option_index: usize,
    handoff_tuple: (u64, u64, u64, u64),
    coast: CoastEvidenceV2,
    classification: CertificationV2,
    reasons: Vec<DirectBridgeReasonV2>,
    margins: ComponentMarginsV2,
    total_fuel_burn_kg: f64,
    total_time_s: f64,
}

fn candidate_steps(policy: &DirectBridgePolicyV2, horizontal_span_m: f64) -> Vec<(f64, u64)> {
    let nominal_s = (2.0 * horizontal_span_m / policy.gravity_mps2).sqrt();
    let mut steps: Vec<(f64, u64)> = policy
        .duration_multipliers
        .iter()
        .copied()
        .map(|multiplier| {
            (
                multiplier,
                (nominal_s * multiplier * f64::from(policy.physics_hz))
                    .round()
                    .max(1.0) as u64,
            )
        })
        .collect();
    steps.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| left.0.total_cmp(&right.0))
    });
    steps.dedup_by(|left, right| left.1 == right.1);
    steps
}

fn candidate_steps_for_multiplier(
    policy: &DirectBridgePolicyV2,
    horizontal_span_m: f64,
    multiplier: f64,
) -> Result<u64, String> {
    let nominal_s = (2.0 * horizontal_span_m / policy.gravity_mps2).sqrt();
    let ticks = nominal_s * multiplier * f64::from(policy.physics_hz);
    if !ticks.is_finite() || ticks >= u64::MAX as f64 {
        return Err("waypoint leg duration exceeds the supported tick range".to_owned());
    }
    Ok(ticks.round().max(1.0) as u64)
}

fn bridge_duration_steps(policy: &DirectBridgePolicyV2) -> Vec<u64> {
    let interval = policy.bridge_interval_ticks();
    let maximum = (policy.mission_budget_s() * f64::from(policy.physics_hz)).floor() as u64;
    (interval..=maximum).step_by(interval as usize).collect()
}

fn release_reference(
    case: &DirectBridgeProbeV2,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> Vec2 {
    Vec2::new(
        case.source.center_x_m,
        case.source.surface_y_m
            + vehicle.geometry.touchdown_base_offset_m
            + policy.minimum_clearance_m,
    )
}

fn touchdown_reference(case: &DirectBridgeProbeV2, vehicle: &VehicleInputV2) -> Vec2 {
    Vec2::new(
        case.target.center_x_m,
        case.target.surface_y_m + vehicle.geometry.touchdown_base_offset_m,
    )
}

fn source_handoff_steps(arc: &VirtualBallisticArcV2, interval: u64) -> Vec<u64> {
    let mut steps = std::collections::BTreeSet::new();
    let last_ascending = (1..=arc.apex_step)
        .rev()
        .find(|step| arc.state_at(*step).velocity_mps.y > 0.0);
    if let Some(last) = last_ascending {
        steps.insert(1);
        for step in (interval..=last).step_by(interval as usize) {
            steps.insert(step);
        }
        steps.insert(last);
    }
    steps.into_iter().collect()
}

fn terminal_handoff_steps(arc: &VirtualBallisticArcV2, interval: u64) -> Vec<u64> {
    let mut steps = std::collections::BTreeSet::new();
    let first_descending =
        (arc.apex_step..arc.steps).find(|step| arc.state_at(*step).velocity_mps.y < 0.0);
    if let Some(first) = first_descending {
        steps.insert(first);
        let last = arc.steps.saturating_sub(1);
        for step in (first..=last).filter(|step| *step % interval == 0) {
            steps.insert(step);
        }
        steps.insert(last);
    }
    steps.into_iter().collect()
}

fn bridge_coefficients(
    policy: &DirectBridgePolicyV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    steps: u64,
) -> (Vec2, Vec2) {
    let dt_s = policy.dt_s();
    let n = steps as f64;
    let d = (end_state.velocity_mps - start_state.velocity_mps) * (1.0 / dt_s);
    let s = (end_state.position_m - start_state.position_m - start_state.velocity_mps * (n * dt_s))
        * (1.0 / (dt_s * dt_s));
    let initial = s * (6.0 / (n * (n + 1.0))) - d * (2.0 / n);
    let delta = d * (6.0 / (n * (n - 1.0))) - s * (12.0 / (n * (n + 1.0) * (n - 1.0)));
    (initial, delta)
}

/// State after `applied_steps` semi-implicit updates of
/// `a[j] = initial_net_acceleration_mps2 + net_acceleration_step_mps2 * j`.
///
/// A bridge sample with zero-based tick `j` is the state after `j + 1`
/// updates.  Keeping this closed form shared by materialized and compact
/// evaluation prevents long-horizon recurrence drift from changing terrain
/// evidence or endpoint certification.
fn exact_affine_bridge_state(
    start_state: KinematicStateV2,
    initial_net_acceleration_mps2: Vec2,
    net_acceleration_step_mps2: Vec2,
    dt_s: f64,
    applied_steps: u64,
) -> KinematicStateV2 {
    let k = applied_steps as f64;
    let velocity_acceleration_sum =
        initial_net_acceleration_mps2 * k + net_acceleration_step_mps2 * (k * (k - 1.0) * 0.5);
    let position_acceleration_sum = initial_net_acceleration_mps2 * (k * (k + 1.0) * 0.5)
        + net_acceleration_step_mps2 * (k * (k + 1.0) * (k - 1.0) / 6.0);
    KinematicStateV2 {
        velocity_mps: start_state.velocity_mps + velocity_acceleration_sum * dt_s,
        position_m: start_state.position_m
            + start_state.velocity_mps * (k * dt_s)
            + position_acceleration_sum * (dt_s * dt_s),
    }
}

fn primitive_precheck_failure(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    steps: u64,
) -> Option<DirectBridgeReasonV2> {
    let (initial, delta) = bridge_coefficients(policy, start_state, end_state, steps);
    let first = initial + Vec2::new(0.0, policy.gravity_mps2);
    let last =
        initial + delta * (steps.saturating_sub(1) as f64) + Vec2::new(0.0, policy.gravity_mps2);
    let peak = first.length().max(last.length());
    let robust_limit =
        vehicle.derated_max_acceleration_mps2(policy) * (1.0 - policy.declared_robustness_margin);
    if peak > robust_limit {
        return Some(DirectBridgeReasonV2::CoupledThrust);
    }

    let thrust_at = |tick: u64| first + delta * (tick as f64);
    let delta_squared = (delta.x * delta.x) + (delta.y * delta.y);
    let closest_magnitude_tick = if delta_squared <= 1.0e-24 {
        0.0
    } else {
        -((first.x * delta.x) + (first.y * delta.y)) / delta_squared
    };
    let mut magnitude_ticks = vec![0, steps - 1];
    for tick in [
        closest_magnitude_tick.floor(),
        closest_magnitude_tick.ceil(),
    ] {
        if tick.is_finite() && tick >= 0.0 && tick <= (steps - 1) as f64 {
            magnitude_ticks.push(tick as u64);
        }
    }
    magnitude_ticks.sort_unstable();
    magnitude_ticks.dedup();
    let minimum_throttle = magnitude_ticks
        .into_iter()
        .map(|tick| thrust_at(tick).length() / vehicle.max_acceleration_mps2())
        .fold(f64::INFINITY, f64::min);
    if !MarginV2::lower(minimum_throttle, vehicle.min_throttle_frac).passes(policy) {
        return Some(DirectBridgeReasonV2::MinimumThrottle);
    }

    let closest_slew_tick = if delta_squared <= 1.0e-24 {
        0.0
    } else {
        -((first.x * delta.x) + (first.y * delta.y)) / delta_squared - 0.5
    };
    let mut slew_ticks = vec![0, steps - 2];
    for tick in [closest_slew_tick.floor(), closest_slew_tick.ceil()] {
        if tick.is_finite() && tick >= 0.0 && tick <= (steps - 2) as f64 {
            slew_ticks.push(tick as u64);
        }
    }
    slew_ticks.sort_unstable();
    slew_ticks.dedup();
    let maximum_slew = slew_ticks
        .into_iter()
        .map(|tick| angle_between(thrust_at(tick), thrust_at(tick + 1)) / policy.dt_s())
        .fold(0.0_f64, f64::max);
    if !MarginV2::upper(vehicle.max_rotation_rate_radps, maximum_slew).passes(policy) {
        return Some(DirectBridgeReasonV2::PoweredSlew);
    }
    None
}

fn local_attitude_precheck_failure(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    kind: BridgeKindV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    steps: u64,
) -> Option<DirectBridgeReasonV2> {
    let (initial, delta) = bridge_coefficients(policy, start_state, end_state, steps);
    let first = initial + Vec2::new(0.0, policy.gravity_mps2);
    let final_direction = first + delta * (steps - 1) as f64;
    let up = Vec2::new(0.0, 1.0);
    match kind {
        BridgeKindV2::Source => (!MarginV2::upper(
            vehicle.safe_touchdown_attitude_error_rad,
            angle_between(up, first),
        )
        .passes(policy))
        .then_some(DirectBridgeReasonV2::SourceAttitude),
        BridgeKindV2::Terminal => {
            if !MarginV2::upper(
                vehicle.safe_touchdown_attitude_error_rad,
                angle_between(up, final_direction),
            )
            .passes(policy)
            {
                return Some(DirectBridgeReasonV2::TouchdownAttitude);
            }
            let previous = first + delta * (steps - 2) as f64;
            let final_rate = angle_between(previous, final_direction) / policy.dt_s();
            (!MarginV2::upper(vehicle.safe_touchdown_angular_rate_radps, final_rate).passes(policy))
                .then_some(DirectBridgeReasonV2::TouchdownAngularRate)
        }
        BridgeKindV2::Intermediate => None,
    }
}

/// Stream one exact bridge recurrence after the cheap primitive and local
/// attitude screens have passed. The assessment deliberately keeps only the
/// scalar evidence needed for option ranking; the full per-tick trace is
/// materialized only for the winning source and terminal options.
#[allow(clippy::too_many_arguments)]
fn compact_bridge_assessment(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    terrain: &ValidatedHeightfieldV2<'_>,
    kind: BridgeKindV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    steps: u64,
    support_pad: Option<&PadInputV2>,
    attempts: &mut AttemptSummaryV2,
) -> Result<CompactBridgeAssessmentV2, DirectBridgeReasonV2> {
    assert!(
        steps >= 2,
        "compact bridge needs at least two physics ticks"
    );
    let dt_s = policy.dt_s();
    let n = steps as f64;
    let (initial_net_acceleration_mps2, net_acceleration_step_mps2) =
        bridge_coefficients(policy, start_state, end_state, steps);
    let max_acceleration = vehicle.max_acceleration_mps2();
    let derated_max_acceleration = vehicle.derated_max_acceleration_mps2(policy);
    let mut max_required_thrust_acceleration = 0.0_f64;
    let mut minimum_nonzero_throttle = f64::INFINITY;
    let mut fuel_burn_kg = 0.0;
    let mut previous_powered_direction: Option<(u64, Vec2)> = None;
    let mut previous_to_last_powered_direction: Option<(u64, Vec2)> = None;
    let mut first_powered_direction: Option<(u64, Vec2)> = None;
    let mut final_powered_direction: Option<(u64, Vec2)> = None;
    let mut maximum_powered_rotation_rate = 0.0_f64;
    let mut clearance_margin = MarginV2::unbounded();
    let mut pad_corridor_tick_count = 0;
    let mut free_flight_tick_count = 0;

    for tick in 0..steps {
        attempts.streamed_tick_count += 1;
        let net_acceleration =
            initial_net_acceleration_mps2 + net_acceleration_step_mps2 * (tick as f64);
        let thrust_acceleration = net_acceleration + Vec2::new(0.0, policy.gravity_mps2);
        let thrust_magnitude = thrust_acceleration.length();
        let throttle_fraction = thrust_magnitude / max_acceleration;
        let thrust_direction_unit = normalize(thrust_acceleration);
        if thrust_magnitude > 1.0e-12 {
            max_required_thrust_acceleration =
                max_required_thrust_acceleration.max(thrust_magnitude);
            minimum_nonzero_throttle = minimum_nonzero_throttle.min(throttle_fraction);
            fuel_burn_kg += throttle_fraction * vehicle.max_fuel_burn_kgps * dt_s;
            if let Some((previous_tick, previous_direction)) = previous_powered_direction {
                let elapsed = (tick - previous_tick) as f64 * dt_s;
                maximum_powered_rotation_rate = maximum_powered_rotation_rate.max(
                    angle_between(
                        previous_direction,
                        thrust_direction_unit.expect("nonzero thrust"),
                    ) / elapsed.max(dt_s),
                );
                previous_to_last_powered_direction = Some((previous_tick, previous_direction));
            }
            let direction = thrust_direction_unit.expect("nonzero thrust has direction");
            first_powered_direction.get_or_insert((tick, direction));
            final_powered_direction = Some((tick, direction));
            previous_powered_direction = Some((tick, direction));
        }

        let state = exact_affine_bridge_state(
            start_state,
            initial_net_acceleration_mps2,
            net_acceleration_step_mps2,
            dt_s,
            tick + 1,
        );
        let (margin, in_pad_corridor) = clearance_for_state_fast(
            terrain,
            state,
            thrust_direction_unit,
            support_pad,
            policy,
            vehicle,
        );
        clearance_margin = min_margin(clearance_margin, margin);
        // A bridge cannot recover from one per-tick terrain-margin failure.
        // Do not retain partial scalar evidence: rejected options are not
        // public certificate material, and the caller records the reason.
        if !margin.passes(policy) {
            return Err(DirectBridgeReasonV2::TerrainClearance);
        }
        if in_pad_corridor {
            pad_corridor_tick_count += 1;
        } else {
            free_flight_tick_count += 1;
        }
    }

    let final_state = exact_affine_bridge_state(
        start_state,
        initial_net_acceleration_mps2,
        net_acceleration_step_mps2,
        dt_s,
        steps,
    );
    let endpoint_position_error_m = distance(final_state.position_m, end_state.position_m);
    let endpoint_velocity_error_mps = distance(final_state.velocity_mps, end_state.velocity_mps);
    let endpoint_error = endpoint_position_error_m.max(endpoint_velocity_error_mps);
    let mut margins = ComponentMarginsV2::unbounded();
    margins.coupled_thrust =
        MarginV2::upper(derated_max_acceleration, max_required_thrust_acceleration);
    margins.minimum_throttle = if minimum_nonzero_throttle.is_finite() {
        MarginV2::lower(minimum_nonzero_throttle, vehicle.min_throttle_frac)
    } else {
        MarginV2::failed()
    };
    margins.bridge_endpoint = MarginV2::upper_relative(ENDPOINT_TOLERANCE, endpoint_error);
    margins.powered_slew = MarginV2::upper(
        vehicle.max_rotation_rate_radps,
        maximum_powered_rotation_rate,
    );

    let mut reasons = Vec::new();
    if !margins.coupled_thrust.passes(policy) {
        reasons.push(DirectBridgeReasonV2::CoupledThrust);
    }
    if !margins.minimum_throttle.passes(policy) {
        reasons.push(DirectBridgeReasonV2::MinimumThrottle);
    }
    if !margins.bridge_endpoint.passes(policy) {
        reasons.push(DirectBridgeReasonV2::BridgeEndpoint);
    }
    if !margins.powered_slew.passes(policy) {
        reasons.push(DirectBridgeReasonV2::PoweredSlew);
    }
    reasons.sort_unstable();
    reasons.dedup();

    let up = Vec2::new(0.0, 1.0);
    let initial_attitude_margin = if kind == BridgeKindV2::Source {
        first_powered_direction.map_or(MarginV2::failed(), |(_, direction)| {
            MarginV2::upper(
                vehicle.safe_touchdown_attitude_error_rad,
                angle_between(up, direction),
            )
        })
    } else {
        MarginV2::unbounded()
    };
    let final_attitude_margin = if kind == BridgeKindV2::Terminal {
        final_powered_direction.map_or(MarginV2::failed(), |(_, direction)| {
            MarginV2::upper(
                vehicle.safe_touchdown_attitude_error_rad,
                angle_between(up, direction),
            )
        })
    } else {
        MarginV2::unbounded()
    };
    let final_angular_rate_margin = if kind == BridgeKindV2::Terminal {
        match (previous_to_last_powered_direction, final_powered_direction) {
            (Some((previous_tick, previous)), Some((last_tick, last))) => {
                let rate = angle_between(previous, last)
                    / ((last_tick - previous_tick) as f64 * (n * dt_s / n));
                MarginV2::upper(vehicle.safe_touchdown_angular_rate_radps, rate)
            }
            _ => MarginV2::failed(),
        }
    } else {
        MarginV2::unbounded()
    };
    let environment = BridgeEnvironmentEvidenceV2 {
        clearance_margin,
        pad_corridor_tick_count,
        free_flight_tick_count,
        first_powered_direction_unit: first_powered_direction.map(|(_, direction)| direction),
        final_powered_direction_unit: final_powered_direction.map(|(_, direction)| direction),
        initial_attitude_margin,
        final_attitude_margin,
        final_angular_rate_margin,
    };
    Ok(CompactBridgeAssessmentV2 {
        kind,
        start_state,
        end_state,
        steps,
        duration_s: n * dt_s,
        initial_net_acceleration_mps2,
        net_acceleration_step_mps2,
        endpoint_position_error_m,
        endpoint_velocity_error_mps,
        fuel_burn_kg,
        classification: if reasons.is_empty() {
            CertificationV2::Certified
        } else {
            CertificationV2::NotCertified
        },
        reasons,
        margins,
        environment,
    })
}

const COMPACT_EQUIVALENCE_TOLERANCE: f64 = 1.0e-10;

fn scalar_matches(left: f64, right: f64) -> bool {
    (left - right).abs() <= COMPACT_EQUIVALENCE_TOLERANCE * left.abs().max(right.abs()).max(1.0)
}

fn assert_margin_matches(label: &str, compact: MarginV2, materialized: MarginV2) {
    assert!(
        scalar_matches(compact.raw, materialized.raw)
            && scalar_matches(compact.normalized, materialized.normalized),
        "compact/materialized {label} margin mismatch: {compact:?} != {materialized:?}"
    );
}

fn assert_vec2_matches(label: &str, compact: Vec2, materialized: Vec2) {
    assert!(
        scalar_matches(compact.x, materialized.x) && scalar_matches(compact.y, materialized.y),
        "compact/materialized {label} mismatch: {compact:?} != {materialized:?}"
    );
}

fn assert_optional_vec2_matches(label: &str, compact: Option<Vec2>, materialized: Option<Vec2>) {
    match (compact, materialized) {
        (Some(compact), Some(materialized)) => assert_vec2_matches(label, compact, materialized),
        (None, None) => {}
        (compact, materialized) => {
            panic!(
                "compact/materialized {label} presence mismatch: {compact:?} != {materialized:?}"
            )
        }
    }
}

fn assert_component_margins_match(compact: ComponentMarginsV2, materialized: ComponentMarginsV2) {
    for (label, compact, materialized) in [
        (
            "coupled_thrust",
            compact.coupled_thrust,
            materialized.coupled_thrust,
        ),
        (
            "minimum_throttle",
            compact.minimum_throttle,
            materialized.minimum_throttle,
        ),
        ("clearance", compact.clearance, materialized.clearance),
        (
            "bridge_endpoint",
            compact.bridge_endpoint,
            materialized.bridge_endpoint,
        ),
        (
            "powered_slew",
            compact.powered_slew,
            materialized.powered_slew,
        ),
        (
            "source_attitude",
            compact.source_attitude,
            materialized.source_attitude,
        ),
        ("coast_slew", compact.coast_slew, materialized.coast_slew),
        ("fuel", compact.fuel, materialized.fuel),
        ("time", compact.time, materialized.time),
        (
            "touchdown_speed",
            compact.touchdown_speed,
            materialized.touchdown_speed,
        ),
        (
            "touchdown_attitude",
            compact.touchdown_attitude,
            materialized.touchdown_attitude,
        ),
        (
            "touchdown_angular_rate",
            compact.touchdown_angular_rate,
            materialized.touchdown_angular_rate,
        ),
    ] {
        assert_margin_matches(label, compact, materialized);
    }
}

fn assert_environment_matches(
    compact: &BridgeEnvironmentEvidenceV2,
    materialized: &BridgeEnvironmentEvidenceV2,
) {
    assert_margin_matches(
        "environment clearance",
        compact.clearance_margin,
        materialized.clearance_margin,
    );
    assert_eq!(
        compact.pad_corridor_tick_count,
        materialized.pad_corridor_tick_count
    );
    assert_eq!(
        compact.free_flight_tick_count,
        materialized.free_flight_tick_count
    );
    assert_optional_vec2_matches(
        "first powered direction",
        compact.first_powered_direction_unit,
        materialized.first_powered_direction_unit,
    );
    assert_optional_vec2_matches(
        "final powered direction",
        compact.final_powered_direction_unit,
        materialized.final_powered_direction_unit,
    );
    assert_margin_matches(
        "initial attitude",
        compact.initial_attitude_margin,
        materialized.initial_attitude_margin,
    );
    assert_margin_matches(
        "final attitude",
        compact.final_attitude_margin,
        materialized.final_attitude_margin,
    );
    assert_margin_matches(
        "final angular rate",
        compact.final_angular_rate_margin,
        materialized.final_angular_rate_margin,
    );
}

fn assert_compact_bridge_matches(
    compact: &CompactBridgeAssessmentV2,
    materialized: &AnalyticalBridgeV2,
    materialized_environment: &BridgeEnvironmentEvidenceV2,
) {
    assert_eq!(compact.kind, materialized.kind);
    assert_vec2_matches(
        "start position",
        compact.start_state.position_m,
        materialized.start_state.position_m,
    );
    assert_vec2_matches(
        "start velocity",
        compact.start_state.velocity_mps,
        materialized.start_state.velocity_mps,
    );
    assert_vec2_matches(
        "end position",
        compact.end_state.position_m,
        materialized.end_state.position_m,
    );
    assert_vec2_matches(
        "end velocity",
        compact.end_state.velocity_mps,
        materialized.end_state.velocity_mps,
    );
    assert_eq!(compact.steps, materialized.steps);
    assert!(scalar_matches(compact.duration_s, materialized.duration_s));
    assert_vec2_matches(
        "initial net acceleration",
        compact.initial_net_acceleration_mps2,
        materialized.initial_net_acceleration_mps2,
    );
    assert_vec2_matches(
        "net acceleration step",
        compact.net_acceleration_step_mps2,
        materialized.net_acceleration_step_mps2,
    );
    assert!(scalar_matches(
        compact.endpoint_position_error_m,
        materialized.endpoint_position_error_m
    ));
    assert!(scalar_matches(
        compact.endpoint_velocity_error_mps,
        materialized.endpoint_velocity_error_mps
    ));
    assert!(scalar_matches(
        compact.fuel_burn_kg,
        materialized.fuel_burn_kg
    ));
    assert_eq!(compact.classification, materialized.classification);
    assert_eq!(compact.reasons, materialized.reasons);
    assert_component_margins_match(compact.margins, materialized.margins);
    assert_environment_matches(&compact.environment, materialized_environment);
}

fn environment_failure_reasons(
    kind: BridgeKindV2,
    environment: &BridgeEnvironmentEvidenceV2,
    policy: &DirectBridgePolicyV2,
) -> std::collections::BTreeSet<DirectBridgeReasonV2> {
    let mut reasons = std::collections::BTreeSet::new();
    if !environment.clearance_margin.passes(policy) {
        reasons.insert(DirectBridgeReasonV2::TerrainClearance);
    }
    match kind {
        BridgeKindV2::Source if !environment.initial_attitude_margin.passes(policy) => {
            reasons.insert(DirectBridgeReasonV2::SourceAttitude);
        }
        BridgeKindV2::Terminal => {
            if !environment.final_attitude_margin.passes(policy) {
                reasons.insert(DirectBridgeReasonV2::TouchdownAttitude);
            }
            if !environment.final_angular_rate_margin.passes(policy) {
                reasons.insert(DirectBridgeReasonV2::TouchdownAngularRate);
            }
        }
        _ => {}
    }
    reasons
}

#[allow(clippy::too_many_arguments)]
fn find_shortest_primitive_bridge(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    terrain: &ValidatedHeightfieldV2<'_>,
    kind: BridgeKindV2,
    start_state: KinematicStateV2,
    end_state: KinematicStateV2,
    support_pad: Option<&PadInputV2>,
    duration_steps: &[u64],
    attempts: &mut AttemptSummaryV2,
) -> Option<CompactBridgeAssessmentV2> {
    for &steps in duration_steps {
        attempts.count += 1;
        if let Some(reason) =
            primitive_precheck_failure(policy, vehicle, start_state, end_state, steps)
        {
            attempts.reasons.insert(reason);
            continue;
        }
        if let Some(reason) =
            local_attitude_precheck_failure(policy, vehicle, kind, start_state, end_state, steps)
        {
            attempts.environment_rejection_count += 1;
            attempts.reasons.insert(reason);
            continue;
        }
        let bridge = match compact_bridge_assessment(
            policy,
            vehicle,
            terrain,
            kind,
            start_state,
            end_state,
            steps,
            support_pad,
            attempts,
        ) {
            Ok(bridge) => bridge,
            Err(reason) => {
                attempts.environment_rejection_count += 1;
                attempts.reasons.insert(reason);
                continue;
            }
        };
        if bridge.classification != CertificationV2::Certified {
            attempts.reasons.extend(bridge.reasons.iter().copied());
            continue;
        }
        let environment_reasons = environment_failure_reasons(kind, &bridge.environment, policy);
        if environment_reasons.is_empty() {
            return Some(bridge);
        }
        attempts.environment_rejection_count += 1;
        attempts.reasons.extend(environment_reasons);
    }
    None
}

fn attitude_rad(direction: Vec2) -> f64 {
    (-direction.x).atan2(direction.y)
}

fn complete_touchdown_footprint_on_pad(
    state: KinematicStateV2,
    direction: Vec2,
    pad: &PadInputV2,
    vehicle: &VehicleInputV2,
) -> bool {
    let attitude = attitude_rad(direction);
    let left = state.position_m
        + Vec2::new(
            -vehicle.geometry.touchdown_half_span_m,
            -vehicle.geometry.touchdown_base_offset_m,
        )
        .rotated(attitude);
    let right = state.position_m
        + Vec2::new(
            vehicle.geometry.touchdown_half_span_m,
            -vehicle.geometry.touchdown_base_offset_m,
        )
        .rotated(attitude);
    let pad_left = pad.center_x_m - pad.width_m * 0.5;
    let pad_right = pad.center_x_m + pad.width_m * 0.5;
    left.x >= pad_left - ENDPOINT_TOLERANCE
        && left.x <= pad_right + ENDPOINT_TOLERANCE
        && right.x >= pad_left - ENDPOINT_TOLERANCE
        && right.x <= pad_right + ENDPOINT_TOLERANCE
}

fn rotated_hull_envelope(
    direction: Option<Vec2>,
    vehicle: &VehicleInputV2,
    extra_vertical_clearance_m: f64,
) -> CorridorEnvelope {
    let half_width = vehicle.geometry.hull_width_m * 0.5;
    let half_height = vehicle.geometry.hull_height_m * 0.5;
    match direction {
        Some(direction) => {
            let angle = attitude_rad(direction);
            let (sin, cos) = angle.sin_cos();
            let horizontal = (cos.abs() * half_width) + (sin.abs() * half_height);
            let vertical = (sin.abs() * half_width) + (cos.abs() * half_height);
            CorridorEnvelope::new(horizontal, vertical + extra_vertical_clearance_m)
        }
        None => {
            let radius = (half_width.mul_add(half_width, half_height * half_height)).sqrt();
            CorridorEnvelope::new(radius, radius + extra_vertical_clearance_m)
        }
    }
}

/// Borrowed terrain facts whose one-time validation is shared by compact
/// bridge and coast screening.  The materialized evidence path intentionally
/// continues to use `TerrainDefinition::exact_point_clearance` below.
struct ValidatedHeightfieldV2<'a> {
    points: &'a [Vec2],
    domain_min_x_m: f64,
    domain_max_x_m: f64,
}

impl<'a> ValidatedHeightfieldV2<'a> {
    fn new(terrain: &'a TerrainDefinition) -> Result<Self, String> {
        terrain.validate()?;
        let points = terrain.points();
        // `TerrainDefinition::validate` proves these indexes are present and
        // finite. Keeping the assertion here documents that this is a cache
        // constructor, not a second terrain contract.
        let first = points
            .first()
            .expect("validated heightfield has a first point");
        let last = points
            .last()
            .expect("validated heightfield has a last point");
        Ok(Self {
            points,
            domain_min_x_m: first.x,
            domain_max_x_m: last.x,
        })
    }

    fn sample_height(&self, x_m: f64) -> f64 {
        // This is the same segment choice and interpolation as
        // `TerrainDefinition::sample_height`, without re-matching the terrain
        // enum for every compact physics tick.
        if x_m <= self.domain_min_x_m {
            return self.points[0].y;
        }
        if x_m >= self.domain_max_x_m {
            return self.points[self.points.len() - 1].y;
        }
        // `sample_height` uses `binary_search` and deliberately chooses the
        // segment immediately left of an exact interior vertex.  A strict
        // lower bound has the same behavior for both exact and in-segment x.
        let first_not_less = self.points.partition_point(|point| point.x < x_m);
        let segment = first_not_less.saturating_sub(1).min(self.points.len() - 2);
        let p0 = self.points[segment];
        let p1 = self.points[segment + 1];
        let dx = p1.x - p0.x;
        if dx.abs() <= f64::EPSILON {
            p0.y
        } else {
            let t = ((x_m - p0.x) / dx).clamp(0.0, 1.0);
            p0.y + ((p1.y - p0.y) * t)
        }
    }

    /// Exact point clearance under the already-validated heightfield
    /// contract. The maximum terrain height over a closed span occurs at the
    /// sampled endpoints or an included terrain vertex, exactly as in the core
    /// point query, but no candidate vector is allocated.
    fn exact_point_clearance(&self, center_m: Vec2, envelope: CorridorEnvelope) -> Option<f64> {
        if !center_m.x.is_finite()
            || !center_m.y.is_finite()
            || !envelope.horizontal_extent_m.is_finite()
            || envelope.horizontal_extent_m < 0.0
            || !envelope.vertical_extent_m.is_finite()
            || envelope.vertical_extent_m < 0.0
        {
            return None;
        }
        let left = center_m.x - envelope.horizontal_extent_m;
        let right = center_m.x + envelope.horizontal_extent_m;
        if !left.is_finite()
            || !right.is_finite()
            || left < self.domain_min_x_m
            || right > self.domain_max_x_m
        {
            return None;
        }
        let required_y = center_m.y - envelope.vertical_extent_m;
        if !required_y.is_finite() {
            return None;
        }
        let mut maximum_terrain_y = self.sample_height(left).max(self.sample_height(right));
        let included_start = self.points.partition_point(|vertex| vertex.x < left);
        let included_end = self.points.partition_point(|vertex| vertex.x <= right);
        for vertex in &self.points[included_start..included_end] {
            if vertex.x >= left && vertex.x <= right {
                maximum_terrain_y = maximum_terrain_y.max(vertex.y);
            }
        }
        maximum_terrain_y
            .is_finite()
            .then_some(required_y - maximum_terrain_y)
    }
}

/// A declared flat pad provides a legal vertical launch/landing corridor while
/// the complete touchdown footprint remains over it. The hull base must not
/// penetrate the pad plane, but the general free-flight reserve begins only
/// after the footprint leaves the pad. Exact touchdown contact and attitude
/// are governed separately by the endpoint and touchdown screens.
fn pad_clearance_height_margin(
    state: KinematicStateV2,
    pad: &PadInputV2,
    vehicle: &VehicleInputV2,
) -> MarginV2 {
    let nominal_center_y = pad.surface_y_m + vehicle.geometry.touchdown_base_offset_m;
    let raw = state.position_m.y - nominal_center_y + ENDPOINT_TOLERANCE;
    if !raw.is_finite() {
        return MarginV2::failed();
    }
    MarginV2 {
        raw,
        normalized: raw / ENDPOINT_TOLERANCE,
    }
}

fn pad_corridor_for_state<'a>(
    state: KinematicStateV2,
    direction: Option<Vec2>,
    support_pad: Option<&'a PadInputV2>,
    vehicle: &VehicleInputV2,
) -> Option<&'a PadInputV2> {
    direction.and_then(|direction| {
        support_pad
            .filter(|pad| complete_touchdown_footprint_on_pad(state, direction, pad, vehicle))
    })
}

fn clearance_for_state(
    terrain: &TerrainDefinition,
    state: KinematicStateV2,
    direction: Option<Vec2>,
    support_pad: Option<&PadInputV2>,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> (MarginV2, bool) {
    if let Some(pad) = pad_corridor_for_state(state, direction, support_pad, vehicle) {
        return (pad_clearance_height_margin(state, pad, vehicle), true);
    }
    // Outside a complete legal contact footprint, this remains an airborne
    // body-clearance screen: the rigid rotated hull and policy clearance must
    // clear authoritative terrain without a pad-contact exception.
    let envelope = rotated_hull_envelope(direction, vehicle, policy.minimum_clearance_m);
    match terrain.exact_point_clearance(state.position_m, envelope) {
        Ok(clearance) => (
            MarginV2::lower_scaled(
                clearance.minimum_clearance_m,
                0.0,
                policy.minimum_clearance_m,
            ),
            false,
        ),
        Err(_) => (MarginV2::failed(), false),
    }
}

fn clearance_for_state_fast(
    terrain: &ValidatedHeightfieldV2<'_>,
    state: KinematicStateV2,
    direction: Option<Vec2>,
    support_pad: Option<&PadInputV2>,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> (MarginV2, bool) {
    if let Some(pad) = pad_corridor_for_state(state, direction, support_pad, vehicle) {
        return (pad_clearance_height_margin(state, pad, vehicle), true);
    }
    // Keep compact and materialized free-flight terrain evidence identical;
    // only the query implementation differs after the contact split above.
    let envelope = rotated_hull_envelope(direction, vehicle, policy.minimum_clearance_m);
    match terrain.exact_point_clearance(state.position_m, envelope) {
        Some(clearance_m) => (
            MarginV2::lower_scaled(clearance_m, 0.0, policy.minimum_clearance_m),
            false,
        ),
        None => (MarginV2::failed(), false),
    }
}

fn first_powered_direction(bridge: &AnalyticalBridgeV2) -> Option<(u64, Vec2)> {
    bridge.samples.iter().find_map(|sample| {
        sample
            .thrust_direction_unit
            .map(|direction| (sample.tick, direction))
    })
}

fn final_powered_direction(bridge: &AnalyticalBridgeV2) -> Option<(u64, Vec2)> {
    bridge.samples.iter().rev().find_map(|sample| {
        sample
            .thrust_direction_unit
            .map(|direction| (sample.tick, direction))
    })
}

fn terminal_final_rotation_rate(bridge: &AnalyticalBridgeV2) -> Option<f64> {
    let mut powered = bridge.samples.iter().rev().filter_map(|sample| {
        sample
            .thrust_direction_unit
            .map(|direction| (sample.tick, direction))
    });
    let last = powered.next()?;
    let previous = powered.next();
    Some(previous.map_or(0.0, |previous| {
        angle_between(previous.1, last.1)
            / ((last.0 - previous.0) as f64 * bridge.duration_s / bridge.steps as f64)
    }))
}

fn screen_bridge_environment(
    terrain: &TerrainDefinition,
    bridge: &AnalyticalBridgeV2,
    support_pad: Option<&PadInputV2>,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> BridgeEnvironmentEvidenceV2 {
    let mut clearance_margin = MarginV2::unbounded();
    let mut pad_corridor_tick_count = 0;
    let mut free_flight_tick_count = 0;
    for sample in &bridge.samples {
        let (margin, in_pad_corridor) = clearance_for_state(
            terrain,
            sample.state_m,
            sample.thrust_direction_unit,
            support_pad,
            policy,
            vehicle,
        );
        clearance_margin = min_margin(clearance_margin, margin);
        if in_pad_corridor {
            pad_corridor_tick_count += 1;
        } else {
            free_flight_tick_count += 1;
        }
    }
    let first = first_powered_direction(bridge);
    let final_direction = final_powered_direction(bridge);
    let up = Vec2::new(0.0, 1.0);
    let initial_attitude_margin = if bridge.kind == BridgeKindV2::Source {
        first.map_or(MarginV2::failed(), |(_, direction)| {
            MarginV2::upper(
                vehicle.safe_touchdown_attitude_error_rad,
                angle_between(up, direction),
            )
        })
    } else {
        MarginV2::unbounded()
    };
    let final_attitude_margin = if bridge.kind == BridgeKindV2::Terminal {
        final_direction.map_or(MarginV2::failed(), |(_, direction)| {
            MarginV2::upper(
                vehicle.safe_touchdown_attitude_error_rad,
                angle_between(up, direction),
            )
        })
    } else {
        MarginV2::unbounded()
    };
    let final_angular_rate_margin = if bridge.kind == BridgeKindV2::Terminal {
        terminal_final_rotation_rate(bridge).map_or(MarginV2::failed(), |rate| {
            MarginV2::upper(vehicle.safe_touchdown_angular_rate_radps, rate)
        })
    } else {
        MarginV2::unbounded()
    };
    BridgeEnvironmentEvidenceV2 {
        clearance_margin,
        pad_corridor_tick_count,
        free_flight_tick_count,
        first_powered_direction_unit: first.map(|(_, direction)| direction),
        final_powered_direction_unit: final_direction.map(|(_, direction)| direction),
        initial_attitude_margin,
        final_attitude_margin,
        final_angular_rate_margin,
    }
}

struct MarginRangeMinimumV2 {
    logs: Vec<usize>,
    levels: Vec<Vec<MarginV2>>,
}

impl MarginRangeMinimumV2 {
    fn new(values: Vec<MarginV2>) -> Self {
        assert!(!values.is_empty(), "range-minimum input is non-empty");
        let mut logs = vec![0; values.len() + 1];
        for index in 2..logs.len() {
            logs[index] = logs[index / 2] + 1;
        }
        let mut levels = vec![values];
        let mut width = 2;
        while width <= levels[0].len() {
            let half = width / 2;
            let previous = levels.last().expect("initial range-minimum level");
            let next = (0..=previous.len() - half - 1)
                .map(|index| min_margin(previous[index], previous[index + half]))
                .collect();
            levels.push(next);
            width *= 2;
        }
        Self { logs, levels }
    }

    fn query(&self, start: usize, end_inclusive: usize) -> MarginV2 {
        assert!(start <= end_inclusive && end_inclusive < self.levels[0].len());
        let length = end_inclusive - start + 1;
        let level = self.logs[length];
        let width = 1usize << level;
        min_margin(
            self.levels[level][start],
            self.levels[level][end_inclusive + 1 - width],
        )
    }
}

fn coast_clearance_range_minimum(
    terrain: &ValidatedHeightfieldV2<'_>,
    arc: &VirtualBallisticArcV2,
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> MarginRangeMinimumV2 {
    MarginRangeMinimumV2::new(
        (0..=arc.steps)
            .map(|step| {
                let (margin, _) = clearance_for_state_fast(
                    terrain,
                    arc.state_at(step),
                    None,
                    None,
                    policy,
                    vehicle,
                );
                margin
            })
            .collect(),
    )
}

fn coast_evidence(
    arc: &VirtualBallisticArcV2,
    source: &BridgeOptionV2,
    terminal: &BridgeOptionV2,
    clearance_range_minimum: &MarginRangeMinimumV2,
    vehicle: &VehicleInputV2,
) -> CoastEvidenceV2 {
    assert!(source.handoff.arc_step < terminal.handoff.arc_step);
    let source_direction = source
        .bridge
        .environment
        .final_powered_direction_unit
        .expect("primitive-certified source bridge has thrust direction");
    let terminal_direction = terminal
        .bridge
        .environment
        .first_powered_direction_unit
        .expect("primitive-certified terminal bridge has thrust direction");
    let source_step = source.handoff.arc_step as usize;
    let terminal_step = terminal.handoff.arc_step as usize;
    let clearance_margin = clearance_range_minimum.query(source_step, terminal_step);
    let duration_s = (terminal.handoff.arc_step - source.handoff.arc_step) as f64 * arc.dt_s;
    let slew_angle_rad = angle_between(source_direction, terminal_direction);
    let required_slew_time_s = slew_angle_rad / vehicle.max_rotation_rate_radps;
    CoastEvidenceV2 {
        source_handoff: source.handoff,
        terminal_handoff: terminal.handoff,
        duration_s,
        clearance_margin,
        slew_angle_rad,
        required_slew_time_s,
        coast_slew_margin: MarginV2::upper(duration_s, required_slew_time_s),
    }
}

fn append_failed_margin_reasons(
    margins: ComponentMarginsV2,
    policy: &DirectBridgePolicyV2,
    reasons: &mut std::collections::BTreeSet<DirectBridgeReasonV2>,
) {
    for (margin, reason) in [
        (margins.coupled_thrust, DirectBridgeReasonV2::CoupledThrust),
        (
            margins.minimum_throttle,
            DirectBridgeReasonV2::MinimumThrottle,
        ),
        (margins.clearance, DirectBridgeReasonV2::TerrainClearance),
        (
            margins.bridge_endpoint,
            DirectBridgeReasonV2::BridgeEndpoint,
        ),
        (margins.powered_slew, DirectBridgeReasonV2::PoweredSlew),
        (
            margins.source_attitude,
            DirectBridgeReasonV2::SourceAttitude,
        ),
        (margins.coast_slew, DirectBridgeReasonV2::CoastSlew),
        (margins.fuel, DirectBridgeReasonV2::Fuel),
        (margins.time, DirectBridgeReasonV2::MissionTime),
        (
            margins.touchdown_speed,
            DirectBridgeReasonV2::TouchdownSpeed,
        ),
        (
            margins.touchdown_attitude,
            DirectBridgeReasonV2::TouchdownAttitude,
        ),
        (
            margins.touchdown_angular_rate,
            DirectBridgeReasonV2::TouchdownAngularRate,
        ),
    ] {
        if !margin.passes(policy) {
            reasons.insert(reason);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pair_selection(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    arc: &VirtualBallisticArcV2,
    source_option_index: usize,
    terminal_option_index: usize,
    source: &BridgeOptionV2,
    terminal: &BridgeOptionV2,
    clearance_range_minimum: &MarginRangeMinimumV2,
) -> PairSelectionV2 {
    let coast = coast_evidence(arc, source, terminal, clearance_range_minimum, vehicle);
    let mut margins = source.bridge.margins.min_with(terminal.bridge.margins);
    margins.clearance = min_margin(
        min_margin(
            source.bridge.environment.clearance_margin,
            terminal.bridge.environment.clearance_margin,
        ),
        coast.clearance_margin,
    );
    margins.source_attitude = source.bridge.environment.initial_attitude_margin;
    margins.coast_slew = coast.coast_slew_margin;
    let total_fuel_burn_kg = source.bridge.fuel_burn_kg + terminal.bridge.fuel_burn_kg;
    let total_time_s = source.bridge.duration_s + coast.duration_s + terminal.bridge.duration_s;
    margins.fuel = MarginV2::upper(vehicle.initial_fuel_kg, total_fuel_burn_kg);
    margins.time = MarginV2::upper(policy.mission_budget_s(), total_time_s);
    margins.touchdown_speed = min_margin(
        MarginV2::upper(
            vehicle.safe_touchdown_tangential_speed_mps,
            terminal.bridge.end_state.velocity_mps.x.abs(),
        ),
        MarginV2::upper(
            vehicle.safe_touchdown_normal_speed_mps,
            terminal.bridge.end_state.velocity_mps.y.abs(),
        ),
    );
    margins.touchdown_attitude = terminal.bridge.environment.final_attitude_margin;
    margins.touchdown_angular_rate = terminal.bridge.environment.final_angular_rate_margin;
    let mut reasons = std::collections::BTreeSet::new();
    reasons.extend(source.bridge.reasons.iter().copied());
    reasons.extend(terminal.bridge.reasons.iter().copied());
    append_failed_margin_reasons(margins, policy, &mut reasons);
    PairSelectionV2 {
        source_option_index,
        terminal_option_index,
        handoff_tuple: (
            source.handoff.arc_step,
            terminal.handoff.arc_step,
            source.bridge.steps,
            terminal.bridge.steps,
        ),
        coast,
        classification: if reasons.is_empty() {
            CertificationV2::Certified
        } else {
            CertificationV2::NotCertified
        },
        reasons: reasons.into_iter().collect(),
        margins,
        total_fuel_burn_kg,
        total_time_s,
    }
}

/// Ranking is intentionally state-only and deterministic: a complete
/// certificate first, then the widest normalized aggregate margin, lower
/// conservative fuel, lower elapsed time, then the handoff/bridge tick tuple.
fn pair_is_better(candidate: &PairSelectionV2, incumbent: &PairSelectionV2) -> bool {
    let candidate_certified = candidate.classification == CertificationV2::Certified;
    let incumbent_certified = incumbent.classification == CertificationV2::Certified;
    if candidate_certified != incumbent_certified {
        return candidate_certified;
    }
    match candidate
        .margins
        .minimum_normalized()
        .total_cmp(&incumbent.margins.minimum_normalized())
    {
        std::cmp::Ordering::Greater => return true,
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate
        .total_fuel_burn_kg
        .total_cmp(&incumbent.total_fuel_burn_kg)
    {
        std::cmp::Ordering::Less => return true,
        std::cmp::Ordering::Greater => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate.total_time_s.total_cmp(&incumbent.total_time_s) {
        std::cmp::Ordering::Less => return true,
        std::cmp::Ordering::Greater => return false,
        std::cmp::Ordering::Equal => {}
    }
    candidate.handoff_tuple < incumbent.handoff_tuple
}

fn candidate_identity(candidate: &DirectBridgeCandidateV2) -> String {
    digest(&CandidateIdentity {
        duration_multiplier: candidate.duration_multiplier,
        virtual_arc: &candidate.virtual_arc,
        source_handoff: candidate.source_handoff,
        terminal_handoff: candidate.terminal_handoff,
        source_bridge: candidate.source_bridge.as_ref(),
        terminal_bridge: candidate.terminal_bridge.as_ref(),
        source_environment: candidate.source_environment.as_ref(),
        terminal_environment: candidate.terminal_environment.as_ref(),
        selected_coast: candidate.selected_coast.as_ref(),
        source_bridge_attempt_count: candidate.source_bridge_attempt_count,
        terminal_bridge_attempt_count: candidate.terminal_bridge_attempt_count,
        source_bridge_streamed_tick_count: candidate.source_bridge_streamed_tick_count,
        terminal_bridge_streamed_tick_count: candidate.terminal_bridge_streamed_tick_count,
        source_environment_rejection_count: candidate.source_environment_rejection_count,
        terminal_environment_rejection_count: candidate.terminal_environment_rejection_count,
        classification: candidate.classification,
        reasons: &candidate.reasons,
        margins: candidate.margins,
        total_fuel_burn_kg: candidate.total_fuel_burn_kg,
        total_time_s: candidate.total_time_s,
    })
}

#[derive(Serialize)]
struct CandidateIdentity<'a> {
    duration_multiplier: f64,
    virtual_arc: &'a VirtualBallisticArcV2,
    source_handoff: Option<HandoffV2>,
    terminal_handoff: Option<HandoffV2>,
    source_bridge: Option<&'a AnalyticalBridgeV2>,
    terminal_bridge: Option<&'a AnalyticalBridgeV2>,
    source_environment: Option<&'a BridgeEnvironmentEvidenceV2>,
    terminal_environment: Option<&'a BridgeEnvironmentEvidenceV2>,
    selected_coast: Option<&'a CoastEvidenceV2>,
    source_bridge_attempt_count: usize,
    terminal_bridge_attempt_count: usize,
    source_bridge_streamed_tick_count: usize,
    terminal_bridge_streamed_tick_count: usize,
    source_environment_rejection_count: usize,
    terminal_environment_rejection_count: usize,
    classification: CertificationV2,
    reasons: &'a [DirectBridgeReasonV2],
    margins: ComponentMarginsV2,
    total_fuel_burn_kg: Option<f64>,
    total_time_s: Option<f64>,
}

fn failed_component_margins() -> ComponentMarginsV2 {
    ComponentMarginsV2 {
        coupled_thrust: MarginV2::failed(),
        minimum_throttle: MarginV2::failed(),
        clearance: MarginV2::failed(),
        bridge_endpoint: MarginV2::failed(),
        powered_slew: MarginV2::failed(),
        source_attitude: MarginV2::failed(),
        coast_slew: MarginV2::failed(),
        fuel: MarginV2::failed(),
        time: MarginV2::failed(),
        touchdown_speed: MarginV2::failed(),
        touchdown_attitude: MarginV2::failed(),
        touchdown_angular_rate: MarginV2::failed(),
    }
}

#[allow(clippy::too_many_arguments)]
fn bridge_options_for_handoffs(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    terrain: &ValidatedHeightfieldV2<'_>,
    arc: &VirtualBallisticArcV2,
    handoff_steps: Vec<u64>,
    kind: BridgeKindV2,
    fixed_state: KinematicStateV2,
    support_pad: Option<&PadInputV2>,
    duration_steps: &[u64],
) -> (Vec<BridgeOptionV2>, AttemptSummaryV2) {
    let mut options = Vec::new();
    let mut attempts = AttemptSummaryV2::default();
    for arc_step in handoff_steps {
        let state = arc.state_at(arc_step);
        let (start_state, end_state) = match kind {
            BridgeKindV2::Source => (fixed_state, state),
            BridgeKindV2::Intermediate => (fixed_state, state),
            BridgeKindV2::Terminal => (state, fixed_state),
        };
        let bridge = find_shortest_primitive_bridge(
            policy,
            vehicle,
            terrain,
            kind,
            start_state,
            end_state,
            support_pad,
            duration_steps,
            &mut attempts,
        );
        if let Some(bridge) = bridge {
            options.push(BridgeOptionV2 {
                handoff: HandoffV2 { arc_step, state },
                bridge,
            });
        }
    }
    (options, attempts)
}

fn evaluate_direct_candidate(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    duration_multiplier: f64,
    arc_steps: u64,
) -> DirectBridgeCandidateV2 {
    let duration_steps = bridge_duration_steps(policy);
    evaluate_direct_candidate_with_duration_steps(
        policy,
        vehicle,
        case,
        duration_multiplier,
        arc_steps,
        &duration_steps,
    )
}

fn evaluate_direct_candidate_with_duration_steps(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    duration_multiplier: f64,
    arc_steps: u64,
    duration_steps: &[u64],
) -> DirectBridgeCandidateV2 {
    let release = release_reference(case, policy, vehicle);
    let touchdown = touchdown_reference(case, vehicle);
    let terminal_state = KinematicStateV2 {
        position_m: touchdown,
        velocity_mps: Vec2::new(
            0.0,
            -(policy.terminal_target_downward_speed_fraction
                * vehicle.safe_touchdown_normal_speed_mps),
        ),
    };
    let arc = VirtualBallisticArcV2::new(policy, release, touchdown, arc_steps);
    let terrain = terrain(case);
    let validated_terrain =
        ValidatedHeightfieldV2::new(&terrain).expect("fixture terrain was validated at load time");
    let (source_options, source_attempts) = bridge_options_for_handoffs(
        policy,
        vehicle,
        &validated_terrain,
        &arc,
        source_handoff_steps(&arc, policy.handoff_interval_ticks()),
        BridgeKindV2::Source,
        KinematicStateV2 {
            position_m: case.initial_position_m,
            velocity_mps: case.initial_velocity_mps,
        },
        Some(&case.source),
        duration_steps,
    );
    let (terminal_options, terminal_attempts) = bridge_options_for_handoffs(
        policy,
        vehicle,
        &validated_terrain,
        &arc,
        terminal_handoff_steps(&arc, policy.handoff_interval_ticks()),
        BridgeKindV2::Terminal,
        terminal_state,
        Some(&case.target),
        duration_steps,
    );
    let clearance_range_minimum =
        coast_clearance_range_minimum(&validated_terrain, &arc, policy, vehicle);
    let mut best_pair: Option<PairSelectionV2> = None;
    let mut saw_nonoverlapping_pair = false;
    for (source_option_index, source) in source_options.iter().enumerate() {
        for (terminal_option_index, terminal) in terminal_options.iter().enumerate() {
            if source.handoff.arc_step >= terminal.handoff.arc_step {
                continue;
            }
            saw_nonoverlapping_pair = true;
            let pair = pair_selection(
                policy,
                vehicle,
                &arc,
                source_option_index,
                terminal_option_index,
                source,
                terminal,
                &clearance_range_minimum,
            );
            if best_pair
                .as_ref()
                .is_none_or(|current| pair_is_better(&pair, current))
            {
                best_pair = Some(pair);
            }
        }
    }

    let mut candidate = if let Some(selection) = best_pair {
        let source = &source_options[selection.source_option_index];
        let terminal = &terminal_options[selection.terminal_option_index];
        let source_bridge = exact_discrete_bridge_v2(
            policy,
            vehicle,
            source.bridge.kind,
            source.bridge.start_state,
            source.bridge.end_state,
            source.bridge.steps,
        )
        .expect("validated selected source bridge inputs");
        let terminal_bridge = exact_discrete_bridge_v2(
            policy,
            vehicle,
            terminal.bridge.kind,
            terminal.bridge.start_state,
            terminal.bridge.end_state,
            terminal.bridge.steps,
        )
        .expect("validated selected terminal bridge inputs");
        let source_environment = screen_bridge_environment(
            &terrain,
            &source_bridge,
            Some(&case.source),
            policy,
            vehicle,
        );
        let terminal_environment = screen_bridge_environment(
            &terrain,
            &terminal_bridge,
            Some(&case.target),
            policy,
            vehicle,
        );
        assert_compact_bridge_matches(&source.bridge, &source_bridge, &source_environment);
        assert_compact_bridge_matches(&terminal.bridge, &terminal_bridge, &terminal_environment);
        DirectBridgeCandidateV2 {
            duration_multiplier,
            virtual_arc: arc,
            source_handoff: Some(source.handoff),
            terminal_handoff: Some(terminal.handoff),
            source_bridge: Some(source_bridge),
            terminal_bridge: Some(terminal_bridge),
            source_environment: Some(source_environment),
            terminal_environment: Some(terminal_environment),
            selected_coast: Some(selection.coast),
            source_bridge_attempt_count: source_attempts.count,
            terminal_bridge_attempt_count: terminal_attempts.count,
            source_bridge_streamed_tick_count: source_attempts.streamed_tick_count,
            terminal_bridge_streamed_tick_count: terminal_attempts.streamed_tick_count,
            source_environment_rejection_count: source_attempts.environment_rejection_count,
            terminal_environment_rejection_count: terminal_attempts.environment_rejection_count,
            classification: selection.classification,
            reasons: selection.reasons,
            margins: selection.margins,
            total_fuel_burn_kg: Some(selection.total_fuel_burn_kg),
            total_time_s: Some(selection.total_time_s),
            identity: String::new(),
        }
    } else {
        let mut reasons = source_attempts.reasons;
        if source_options.is_empty() {
            reasons.insert(DirectBridgeReasonV2::NoSourceBridge);
        }
        reasons.extend(terminal_attempts.reasons);
        if terminal_options.is_empty() {
            reasons.insert(DirectBridgeReasonV2::NoTerminalBridge);
        }
        if !source_options.is_empty() && !terminal_options.is_empty() && !saw_nonoverlapping_pair {
            reasons.insert(DirectBridgeReasonV2::NoNonoverlappingPair);
        }
        DirectBridgeCandidateV2 {
            duration_multiplier,
            virtual_arc: arc,
            source_handoff: None,
            terminal_handoff: None,
            source_bridge: None,
            terminal_bridge: None,
            source_environment: None,
            terminal_environment: None,
            selected_coast: None,
            source_bridge_attempt_count: source_attempts.count,
            terminal_bridge_attempt_count: terminal_attempts.count,
            source_bridge_streamed_tick_count: source_attempts.streamed_tick_count,
            terminal_bridge_streamed_tick_count: terminal_attempts.streamed_tick_count,
            source_environment_rejection_count: source_attempts.environment_rejection_count,
            terminal_environment_rejection_count: terminal_attempts.environment_rejection_count,
            classification: CertificationV2::NotCertified,
            reasons: reasons.into_iter().collect(),
            margins: failed_component_margins(),
            total_fuel_burn_kg: None,
            total_time_s: None,
            identity: String::new(),
        }
    };
    candidate.identity = candidate_identity(&candidate);
    candidate
}

fn candidate_handoff_tuple(candidate: &DirectBridgeCandidateV2) -> (u64, u64, u64, u64) {
    (
        candidate
            .source_handoff
            .map_or(u64::MAX, |handoff| handoff.arc_step),
        candidate
            .terminal_handoff
            .map_or(u64::MAX, |handoff| handoff.arc_step),
        candidate
            .source_bridge
            .as_ref()
            .map_or(u64::MAX, |bridge| bridge.steps),
        candidate
            .terminal_bridge
            .as_ref()
            .map_or(u64::MAX, |bridge| bridge.steps),
    )
}

fn candidate_is_better(
    candidate: &DirectBridgeCandidateV2,
    incumbent: &DirectBridgeCandidateV2,
) -> bool {
    let candidate_certified = candidate.classification == CertificationV2::Certified;
    let incumbent_certified = incumbent.classification == CertificationV2::Certified;
    if candidate_certified != incumbent_certified {
        return candidate_certified;
    }
    match candidate
        .margins
        .minimum_normalized()
        .total_cmp(&incumbent.margins.minimum_normalized())
    {
        std::cmp::Ordering::Greater => return true,
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate
        .total_fuel_burn_kg
        .unwrap_or(f64::INFINITY)
        .total_cmp(&incumbent.total_fuel_burn_kg.unwrap_or(f64::INFINITY))
    {
        std::cmp::Ordering::Less => return true,
        std::cmp::Ordering::Greater => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate
        .total_time_s
        .unwrap_or(f64::INFINITY)
        .total_cmp(&incumbent.total_time_s.unwrap_or(f64::INFINITY))
    {
        std::cmp::Ordering::Less => return true,
        std::cmp::Ordering::Greater => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate_handoff_tuple(candidate).cmp(&candidate_handoff_tuple(incumbent)) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => candidate.identity < incumbent.identity,
    }
}

fn result_identity(result: &DirectBridgeProbeResultV2) -> String {
    digest(&ProbeResultIdentity {
        id: &result.id,
        release_reference_m: result.release_reference_m,
        touchdown_reference_m: result.touchdown_reference_m,
        candidates: &result.candidates,
        certified_candidate_count: result.certified_candidate_count,
        selected_candidate_identity: &result.selected_candidate_identity,
    })
}

#[derive(Serialize)]
struct ProbeResultIdentity<'a> {
    id: &'a str,
    release_reference_m: Vec2,
    touchdown_reference_m: Vec2,
    candidates: &'a [DirectBridgeCandidateV2],
    certified_candidate_count: usize,
    selected_candidate_identity: &'a str,
}

fn evaluate_case(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
) -> DirectBridgeProbeResultV2 {
    let release = release_reference(case, policy, vehicle);
    let touchdown = touchdown_reference(case, vehicle);
    let mut candidates: Vec<_> =
        candidate_steps(policy, case.target.center_x_m - case.source.center_x_m)
            .into_iter()
            .map(|(multiplier, steps)| {
                evaluate_direct_candidate(policy, vehicle, case, multiplier, steps)
            })
            .collect();
    candidates.sort_by(|left, right| {
        left.virtual_arc
            .steps
            .cmp(&right.virtual_arc.steps)
            .then_with(|| {
                left.duration_multiplier
                    .total_cmp(&right.duration_multiplier)
            })
    });
    let selected_candidate_identity = candidates
        .iter()
        .reduce(|best, candidate| {
            if candidate_is_better(candidate, best) {
                candidate
            } else {
                best
            }
        })
        .expect("validated fixture has duration candidates")
        .identity
        .clone();
    let certified_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .count();
    let mut result = DirectBridgeProbeResultV2 {
        id: case.id.clone(),
        release_reference_m: release,
        touchdown_reference_m: touchdown,
        candidates,
        certified_candidate_count,
        selected_candidate_identity,
        identity: String::new(),
    };
    result.identity = result_identity(&result);
    result
}

#[derive(Clone, Debug)]
struct RidgeFeatureV2 {
    base_y_m: f64,
    left_x_m: f64,
    right_x_m: f64,
    top_y_m: f64,
    identity: String,
}

#[derive(Serialize)]
struct RidgeFeatureIdentity<'a> {
    case_id: &'a str,
    base_y_m: f64,
    elevated_points_m: &'a [Vec2],
}

fn ridge_feature(case: &DirectBridgeProbeV2) -> RidgeFeatureV2 {
    let base_y_m = case.source.surface_y_m;
    let elevated_points: Vec<Vec2> = case
        .terrain_points_m
        .iter()
        .copied()
        .filter(|point| point.y > base_y_m + ENDPOINT_TOLERANCE)
        .collect();
    if elevated_points.is_empty() {
        return RidgeFeatureV2 {
            base_y_m,
            left_x_m: case.source.center_x_m,
            right_x_m: case.source.center_x_m,
            top_y_m: base_y_m,
            identity: digest(&RidgeFeatureIdentity {
                case_id: &case.id,
                base_y_m,
                elevated_points_m: &elevated_points,
            }),
        };
    }
    let left_x_m = elevated_points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let right_x_m = elevated_points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let top_y_m = elevated_points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    RidgeFeatureV2 {
        base_y_m,
        left_x_m,
        right_x_m,
        top_y_m,
        identity: digest(&RidgeFeatureIdentity {
            case_id: &case.id,
            base_y_m,
            elevated_points_m: &elevated_points,
        }),
    }
}

fn flat_twin_case(case: &DirectBridgeProbeV2) -> DirectBridgeProbeV2 {
    let mut twin = case.clone();
    twin.id = format!("{}_flat_twin", case.id);
    let domain_start = case
        .terrain_points_m
        .first()
        .expect("validated probe has terrain domain")
        .x;
    let domain_end = case
        .terrain_points_m
        .last()
        .expect("validated probe has terrain domain")
        .x;
    twin.terrain_points_m = vec![
        Vec2::new(domain_start, case.source.surface_y_m),
        Vec2::new(domain_end, case.source.surface_y_m),
    ];
    twin
}

fn flat_twin_evidence(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
) -> FlatTwinEvidenceV2 {
    let twin = flat_twin_case(case);
    let duration_steps = canary_bridge_duration_steps(policy);
    let mut candidates: Vec<_> =
        candidate_steps(policy, twin.target.center_x_m - twin.source.center_x_m)
            .into_iter()
            .map(|(multiplier, steps)| {
                evaluate_direct_candidate_with_duration_steps(
                    policy,
                    vehicle,
                    &twin,
                    multiplier,
                    steps,
                    &duration_steps,
                )
            })
            .collect();
    candidates.sort_by(|left, right| {
        left.virtual_arc
            .steps
            .cmp(&right.virtual_arc.steps)
            .then_with(|| {
                left.duration_multiplier
                    .total_cmp(&right.duration_multiplier)
            })
    });
    let certified_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.classification == CertificationV2::Certified)
        .count();
    let nominal = candidates
        .iter()
        .find(|candidate| candidate.classification == CertificationV2::Certified)
        .or_else(|| candidates.first())
        .expect("validated probe has at least one duration candidate");
    let candidate_identities = candidates
        .iter()
        .map(|candidate| candidate.identity.clone())
        .collect();
    let candidate_duration_multipliers = candidates
        .iter()
        .map(|candidate| candidate.duration_multiplier)
        .collect();
    let candidate_classifications = candidates
        .iter()
        .map(|candidate| candidate.classification)
        .collect();
    let mut evidence = FlatTwinEvidenceV2 {
        source_case_id: case.id.clone(),
        terrain_identity: digest(&twin.terrain_points_m),
        candidate_duration_multipliers,
        candidate_classifications,
        candidate_identities,
        certified_candidate_count,
        nominal_candidate_identity: nominal.identity.clone(),
        nominal_duration_multiplier: nominal.duration_multiplier,
        nominal_duration_s: nominal.virtual_arc.duration_s,
        nominal_candidate: nominal.clone(),
        status: if certified_candidate_count > 0 {
            MissionStatusV2::Green
        } else {
            MissionStatusV2::Red
        },
        identity: String::new(),
    };
    evidence.identity = digest(&FlatTwinIdentity {
        source_case_id: &evidence.source_case_id,
        terrain_identity: &evidence.terrain_identity,
        candidate_duration_multipliers: &evidence.candidate_duration_multipliers,
        candidate_classifications: &evidence.candidate_classifications,
        candidate_identities: &evidence.candidate_identities,
        certified_candidate_count: evidence.certified_candidate_count,
        nominal_candidate_identity: &evidence.nominal_candidate_identity,
        nominal_duration_multiplier: evidence.nominal_duration_multiplier,
        nominal_duration_s: evidence.nominal_duration_s,
        nominal_candidate: &evidence.nominal_candidate,
        status: evidence.status,
    });
    evidence
}

#[derive(Serialize)]
struct FlatTwinIdentity<'a> {
    source_case_id: &'a str,
    terrain_identity: &'a str,
    candidate_duration_multipliers: &'a [f64],
    candidate_classifications: &'a [CertificationV2],
    candidate_identities: &'a [String],
    certified_candidate_count: usize,
    nominal_candidate_identity: &'a str,
    nominal_duration_multiplier: f64,
    nominal_duration_s: f64,
    nominal_candidate: &'a DirectBridgeCandidateV2,
    status: MissionStatusV2,
}

fn closest_arc_step_by_x(arc: &VirtualBallisticArcV2, x_m: f64) -> u64 {
    (0..=arc.steps)
        .min_by(|left, right| {
            (arc.state_at(*left).position_m.x - x_m)
                .abs()
                .total_cmp(&(arc.state_at(*right).position_m.x - x_m).abs())
                .then_with(|| left.cmp(right))
        })
        .expect("ballistic arc has a sample")
}

fn time_shift_drift_bounds(
    nominal_velocity_mps: Vec2,
    gravity_mps2: f64,
    maximum_time_shift_s: f64,
) -> CorrectionEnvelopeBoundsV2 {
    let mut x_values = [0.0, nominal_velocity_mps.x * maximum_time_shift_s];
    let mut y_values = vec![
        0.0,
        nominal_velocity_mps.y * maximum_time_shift_s
            - 0.5 * gravity_mps2 * maximum_time_shift_s * maximum_time_shift_s,
    ];
    let vertical_apex_shift_s = nominal_velocity_mps.y / gravity_mps2;
    if vertical_apex_shift_s > 0.0 && vertical_apex_shift_s < maximum_time_shift_s {
        y_values.push(
            nominal_velocity_mps.y * vertical_apex_shift_s
                - 0.5 * gravity_mps2 * vertical_apex_shift_s * vertical_apex_shift_s,
        );
    }
    x_values.sort_by(f64::total_cmp);
    y_values.sort_by(f64::total_cmp);
    CorrectionEnvelopeBoundsV2 {
        lower_m: Vec2::new(x_values[0], y_values[0]),
        upper_m: Vec2::new(
            *x_values.last().expect("time-shift x bounds"),
            *y_values.last().expect("time-shift y bounds"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn correction_envelope_sample(
    policy: &DirectBridgePolicyV2,
    nominal: &DirectBridgeCandidateV2,
    source_handoff: HandoffV2,
    arc_step: u64,
    correction_horizon_s: f64,
    crossing_time_allowance_s: f64,
    derated_max_acceleration_mps2: f64,
    vehicle_clearance: CorridorEnvelope,
) -> CorrectionEnvelopeSampleV2 {
    let nominal_state = nominal.virtual_arc.state_at(arc_step);
    let elapsed_s =
        (arc_step.saturating_sub(source_handoff.arc_step) as f64) * nominal.virtual_arc.dt_s;
    // Any bounded delay is applied to the nominal state explicitly.  This
    // closes the previous gap where extra correction time widened the thrust
    // ball but omitted the nominal forward/vertical drift during that time.
    let time_shift_allowance_s =
        crossing_time_allowance_s.min((correction_horizon_s - elapsed_s).max(0.0));
    let nominal_time_shift_drift_bounds_m = time_shift_drift_bounds(
        nominal_state.velocity_mps,
        policy.gravity_mps2,
        time_shift_allowance_s,
    );
    let correction_time_s = elapsed_s + time_shift_allowance_s;
    let thrust_displacement_bound_m =
        0.5 * derated_max_acceleration_mps2 * correction_time_s * correction_time_s;
    let horizontal_displacement_bound_m =
        thrust_displacement_bound_m + vehicle_clearance.horizontal_extent_m;
    let vertical_displacement_bound_m =
        thrust_displacement_bound_m + vehicle_clearance.vertical_extent_m;
    let bounds = CorrectionEnvelopeBoundsV2 {
        lower_m: Vec2::new(
            nominal_state.position_m.x + nominal_time_shift_drift_bounds_m.lower_m.x
                - horizontal_displacement_bound_m,
            nominal_state.position_m.y + nominal_time_shift_drift_bounds_m.lower_m.y
                - vertical_displacement_bound_m,
        ),
        upper_m: Vec2::new(
            nominal_state.position_m.x
                + nominal_time_shift_drift_bounds_m.upper_m.x
                + horizontal_displacement_bound_m,
            nominal_state.position_m.y
                + nominal_time_shift_drift_bounds_m.upper_m.y
                + vertical_displacement_bound_m,
        ),
    };
    CorrectionEnvelopeSampleV2 {
        arc_step,
        elapsed_s,
        nominal_state,
        time_shift_allowance_s,
        nominal_time_shift_drift_bounds_m,
        correction_time_s,
        thrust_displacement_bound_m,
        horizontal_displacement_bound_m,
        vertical_displacement_bound_m,
        bounds,
    }
}

fn expand_correction_bounds(
    aggregate: &mut CorrectionEnvelopeBoundsV2,
    candidate: CorrectionEnvelopeBoundsV2,
) {
    aggregate.lower_m.x = aggregate.lower_m.x.min(candidate.lower_m.x);
    aggregate.lower_m.y = aggregate.lower_m.y.min(candidate.lower_m.y);
    aggregate.upper_m.x = aggregate.upper_m.x.max(candidate.upper_m.x);
    aggregate.upper_m.y = aggregate.upper_m.y.max(candidate.upper_m.y);
}

fn correction_envelope(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    feature: &RidgeFeatureV2,
    nominal: &DirectBridgeCandidateV2,
) -> LocalCorrectionEnvelopeV2 {
    let source_handoff = nominal.source_handoff.unwrap_or(HandoffV2 {
        arc_step: 0,
        state: nominal.virtual_arc.state_at(0),
    });
    let nominal_crossing_arc_step = closest_arc_step_by_x(
        &nominal.virtual_arc,
        (feature.left_x_m + feature.right_x_m) * 0.5,
    );
    let nominal_crossing_state = nominal.virtual_arc.state_at(nominal_crossing_arc_step);
    let crossing_time_s = nominal_crossing_arc_step.saturating_sub(source_handoff.arc_step) as f64
        * nominal.virtual_arc.dt_s;
    // This allowance is derived solely from the existing analytical sampling
    // intervals plus one physics tick. It is deliberately generous for a
    // local lane, but is not copied from any controller horizon or constant.
    let crossing_time_allowance_s =
        policy.handoff_interval_s + policy.bridge_duration_interval_s + nominal.virtual_arc.dt_s;
    let source_fuel = nominal
        .source_bridge
        .as_ref()
        .map_or(0.0, |bridge| bridge.fuel_burn_kg);
    let terminal_reserve_fuel_kg = nominal
        .terminal_bridge
        .as_ref()
        .map_or(0.0, |bridge| bridge.fuel_burn_kg);
    let mission_reserve_fuel_kg = vehicle.max_fuel_burn_kgps * policy.mission_time_reserve_s;
    let available_correction_fuel_kg = (vehicle.initial_fuel_kg
        - source_fuel
        - terminal_reserve_fuel_kg
        - mission_reserve_fuel_kg)
        .max(0.0);
    let fuel_horizon_s = available_correction_fuel_kg / vehicle.max_fuel_burn_kgps;
    let mission_horizon_s = (policy.mission_budget_s()
        - nominal
            .source_bridge
            .as_ref()
            .map_or(0.0, |bridge| bridge.duration_s)
        - nominal
            .terminal_bridge
            .as_ref()
            .map_or(0.0, |bridge| bridge.duration_s))
    .max(0.0);
    let correction_horizon_s = (crossing_time_s + crossing_time_allowance_s)
        .min(fuel_horizon_s)
        .min(mission_horizon_s);
    let derated_max_acceleration_mps2 = vehicle.derated_max_acceleration_mps2(policy);
    let vehicle_clearance = rotated_hull_envelope(None, vehicle, policy.minimum_clearance_m);
    let sample_count = 9_usize;
    let mut samples = Vec::with_capacity(sample_count);
    let mut bounds = CorrectionEnvelopeBoundsV2 {
        lower_m: Vec2::new(f64::INFINITY, f64::INFINITY),
        upper_m: Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };
    for index in 0..sample_count {
        let fraction = index as f64 / (sample_count - 1) as f64;
        let arc_step = source_handoff.arc_step
            + ((nominal_crossing_arc_step.saturating_sub(source_handoff.arc_step)) as f64
                * fraction)
                .round() as u64;
        let sample = correction_envelope_sample(
            policy,
            nominal,
            source_handoff,
            arc_step,
            correction_horizon_s,
            crossing_time_allowance_s,
            derated_max_acceleration_mps2,
            vehicle_clearance,
        );
        expand_correction_bounds(&mut bounds, sample.bounds);
        samples.push(sample);
    }
    let horizon_steps = (correction_horizon_s / nominal.virtual_arc.dt_s)
        .floor()
        .max(0.0) as u64;
    let final_arc_step = source_handoff
        .arc_step
        .saturating_add(horizon_steps)
        .min(nominal.virtual_arc.steps);
    let mut crossing_cut_bounds = CorrectionEnvelopeBoundsV2 {
        lower_m: Vec2::new(f64::INFINITY, f64::INFINITY),
        upper_m: Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };
    let mut crossing_cut_steps = Vec::new();
    let feature_center_x_m = (feature.left_x_m + feature.right_x_m) * 0.5;
    for arc_step in source_handoff.arc_step..=final_arc_step {
        let sample = correction_envelope_sample(
            policy,
            nominal,
            source_handoff,
            arc_step,
            correction_horizon_s,
            crossing_time_allowance_s,
            derated_max_acceleration_mps2,
            vehicle_clearance,
        );
        if sample.bounds.lower_m.x <= feature_center_x_m
            && sample.bounds.upper_m.x >= feature_center_x_m
        {
            expand_correction_bounds(&mut crossing_cut_bounds, sample.bounds);
            crossing_cut_steps.push(arc_step);
        }
    }
    let crossing_cut = CorrectionCrossingCutV2 {
        feature_center_x_m,
        eligible_sample_count: crossing_cut_steps.len(),
        first_arc_step: crossing_cut_steps.first().copied(),
        last_arc_step: crossing_cut_steps.last().copied(),
        bounds: crossing_cut_bounds,
    };
    let horizontal_displacement_bound_m = samples
        .last()
        .map_or(vehicle_clearance.horizontal_extent_m, |sample| {
            sample.horizontal_displacement_bound_m
        });
    let vertical_displacement_bound_m = samples
        .last()
        .map_or(vehicle_clearance.vertical_extent_m, |sample| {
            sample.vertical_displacement_bound_m
        });
    let mut envelope = LocalCorrectionEnvelopeV2 {
        nominal_candidate_identity: nominal.identity.clone(),
        commitment_phase: CorrectionCommitmentPhaseV2::PostSourceHandoff,
        source_handoff,
        commitment_state: source_handoff.state,
        feature_center_x_m: (feature.left_x_m + feature.right_x_m) * 0.5,
        nominal_crossing_arc_step,
        nominal_crossing_state,
        crossing_time_s,
        crossing_time_allowance_s,
        correction_horizon_s,
        mission_horizon_s,
        fuel_horizon_s,
        available_correction_fuel_kg,
        terminal_reserve_fuel_kg,
        mission_reserve_fuel_kg,
        worst_case_mass_kg: vehicle.worst_case_mass_kg(),
        derated_max_acceleration_mps2,
        vehicle_clearance_extent_m: Vec2::new(
            vehicle_clearance.horizontal_extent_m,
            vehicle_clearance.vertical_extent_m,
        ),
        horizontal_displacement_bound_m,
        vertical_displacement_bound_m,
        bounds,
        samples,
        crossing_cut,
        identity: String::new(),
    };
    envelope.identity = digest(&CorrectionEnvelopeIdentity {
        nominal_candidate_identity: &envelope.nominal_candidate_identity,
        commitment_phase: envelope.commitment_phase,
        source_handoff: envelope.source_handoff,
        commitment_state: envelope.commitment_state,
        feature_center_x_m: envelope.feature_center_x_m,
        nominal_crossing_arc_step: envelope.nominal_crossing_arc_step,
        nominal_crossing_state: envelope.nominal_crossing_state,
        crossing_time_s: envelope.crossing_time_s,
        crossing_time_allowance_s: envelope.crossing_time_allowance_s,
        correction_horizon_s: envelope.correction_horizon_s,
        mission_horizon_s: envelope.mission_horizon_s,
        fuel_horizon_s: envelope.fuel_horizon_s,
        available_correction_fuel_kg: envelope.available_correction_fuel_kg,
        terminal_reserve_fuel_kg: envelope.terminal_reserve_fuel_kg,
        mission_reserve_fuel_kg: envelope.mission_reserve_fuel_kg,
        worst_case_mass_kg: envelope.worst_case_mass_kg,
        derated_max_acceleration_mps2: envelope.derated_max_acceleration_mps2,
        vehicle_clearance_extent_m: envelope.vehicle_clearance_extent_m,
        horizontal_displacement_bound_m: envelope.horizontal_displacement_bound_m,
        vertical_displacement_bound_m: envelope.vertical_displacement_bound_m,
        bounds: envelope.bounds,
        samples: &envelope.samples,
        crossing_cut: &envelope.crossing_cut,
    });
    envelope
}

#[derive(Serialize)]
struct CorrectionEnvelopeIdentity<'a> {
    nominal_candidate_identity: &'a str,
    commitment_phase: CorrectionCommitmentPhaseV2,
    source_handoff: HandoffV2,
    commitment_state: KinematicStateV2,
    feature_center_x_m: f64,
    nominal_crossing_arc_step: u64,
    nominal_crossing_state: KinematicStateV2,
    crossing_time_s: f64,
    crossing_time_allowance_s: f64,
    correction_horizon_s: f64,
    mission_horizon_s: f64,
    fuel_horizon_s: f64,
    available_correction_fuel_kg: f64,
    terminal_reserve_fuel_kg: f64,
    mission_reserve_fuel_kg: f64,
    worst_case_mass_kg: f64,
    derated_max_acceleration_mps2: f64,
    vehicle_clearance_extent_m: Vec2,
    horizontal_displacement_bound_m: f64,
    vertical_displacement_bound_m: f64,
    bounds: CorrectionEnvelopeBoundsV2,
    samples: &'a [CorrectionEnvelopeSampleV2],
    crossing_cut: &'a CorrectionCrossingCutV2,
}

fn mesa_geometry(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    feature: &RidgeFeatureV2,
    envelope: &LocalCorrectionEnvelopeV2,
) -> Result<MesaGeometryV2, ExperimentalRidgeCandidateErrorV2> {
    let domain_start = case
        .terrain_points_m
        .first()
        .ok_or(ExperimentalRidgeCandidateErrorV2::DerivedMesaGeometryInvalid)?
        .x;
    let domain_end = case
        .terrain_points_m
        .last()
        .ok_or(ExperimentalRidgeCandidateErrorV2::DerivedMesaGeometryInvalid)?
        .x;
    let elevated = feature.top_y_m > feature.base_y_m + ENDPOINT_TOLERANCE;
    if !elevated {
        let terrain_points_m = vec![
            Vec2::new(domain_start, feature.base_y_m),
            Vec2::new(domain_end, feature.base_y_m),
        ];
        let mut flat = MesaGeometryV2 {
            source_feature_identity: feature.identity.clone(),
            base_y_m: feature.base_y_m,
            feature_left_x_m: feature.left_x_m,
            feature_right_x_m: feature.right_x_m,
            feature_top_y_m: feature.top_y_m,
            center_x_m: envelope.feature_center_x_m,
            base_left_x_m: envelope.feature_center_x_m,
            top_left_x_m: envelope.feature_center_x_m,
            top_right_x_m: envelope.feature_center_x_m,
            base_right_x_m: envelope.feature_center_x_m,
            top_y_m: feature.base_y_m,
            terrain_points_m,
            correction_blocking_margin_m: MarginV2::failed(),
            blocks_full_correction_corridor: false,
            identity: String::new(),
        };
        flat.identity = digest(&MesaGeometryIdentity {
            source_feature_identity: &flat.source_feature_identity,
            base_y_m: flat.base_y_m,
            feature_left_x_m: flat.feature_left_x_m,
            feature_right_x_m: flat.feature_right_x_m,
            feature_top_y_m: flat.feature_top_y_m,
            center_x_m: flat.center_x_m,
            base_left_x_m: flat.base_left_x_m,
            top_left_x_m: flat.top_left_x_m,
            top_right_x_m: flat.top_right_x_m,
            base_right_x_m: flat.base_right_x_m,
            top_y_m: flat.top_y_m,
            terrain_points_m: &flat.terrain_points_m,
            correction_blocking_margin_m: flat.correction_blocking_margin_m,
            blocks_full_correction_corridor: flat.blocks_full_correction_corridor,
        });
        return Ok(flat);
    }
    let crossing_bounds = envelope.crossing_cut.bounds;
    if envelope.crossing_cut.eligible_sample_count == 0 {
        let mut empty = MesaGeometryV2 {
            source_feature_identity: feature.identity.clone(),
            base_y_m: feature.base_y_m,
            feature_left_x_m: feature.left_x_m,
            feature_right_x_m: feature.right_x_m,
            feature_top_y_m: feature.top_y_m,
            center_x_m: envelope.feature_center_x_m,
            base_left_x_m: envelope.feature_center_x_m,
            top_left_x_m: envelope.feature_center_x_m,
            top_right_x_m: envelope.feature_center_x_m,
            base_right_x_m: envelope.feature_center_x_m,
            top_y_m: feature.base_y_m,
            terrain_points_m: vec![
                Vec2::new(domain_start, feature.base_y_m),
                Vec2::new(domain_end, feature.base_y_m),
            ],
            correction_blocking_margin_m: MarginV2::failed(),
            blocks_full_correction_corridor: false,
            identity: String::new(),
        };
        empty.identity = digest(&MesaGeometryIdentity {
            source_feature_identity: &empty.source_feature_identity,
            base_y_m: empty.base_y_m,
            feature_left_x_m: empty.feature_left_x_m,
            feature_right_x_m: empty.feature_right_x_m,
            feature_top_y_m: empty.feature_top_y_m,
            center_x_m: empty.center_x_m,
            base_left_x_m: empty.base_left_x_m,
            top_left_x_m: empty.top_left_x_m,
            top_right_x_m: empty.top_right_x_m,
            base_right_x_m: empty.base_right_x_m,
            top_y_m: empty.top_y_m,
            terrain_points_m: &empty.terrain_points_m,
            correction_blocking_margin_m: empty.correction_blocking_margin_m,
            blocks_full_correction_corridor: empty.blocks_full_correction_corridor,
        });
        return Ok(empty);
    }
    let top_left_x_m = feature
        .left_x_m
        .min(crossing_bounds.lower_m.x)
        .max(domain_start + policy.minimum_clearance_m);
    let top_right_x_m = feature
        .right_x_m
        .max(crossing_bounds.upper_m.x)
        .min(domain_end - policy.minimum_clearance_m);
    let top_y_m = feature.top_y_m.max(
        crossing_bounds.upper_m.y
            + policy.minimum_clearance_m * (1.0 + policy.declared_robustness_margin),
    );
    let horizontal_run_m = rotated_hull_envelope(None, vehicle, policy.minimum_clearance_m)
        .horizontal_extent_m
        .max(policy.minimum_clearance_m);
    let base_left_x_m = (top_left_x_m - horizontal_run_m).max(domain_start);
    let base_right_x_m = (top_right_x_m + horizontal_run_m).min(domain_end);
    if ![
        domain_start,
        base_left_x_m,
        top_left_x_m,
        top_right_x_m,
        base_right_x_m,
        domain_end,
        top_y_m,
    ]
    .into_iter()
    .all(f64::is_finite)
        || domain_start > base_left_x_m
        || base_left_x_m > top_left_x_m
        || top_left_x_m > top_right_x_m
        || top_right_x_m > base_right_x_m
        || base_right_x_m > domain_end
    {
        return Err(ExperimentalRidgeCandidateErrorV2::DerivedMesaGeometryInvalid);
    }
    let terrain_points_m = vec![
        Vec2::new(domain_start, feature.base_y_m),
        Vec2::new(base_left_x_m, feature.base_y_m),
        Vec2::new(top_left_x_m, top_y_m),
        Vec2::new(top_right_x_m, top_y_m),
        Vec2::new(base_right_x_m, feature.base_y_m),
        Vec2::new(domain_end, feature.base_y_m),
    ];
    let correction_blocking_margin_m = MarginV2::lower_scaled(
        top_y_m - crossing_bounds.upper_m.y,
        policy.minimum_clearance_m,
        policy.minimum_clearance_m,
    );
    let blocks_full_correction_corridor = correction_blocking_margin_m.passes(policy)
        && top_left_x_m <= crossing_bounds.lower_m.x + ENDPOINT_TOLERANCE
        && top_right_x_m >= crossing_bounds.upper_m.x - ENDPOINT_TOLERANCE;
    let mut mesa = MesaGeometryV2 {
        source_feature_identity: feature.identity.clone(),
        base_y_m: feature.base_y_m,
        feature_left_x_m: feature.left_x_m,
        feature_right_x_m: feature.right_x_m,
        feature_top_y_m: feature.top_y_m,
        center_x_m: envelope.feature_center_x_m,
        base_left_x_m,
        top_left_x_m,
        top_right_x_m,
        base_right_x_m,
        top_y_m,
        terrain_points_m,
        correction_blocking_margin_m,
        blocks_full_correction_corridor,
        identity: String::new(),
    };
    mesa.identity = digest(&MesaGeometryIdentity {
        source_feature_identity: &mesa.source_feature_identity,
        base_y_m: mesa.base_y_m,
        feature_left_x_m: mesa.feature_left_x_m,
        feature_right_x_m: mesa.feature_right_x_m,
        feature_top_y_m: mesa.feature_top_y_m,
        center_x_m: mesa.center_x_m,
        base_left_x_m: mesa.base_left_x_m,
        top_left_x_m: mesa.top_left_x_m,
        top_right_x_m: mesa.top_right_x_m,
        base_right_x_m: mesa.base_right_x_m,
        top_y_m: mesa.top_y_m,
        terrain_points_m: &mesa.terrain_points_m,
        correction_blocking_margin_m: mesa.correction_blocking_margin_m,
        blocks_full_correction_corridor: mesa.blocks_full_correction_corridor,
    });
    Ok(mesa)
}

fn correction_envelope_is_valid(envelope: &LocalCorrectionEnvelopeV2) -> bool {
    let bounds_are_ordered = |bounds: CorrectionEnvelopeBoundsV2| {
        bounds.lower_m.x.is_finite()
            && bounds.lower_m.y.is_finite()
            && bounds.upper_m.x.is_finite()
            && bounds.upper_m.y.is_finite()
            && bounds.lower_m.x <= bounds.upper_m.x
            && bounds.lower_m.y <= bounds.upper_m.y
    };
    envelope.commitment_phase == CorrectionCommitmentPhaseV2::PostSourceHandoff
        && envelope.source_handoff.state == envelope.commitment_state
        && envelope.samples.len() >= 2
        && envelope.crossing_time_s.is_finite()
        && envelope.crossing_time_s >= 0.0
        && envelope.crossing_time_allowance_s.is_finite()
        && envelope.crossing_time_allowance_s >= 0.0
        && envelope.correction_horizon_s.is_finite()
        && envelope.correction_horizon_s >= envelope.crossing_time_s
        && envelope.worst_case_mass_kg.is_finite()
        && envelope.worst_case_mass_kg > 0.0
        && envelope.derated_max_acceleration_mps2.is_finite()
        && envelope.derated_max_acceleration_mps2 > 0.0
        && envelope.available_correction_fuel_kg.is_finite()
        && envelope.available_correction_fuel_kg >= 0.0
        && bounds_are_ordered(envelope.bounds)
        && envelope.crossing_cut.eligible_sample_count > 0
        && envelope.crossing_cut.first_arc_step.is_some()
        && envelope.crossing_cut.last_arc_step.is_some()
        && envelope
            .crossing_cut
            .first_arc_step
            .zip(envelope.crossing_cut.last_arc_step)
            .is_some_and(|(first, last)| first <= last)
        && bounds_are_ordered(envelope.crossing_cut.bounds)
        && envelope.samples.iter().all(|sample| {
            sample.elapsed_s.is_finite()
                && sample.elapsed_s >= 0.0
                && sample.time_shift_allowance_s.is_finite()
                && sample.time_shift_allowance_s >= 0.0
                && bounds_are_ordered(sample.nominal_time_shift_drift_bounds_m)
                && sample.correction_time_s.is_finite()
                && sample.correction_time_s >= 0.0
                && sample.thrust_displacement_bound_m.is_finite()
                && sample.thrust_displacement_bound_m >= 0.0
                && sample.horizontal_displacement_bound_m.is_finite()
                && sample.horizontal_displacement_bound_m >= 0.0
                && sample.vertical_displacement_bound_m.is_finite()
                && sample.vertical_displacement_bound_m >= 0.0
                && bounds_are_ordered(sample.bounds)
        })
}

#[derive(Serialize)]
struct MesaGeometryIdentity<'a> {
    source_feature_identity: &'a str,
    base_y_m: f64,
    feature_left_x_m: f64,
    feature_right_x_m: f64,
    feature_top_y_m: f64,
    center_x_m: f64,
    base_left_x_m: f64,
    top_left_x_m: f64,
    top_right_x_m: f64,
    base_right_x_m: f64,
    top_y_m: f64,
    terrain_points_m: &'a [Vec2],
    correction_blocking_margin_m: MarginV2,
    blocks_full_correction_corridor: bool,
}

const WAYPOINT_HANDOFF_OPTION_LIMIT_V2: usize = 4;
const WAYPOINT_POSITION_LIMIT_V2: usize = 8;
const WAYPOINT_CANDIDATE_LIMIT_V2: usize = 96;

fn bounded_steps(mut steps: Vec<u64>, limit: usize) -> Vec<u64> {
    steps.sort_unstable();
    steps.dedup();
    if steps.len() <= limit {
        return steps;
    }
    let mut selected = Vec::with_capacity(limit);
    for index in 0..limit {
        let source_index = index * (steps.len() - 1) / (limit - 1);
        selected.push(steps[source_index]);
    }
    selected.dedup();
    selected
}

fn canary_bridge_duration_steps(policy: &DirectBridgePolicyV2) -> Vec<u64> {
    // The existing direct certificate keeps the complete mission-budget grid.
    // The ridge witness search is a separate, explicitly bounded lane: this
    // deterministic evenly spaced 32-point subset still spans the full policy budget,
    // including long source/terminal transitions, without turning every
    // report rebuild into an unbounded-duration sweep.
    bounded_steps(bridge_duration_steps(policy), 32)
}

fn bounded_bridge_options(mut options: Vec<BridgeOptionV2>, limit: usize) -> Vec<BridgeOptionV2> {
    if options.len() <= limit {
        return options;
    }
    options.sort_by(|left, right| {
        right
            .bridge
            .margins
            .minimum_normalized()
            .total_cmp(&left.bridge.margins.minimum_normalized())
            .then_with(|| left.handoff.arc_step.cmp(&right.handoff.arc_step))
            .then_with(|| left.bridge.steps.cmp(&right.bridge.steps))
    });
    options.truncate(limit);
    options.sort_by_key(|left| left.handoff.arc_step);
    options
}

fn waypoint_positions(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    mesa: &MesaGeometryV2,
) -> Vec<Vec2> {
    if !mesa.blocks_full_correction_corridor {
        return Vec::new();
    }
    let clearance = rotated_hull_envelope(None, vehicle, policy.minimum_clearance_m);
    let side_step = clearance
        .horizontal_extent_m
        .max(policy.minimum_clearance_m);
    let height_step = clearance.vertical_extent_m.max(policy.minimum_clearance_m);
    let release_x_m = release_reference(case, policy, vehicle).x;
    let touchdown_x_m = touchdown_reference(case, vehicle).x;
    let domain_start_x_m = case
        .terrain_points_m
        .first()
        .expect("validated probe has terrain domain")
        .x;
    let domain_end_x_m = case
        .terrain_points_m
        .last()
        .expect("validated probe has terrain domain")
        .x;
    let legal_left_x_m = release_x_m
        .min(touchdown_x_m)
        .max(domain_start_x_m.min(domain_end_x_m));
    let legal_right_x_m = release_x_m
        .max(touchdown_x_m)
        .min(domain_start_x_m.max(domain_end_x_m));
    let mut positions = Vec::new();
    // Positions are generated from the derived mesa edges and the shared
    // vehicle clearance, not from a per-case coordinate or golden witness.
    for side in [1.0_f64, 2.0] {
        positions.push(Vec2::new(
            mesa.base_right_x_m + side * side_step,
            mesa.top_y_m + side * height_step,
        ));
    }
    for side in [1.0_f64, 2.0] {
        positions.push(Vec2::new(
            mesa.base_left_x_m - side * side_step,
            mesa.top_y_m + side * height_step,
        ));
    }
    positions.sort_by(|left, right| {
        left.x
            .total_cmp(&right.x)
            .then_with(|| left.y.total_cmp(&right.y))
    });
    positions.dedup_by(|left, right| {
        (left.x - right.x).abs() <= ENDPOINT_TOLERANCE
            && (left.y - right.y).abs() <= ENDPOINT_TOLERANCE
    });
    // Keep terrain-derived positions in the common legal x-span before any
    // ballistic duration is derived. This prevents a mesa edge outside the
    // source/target or terrain domain from turning candidate_steps into a
    // degenerate or reversed leg.
    positions.retain(|position| {
        position.x.is_finite()
            && position.y.is_finite()
            && position.x > legal_left_x_m + ENDPOINT_TOLERANCE
            && position.x < legal_right_x_m - ENDPOINT_TOLERANCE
    });
    positions.truncate(WAYPOINT_POSITION_LIMIT_V2);
    positions
}

#[derive(Clone)]
struct WaypointSelectionV2 {
    source_option_index: usize,
    intermediate_entry_arc_step: u64,
    intermediate_exit_arc_step: u64,
    terminal_option_index: usize,
    intermediate_bridge: CompactBridgeAssessmentV2,
    route_progress: RouteProgressEvidenceV2,
    margins: ComponentMarginsV2,
    classification: CertificationV2,
    reasons: Vec<DirectBridgeReasonV2>,
    total_fuel_burn_kg: f64,
    total_time_s: f64,
}

fn coast_slew_margin(
    duration_s: f64,
    source_direction: Option<Vec2>,
    terminal_direction: Option<Vec2>,
    vehicle: &VehicleInputV2,
) -> MarginV2 {
    let required_slew_time_s = match (source_direction, terminal_direction) {
        (Some(source), Some(terminal)) => {
            angle_between(source, terminal) / vehicle.max_rotation_rate_radps
        }
        _ => f64::MAX,
    };
    MarginV2::upper(duration_s, required_slew_time_s)
}

fn tangent_component(vector: Vec2, tangent_unit: Vec2) -> f64 {
    vector.x * tangent_unit.x + vector.y * tangent_unit.y
}

fn bridge_route_progress_segment(
    bridge: &CompactBridgeAssessmentV2,
    segment: RouteProgressSegmentV2,
    tangent_unit: Vec2,
    allowed_backtracking_distance_m: f64,
    policy: &DirectBridgePolicyV2,
) -> RouteProgressSegmentEvidenceV2 {
    let mut previous_progress_m = tangent_component(bridge.start_state.position_m, tangent_unit);
    let mut minimum_tangent_velocity_mps =
        tangent_component(bridge.start_state.velocity_mps, tangent_unit);
    let mut backtracking_distance_m = 0.0;
    for applied_steps in 1..=bridge.steps {
        let state = exact_affine_bridge_state(
            bridge.start_state,
            bridge.initial_net_acceleration_mps2,
            bridge.net_acceleration_step_mps2,
            policy.dt_s(),
            applied_steps,
        );
        let progress_m = tangent_component(state.position_m, tangent_unit);
        backtracking_distance_m += (previous_progress_m - progress_m).max(0.0);
        previous_progress_m = progress_m;
        minimum_tangent_velocity_mps =
            minimum_tangent_velocity_mps.min(tangent_component(state.velocity_mps, tangent_unit));
    }
    let backtracking_margin_m = allowed_backtracking_distance_m - backtracking_distance_m;
    RouteProgressSegmentEvidenceV2 {
        segment,
        sample_count: usize::try_from(bridge.steps)
            .expect("validated bridge step count fits usize")
            + 1,
        minimum_tangent_velocity_mps,
        backtracking_distance_m,
        allowed_backtracking_distance_m,
        backtracking_margin_m,
        passes: backtracking_margin_m + ENDPOINT_TOLERANCE >= 0.0,
    }
}

fn coast_route_progress_segment(
    arc: &VirtualBallisticArcV2,
    start_step: u64,
    end_step: u64,
    segment: RouteProgressSegmentV2,
    tangent_unit: Vec2,
) -> RouteProgressSegmentEvidenceV2 {
    let start = arc.state_at(start_step);
    let end = arc.state_at(end_step);
    let start_progress_m = tangent_component(start.position_m, tangent_unit);
    let end_progress_m = tangent_component(end.position_m, tangent_unit);
    let backtracking_distance_m = (start_progress_m - end_progress_m).max(0.0);
    let minimum_tangent_velocity_mps = tangent_component(start.velocity_mps, tangent_unit)
        .min(tangent_component(end.velocity_mps, tangent_unit));
    let allowed_backtracking_distance_m = ENDPOINT_TOLERANCE;
    let backtracking_margin_m = allowed_backtracking_distance_m - backtracking_distance_m;
    RouteProgressSegmentEvidenceV2 {
        segment,
        sample_count: usize::try_from(end_step - start_step)
            .expect("validated coast step count fits usize")
            + 1,
        minimum_tangent_velocity_mps,
        backtracking_distance_m,
        allowed_backtracking_distance_m,
        backtracking_margin_m,
        passes: backtracking_margin_m + ENDPOINT_TOLERANCE >= 0.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn waypoint_route_progress_evidence(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    source: &BridgeOptionV2,
    source_leg: &VirtualBallisticArcV2,
    source_coast_end_step: u64,
    intermediate_bridge: &CompactBridgeAssessmentV2,
    target_leg: &VirtualBallisticArcV2,
    terminal_coast_start_step: u64,
    terminal: &BridgeOptionV2,
) -> RouteProgressEvidenceV2 {
    let horizontal_sign = (case.target.center_x_m - case.source.center_x_m).signum();
    let tangent_unit = Vec2::new(horizontal_sign, 0.0);
    let endpoint_alignment_allowance_m = vehicle.geometry.touchdown_half_span_m;
    let segments = vec![
        bridge_route_progress_segment(
            &source.bridge,
            RouteProgressSegmentV2::SourceBridge,
            tangent_unit,
            endpoint_alignment_allowance_m,
            policy,
        ),
        coast_route_progress_segment(
            source_leg,
            source.handoff.arc_step,
            source_coast_end_step,
            RouteProgressSegmentV2::SourceCoast,
            tangent_unit,
        ),
        bridge_route_progress_segment(
            intermediate_bridge,
            RouteProgressSegmentV2::IntermediateBridge,
            tangent_unit,
            ENDPOINT_TOLERANCE,
            policy,
        ),
        coast_route_progress_segment(
            target_leg,
            terminal_coast_start_step,
            terminal.handoff.arc_step,
            RouteProgressSegmentV2::TerminalCoast,
            tangent_unit,
        ),
        bridge_route_progress_segment(
            &terminal.bridge,
            RouteProgressSegmentV2::TerminalBridge,
            tangent_unit,
            endpoint_alignment_allowance_m,
            policy,
        ),
    ];
    let minimum_tangent_velocity_mps = segments
        .iter()
        .map(|segment| segment.minimum_tangent_velocity_mps)
        .fold(f64::INFINITY, f64::min);
    RouteProgressEvidenceV2 {
        ground_track_tangent_unit: tangent_unit,
        minimum_tangent_velocity_mps,
        total_backtracking_distance_m: segments
            .iter()
            .map(|segment| segment.backtracking_distance_m)
            .sum(),
        passes: segments.iter().all(|segment| segment.passes),
        segments,
    }
}

fn waypoint_selection_is_better(
    candidate: &WaypointSelectionV2,
    incumbent: &WaypointSelectionV2,
) -> bool {
    let candidate_certified = candidate.classification == CertificationV2::Certified;
    let incumbent_certified = incumbent.classification == CertificationV2::Certified;
    if candidate_certified != incumbent_certified {
        return candidate_certified;
    }
    match candidate
        .margins
        .minimum_normalized()
        .total_cmp(&incumbent.margins.minimum_normalized())
    {
        std::cmp::Ordering::Greater => return true,
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate
        .total_fuel_burn_kg
        .total_cmp(&incumbent.total_fuel_burn_kg)
    {
        std::cmp::Ordering::Less => return true,
        std::cmp::Ordering::Greater => return false,
        std::cmp::Ordering::Equal => {}
    }
    match candidate.total_time_s.total_cmp(&incumbent.total_time_s) {
        std::cmp::Ordering::Less => return true,
        std::cmp::Ordering::Greater => return false,
        std::cmp::Ordering::Equal => {}
    }
    (
        candidate.intermediate_entry_arc_step,
        candidate.intermediate_exit_arc_step,
        candidate.source_option_index,
        candidate.terminal_option_index,
    ) < (
        incumbent.intermediate_entry_arc_step,
        incumbent.intermediate_exit_arc_step,
        incumbent.source_option_index,
        incumbent.terminal_option_index,
    )
}

#[allow(clippy::too_many_arguments)]
fn evaluate_waypoint_candidate(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    terrain: &TerrainDefinition,
    waypoint_position_m: Vec2,
    source_leg_multiplier: f64,
    target_leg_multiplier: f64,
) -> WaypointCandidateV2 {
    let release = release_reference(case, policy, vehicle);
    let touchdown = touchdown_reference(case, vehicle);
    let terminal_state = KinematicStateV2 {
        position_m: touchdown,
        velocity_mps: Vec2::new(
            0.0,
            -(policy.terminal_target_downward_speed_fraction
                * vehicle.safe_touchdown_normal_speed_mps),
        ),
    };
    let source_steps = candidate_steps(policy, waypoint_position_m.x - release.x)
        .into_iter()
        .find(|(multiplier, _)| (*multiplier - source_leg_multiplier).abs() <= 1.0e-12)
        .map_or(1, |(_, steps)| steps);
    let target_steps = candidate_steps(policy, touchdown.x - waypoint_position_m.x)
        .into_iter()
        .find(|(multiplier, _)| (*multiplier - target_leg_multiplier).abs() <= 1.0e-12)
        .map_or(1, |(_, steps)| steps);
    let source_leg = VirtualBallisticArcV2::new(policy, release, waypoint_position_m, source_steps);
    let target_leg =
        VirtualBallisticArcV2::new(policy, waypoint_position_m, touchdown, target_steps);
    let validated_terrain =
        ValidatedHeightfieldV2::new(terrain).expect("derived mesa terrain is validated");
    let duration_steps = canary_bridge_duration_steps(policy);
    let (source_options_all, source_attempts) = bridge_options_for_handoffs(
        policy,
        vehicle,
        &validated_terrain,
        &source_leg,
        bounded_steps(
            source_handoff_steps(&source_leg, policy.handoff_interval_ticks()),
            WAYPOINT_HANDOFF_OPTION_LIMIT_V2,
        ),
        BridgeKindV2::Source,
        KinematicStateV2 {
            position_m: case.initial_position_m,
            velocity_mps: case.initial_velocity_mps,
        },
        Some(&case.source),
        &duration_steps,
    );
    let (terminal_options_all, terminal_attempts) = bridge_options_for_handoffs(
        policy,
        vehicle,
        &validated_terrain,
        &target_leg,
        bounded_steps(
            terminal_handoff_steps(&target_leg, policy.handoff_interval_ticks()),
            WAYPOINT_HANDOFF_OPTION_LIMIT_V2,
        ),
        BridgeKindV2::Terminal,
        terminal_state,
        Some(&case.target),
        &duration_steps,
    );
    let source_options =
        bounded_bridge_options(source_options_all, WAYPOINT_HANDOFF_OPTION_LIMIT_V2);
    let terminal_options =
        bounded_bridge_options(terminal_options_all, WAYPOINT_HANDOFF_OPTION_LIMIT_V2);
    let source_entry_steps = bounded_steps(
        terminal_handoff_steps(&source_leg, policy.handoff_interval_ticks()),
        WAYPOINT_HANDOFF_OPTION_LIMIT_V2,
    );
    let target_exit_steps = bounded_steps(
        source_handoff_steps(&target_leg, policy.handoff_interval_ticks()),
        WAYPOINT_HANDOFF_OPTION_LIMIT_V2,
    );
    let source_coast_range =
        coast_clearance_range_minimum(&validated_terrain, &source_leg, policy, vehicle);
    let target_coast_range =
        coast_clearance_range_minimum(&validated_terrain, &target_leg, policy, vehicle);
    let mut intermediate_attempts = AttemptSummaryV2::default();
    let mut best_selection: Option<WaypointSelectionV2> = None;
    for (source_option_index, source) in source_options.iter().enumerate() {
        for &entry_arc_step in &source_entry_steps {
            if entry_arc_step <= source.handoff.arc_step
                || entry_arc_step < source_leg.apex_step
                || entry_arc_step >= source_leg.steps
            {
                continue;
            }
            for &exit_arc_step in &target_exit_steps {
                if exit_arc_step > target_leg.apex_step || exit_arc_step == 0 {
                    continue;
                }
                let start_state = source_leg.state_at(entry_arc_step);
                let end_state = target_leg.state_at(exit_arc_step);
                let Some(intermediate_bridge) = find_shortest_primitive_bridge(
                    policy,
                    vehicle,
                    &validated_terrain,
                    BridgeKindV2::Intermediate,
                    start_state,
                    end_state,
                    None,
                    &duration_steps,
                    &mut intermediate_attempts,
                ) else {
                    continue;
                };
                for (terminal_option_index, terminal) in terminal_options.iter().enumerate() {
                    if exit_arc_step >= terminal.handoff.arc_step {
                        continue;
                    }
                    let source_coast_duration_s =
                        (entry_arc_step - source.handoff.arc_step) as f64 * source_leg.dt_s;
                    let terminal_coast_duration_s =
                        (terminal.handoff.arc_step - exit_arc_step) as f64 * target_leg.dt_s;
                    let source_coast_clearance = source_coast_range
                        .query(source.handoff.arc_step as usize, entry_arc_step as usize);
                    let terminal_coast_clearance = target_coast_range
                        .query(exit_arc_step as usize, terminal.handoff.arc_step as usize);
                    let source_coast_slew = coast_slew_margin(
                        source_coast_duration_s,
                        source.bridge.environment.final_powered_direction_unit,
                        intermediate_bridge.environment.first_powered_direction_unit,
                        vehicle,
                    );
                    let terminal_coast_slew = coast_slew_margin(
                        terminal_coast_duration_s,
                        intermediate_bridge.environment.final_powered_direction_unit,
                        terminal.bridge.environment.first_powered_direction_unit,
                        vehicle,
                    );
                    let mut margins = source.bridge.margins.min_with(terminal.bridge.margins);
                    margins = margins.min_with(intermediate_bridge.margins);
                    margins.clearance = min_margin(
                        min_margin(
                            min_margin(
                                source.bridge.environment.clearance_margin,
                                intermediate_bridge.environment.clearance_margin,
                            ),
                            terminal.bridge.environment.clearance_margin,
                        ),
                        min_margin(source_coast_clearance, terminal_coast_clearance),
                    );
                    margins.source_attitude = source.bridge.environment.initial_attitude_margin;
                    margins.coast_slew = min_margin(source_coast_slew, terminal_coast_slew);
                    let total_fuel_burn_kg = source.bridge.fuel_burn_kg
                        + intermediate_bridge.fuel_burn_kg
                        + terminal.bridge.fuel_burn_kg;
                    let total_time_s = source.bridge.duration_s
                        + source_coast_duration_s
                        + intermediate_bridge.duration_s
                        + terminal_coast_duration_s
                        + terminal.bridge.duration_s;
                    margins.fuel = MarginV2::upper(vehicle.initial_fuel_kg, total_fuel_burn_kg);
                    margins.time = MarginV2::upper(policy.mission_budget_s(), total_time_s);
                    margins.touchdown_speed = min_margin(
                        MarginV2::upper(
                            vehicle.safe_touchdown_tangential_speed_mps,
                            terminal.bridge.end_state.velocity_mps.x.abs(),
                        ),
                        MarginV2::upper(
                            vehicle.safe_touchdown_normal_speed_mps,
                            terminal.bridge.end_state.velocity_mps.y.abs(),
                        ),
                    );
                    margins.touchdown_attitude = terminal.bridge.environment.final_attitude_margin;
                    margins.touchdown_angular_rate =
                        terminal.bridge.environment.final_angular_rate_margin;
                    let mut reasons = std::collections::BTreeSet::new();
                    reasons.extend(source.bridge.reasons.iter().copied());
                    reasons.extend(intermediate_bridge.reasons.iter().copied());
                    reasons.extend(terminal.bridge.reasons.iter().copied());
                    if !source_coast_clearance.passes(policy)
                        || !terminal_coast_clearance.passes(policy)
                    {
                        reasons.insert(DirectBridgeReasonV2::TerrainClearance);
                    }
                    if !source_coast_slew.passes(policy) || !terminal_coast_slew.passes(policy) {
                        reasons.insert(DirectBridgeReasonV2::CoastSlew);
                    }
                    append_failed_margin_reasons(margins, policy, &mut reasons);
                    let route_progress = waypoint_route_progress_evidence(
                        policy,
                        vehicle,
                        case,
                        source,
                        &source_leg,
                        entry_arc_step,
                        &intermediate_bridge,
                        &target_leg,
                        exit_arc_step,
                        terminal,
                    );
                    if !route_progress.passes {
                        reasons.insert(DirectBridgeReasonV2::RouteProgress);
                    }
                    let selection = WaypointSelectionV2 {
                        source_option_index,
                        intermediate_entry_arc_step: entry_arc_step,
                        intermediate_exit_arc_step: exit_arc_step,
                        terminal_option_index,
                        intermediate_bridge: intermediate_bridge.clone(),
                        route_progress,
                        margins,
                        classification: if reasons.is_empty() {
                            CertificationV2::Certified
                        } else {
                            CertificationV2::NotCertified
                        },
                        reasons: reasons.into_iter().collect(),
                        total_fuel_burn_kg,
                        total_time_s,
                    };
                    if best_selection
                        .as_ref()
                        .is_none_or(|incumbent| waypoint_selection_is_better(&selection, incumbent))
                    {
                        best_selection = Some(selection);
                    }
                }
            }
        }
    }

    let mut candidate = if let Some(selection) = best_selection {
        let source = &source_options[selection.source_option_index];
        let terminal = &terminal_options[selection.terminal_option_index];
        let source_bridge = exact_discrete_bridge_v2(
            policy,
            vehicle,
            source.bridge.kind,
            source.bridge.start_state,
            source.bridge.end_state,
            source.bridge.steps,
        )
        .expect("validated selected waypoint source bridge inputs");
        let intermediate_bridge = exact_discrete_bridge_v2(
            policy,
            vehicle,
            BridgeKindV2::Intermediate,
            selection.intermediate_bridge.start_state,
            selection.intermediate_bridge.end_state,
            selection.intermediate_bridge.steps,
        )
        .expect("validated selected waypoint intermediate bridge inputs");
        let terminal_bridge = exact_discrete_bridge_v2(
            policy,
            vehicle,
            terminal.bridge.kind,
            terminal.bridge.start_state,
            terminal.bridge.end_state,
            terminal.bridge.steps,
        )
        .expect("validated selected waypoint terminal bridge inputs");
        let source_environment =
            screen_bridge_environment(terrain, &source_bridge, Some(&case.source), policy, vehicle);
        let intermediate_environment =
            screen_bridge_environment(terrain, &intermediate_bridge, None, policy, vehicle);
        let terminal_environment = screen_bridge_environment(
            terrain,
            &terminal_bridge,
            Some(&case.target),
            policy,
            vehicle,
        );
        assert_compact_bridge_matches(&source.bridge, &source_bridge, &source_environment);
        assert_compact_bridge_matches(
            &selection.intermediate_bridge,
            &intermediate_bridge,
            &intermediate_environment,
        );
        assert_compact_bridge_matches(&terminal.bridge, &terminal_bridge, &terminal_environment);
        let intermediate_entry_state = source_leg.state_at(selection.intermediate_entry_arc_step);
        let intermediate_exit_state = target_leg.state_at(selection.intermediate_exit_arc_step);
        WaypointCandidateV2 {
            waypoint_position_m,
            source_leg,
            target_leg,
            source_handoff: Some(source.handoff),
            intermediate_entry_handoff: Some(HandoffV2 {
                arc_step: selection.intermediate_entry_arc_step,
                state: intermediate_entry_state,
            }),
            intermediate_exit_handoff: Some(HandoffV2 {
                arc_step: selection.intermediate_exit_arc_step,
                state: intermediate_exit_state,
            }),
            terminal_handoff: Some(terminal.handoff),
            source_bridge: Some(source_bridge),
            intermediate_bridge: Some(intermediate_bridge),
            terminal_bridge: Some(terminal_bridge),
            source_environment: Some(source_environment),
            intermediate_environment: Some(intermediate_environment),
            terminal_environment: Some(terminal_environment),
            route_progress: Some(selection.route_progress),
            source_bridge_attempt_count: source_attempts.count,
            intermediate_bridge_attempt_count: intermediate_attempts.count,
            terminal_bridge_attempt_count: terminal_attempts.count,
            source_bridge_streamed_tick_count: source_attempts.streamed_tick_count,
            intermediate_bridge_streamed_tick_count: intermediate_attempts.streamed_tick_count,
            terminal_bridge_streamed_tick_count: terminal_attempts.streamed_tick_count,
            source_environment_rejection_count: source_attempts.environment_rejection_count,
            intermediate_environment_rejection_count: intermediate_attempts
                .environment_rejection_count,
            terminal_environment_rejection_count: terminal_attempts.environment_rejection_count,
            classification: selection.classification,
            reasons: selection.reasons,
            margins: selection.margins,
            total_fuel_burn_kg: Some(selection.total_fuel_burn_kg),
            total_time_s: Some(selection.total_time_s),
            identity: String::new(),
        }
    } else {
        let mut reasons = source_attempts.reasons;
        if source_options.is_empty() {
            reasons.insert(DirectBridgeReasonV2::NoSourceBridge);
        }
        reasons.extend(intermediate_attempts.reasons);
        if source_options.is_empty() || target_exit_steps.is_empty() {
            reasons.insert(DirectBridgeReasonV2::NoIntermediateBridge);
        }
        reasons.extend(terminal_attempts.reasons);
        if terminal_options.is_empty() {
            reasons.insert(DirectBridgeReasonV2::NoTerminalBridge);
        }
        WaypointCandidateV2 {
            waypoint_position_m,
            source_leg,
            target_leg,
            source_handoff: None,
            intermediate_entry_handoff: None,
            intermediate_exit_handoff: None,
            terminal_handoff: None,
            source_bridge: None,
            intermediate_bridge: None,
            terminal_bridge: None,
            source_environment: None,
            intermediate_environment: None,
            terminal_environment: None,
            route_progress: None,
            source_bridge_attempt_count: source_attempts.count,
            intermediate_bridge_attempt_count: intermediate_attempts.count,
            terminal_bridge_attempt_count: terminal_attempts.count,
            source_bridge_streamed_tick_count: source_attempts.streamed_tick_count,
            intermediate_bridge_streamed_tick_count: intermediate_attempts.streamed_tick_count,
            terminal_bridge_streamed_tick_count: terminal_attempts.streamed_tick_count,
            source_environment_rejection_count: source_attempts.environment_rejection_count,
            intermediate_environment_rejection_count: intermediate_attempts
                .environment_rejection_count,
            terminal_environment_rejection_count: terminal_attempts.environment_rejection_count,
            classification: CertificationV2::NotCertified,
            reasons: reasons.into_iter().collect(),
            margins: failed_component_margins(),
            total_fuel_burn_kg: None,
            total_time_s: None,
            identity: String::new(),
        }
    };
    candidate.identity = waypoint_candidate_identity(&candidate);
    candidate
}

fn waypoint_candidate_identity(candidate: &WaypointCandidateV2) -> String {
    digest(&WaypointCandidateIdentity {
        waypoint_position_m: candidate.waypoint_position_m,
        source_leg: &candidate.source_leg,
        target_leg: &candidate.target_leg,
        source_handoff: candidate.source_handoff,
        intermediate_entry_handoff: candidate.intermediate_entry_handoff,
        intermediate_exit_handoff: candidate.intermediate_exit_handoff,
        terminal_handoff: candidate.terminal_handoff,
        source_bridge: candidate.source_bridge.as_ref(),
        intermediate_bridge: candidate.intermediate_bridge.as_ref(),
        terminal_bridge: candidate.terminal_bridge.as_ref(),
        source_environment: candidate.source_environment.as_ref(),
        intermediate_environment: candidate.intermediate_environment.as_ref(),
        terminal_environment: candidate.terminal_environment.as_ref(),
        route_progress: candidate.route_progress.as_ref(),
        source_bridge_attempt_count: candidate.source_bridge_attempt_count,
        intermediate_bridge_attempt_count: candidate.intermediate_bridge_attempt_count,
        terminal_bridge_attempt_count: candidate.terminal_bridge_attempt_count,
        source_bridge_streamed_tick_count: candidate.source_bridge_streamed_tick_count,
        intermediate_bridge_streamed_tick_count: candidate.intermediate_bridge_streamed_tick_count,
        terminal_bridge_streamed_tick_count: candidate.terminal_bridge_streamed_tick_count,
        source_environment_rejection_count: candidate.source_environment_rejection_count,
        intermediate_environment_rejection_count: candidate
            .intermediate_environment_rejection_count,
        terminal_environment_rejection_count: candidate.terminal_environment_rejection_count,
        classification: candidate.classification,
        reasons: &candidate.reasons,
        margins: candidate.margins,
        total_fuel_burn_kg: candidate.total_fuel_burn_kg,
        total_time_s: candidate.total_time_s,
    })
}

#[derive(Serialize)]
struct WaypointCandidateIdentity<'a> {
    waypoint_position_m: Vec2,
    source_leg: &'a VirtualBallisticArcV2,
    target_leg: &'a VirtualBallisticArcV2,
    source_handoff: Option<HandoffV2>,
    intermediate_entry_handoff: Option<HandoffV2>,
    intermediate_exit_handoff: Option<HandoffV2>,
    terminal_handoff: Option<HandoffV2>,
    source_bridge: Option<&'a AnalyticalBridgeV2>,
    intermediate_bridge: Option<&'a AnalyticalBridgeV2>,
    terminal_bridge: Option<&'a AnalyticalBridgeV2>,
    source_environment: Option<&'a BridgeEnvironmentEvidenceV2>,
    intermediate_environment: Option<&'a BridgeEnvironmentEvidenceV2>,
    terminal_environment: Option<&'a BridgeEnvironmentEvidenceV2>,
    route_progress: Option<&'a RouteProgressEvidenceV2>,
    source_bridge_attempt_count: usize,
    intermediate_bridge_attempt_count: usize,
    terminal_bridge_attempt_count: usize,
    source_bridge_streamed_tick_count: usize,
    intermediate_bridge_streamed_tick_count: usize,
    terminal_bridge_streamed_tick_count: usize,
    source_environment_rejection_count: usize,
    intermediate_environment_rejection_count: usize,
    terminal_environment_rejection_count: usize,
    classification: CertificationV2,
    reasons: &'a [DirectBridgeReasonV2],
    margins: ComponentMarginsV2,
    total_fuel_burn_kg: Option<f64>,
    total_time_s: Option<f64>,
}

fn ridge_direct_diagnostic(candidate: &DirectBridgeCandidateV2) -> RidgeDirectDiagnosticV2 {
    RidgeDirectDiagnosticV2 {
        duration_multiplier: candidate.duration_multiplier,
        arc_steps: candidate.virtual_arc.steps,
        duration_s: candidate.virtual_arc.duration_s,
        classification: candidate.classification,
        reasons: candidate.reasons.clone(),
        minimum_normalized_margin: candidate.margins.minimum_normalized(),
        candidate_identity: candidate.identity.clone(),
    }
}

#[derive(Serialize)]
struct WaypointSearchIdentity<'a> {
    max_candidates: usize,
    waypoint_position_count: usize,
    leg_duration_pair_count: usize,
    candidate_count: usize,
    certified_candidate_count: usize,
    rejection_counts: &'a [(DirectBridgeReasonV2, usize)],
    selected_candidate_identity: Option<&'a String>,
    selected_candidate: Option<&'a WaypointCandidateV2>,
    candidates: &'a [WaypointCandidateV2],
}

fn waypoint_search(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    mesa: &MesaGeometryV2,
) -> WaypointSearchEvidenceV2 {
    let positions = waypoint_positions(policy, vehicle, case, mesa);
    let mut duration_pairs = Vec::new();
    for position in &positions {
        let source_durations = candidate_steps(
            policy,
            position.x - release_reference(case, policy, vehicle).x,
        );
        let target_durations =
            candidate_steps(policy, touchdown_reference(case, vehicle).x - position.x);
        for (source_multiplier, source_steps) in &source_durations {
            for (target_multiplier, target_steps) in &target_durations {
                duration_pairs.push((
                    *source_multiplier,
                    *target_multiplier,
                    *source_steps,
                    *target_steps,
                ));
            }
        }
    }
    duration_pairs.sort_by(|left, right| {
        (left.2 + left.3)
            .cmp(&(right.2 + right.3))
            .then_with(|| left.0.total_cmp(&right.0))
            .then_with(|| left.1.total_cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
    });
    let leg_duration_pair_count = duration_pairs.len();
    let mut rejection_counts = std::collections::BTreeMap::new();
    let mut selected_candidate = None;
    let mut candidate_count = 0;
    for position in positions.iter().copied() {
        let source_durations = candidate_steps(
            policy,
            position.x - release_reference(case, policy, vehicle).x,
        );
        let target_durations =
            candidate_steps(policy, touchdown_reference(case, vehicle).x - position.x);
        let mut pairs: Vec<_> = source_durations
            .iter()
            .flat_map(|(source_multiplier, source_steps)| {
                target_durations
                    .iter()
                    .map(move |(target_multiplier, target_steps)| {
                        (
                            *source_multiplier,
                            *target_multiplier,
                            *source_steps,
                            *target_steps,
                        )
                    })
            })
            .collect();
        pairs.sort_by(|left, right| {
            (left.2 + left.3)
                .cmp(&(right.2 + right.3))
                .then_with(|| left.0.total_cmp(&right.0))
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.3.cmp(&right.3))
        });
        for (source_multiplier, target_multiplier, _, _) in pairs {
            if candidate_count >= WAYPOINT_CANDIDATE_LIMIT_V2 {
                break;
            }
            candidate_count += 1;
            let candidate = evaluate_waypoint_candidate(
                policy,
                vehicle,
                case,
                &TerrainDefinition::Heightfield {
                    points_m: mesa.terrain_points_m.clone(),
                },
                position,
                source_multiplier,
                target_multiplier,
            );
            if candidate.classification == CertificationV2::Certified {
                selected_candidate = Some(candidate);
                break;
            }
            for reason in &candidate.reasons {
                *rejection_counts.entry(*reason).or_insert(0) += 1;
            }
        }
        if selected_candidate.is_some() || candidate_count >= WAYPOINT_CANDIDATE_LIMIT_V2 {
            break;
        }
    }
    let certified_candidate_count = usize::from(selected_candidate.is_some());
    let candidates = selected_candidate.clone().into_iter().collect::<Vec<_>>();
    let selected_candidate_identity = selected_candidate
        .as_ref()
        .map(|candidate| candidate.identity.clone());
    let rejection_counts = rejection_counts.into_iter().collect::<Vec<_>>();
    let mut evidence = WaypointSearchEvidenceV2 {
        max_candidates: WAYPOINT_CANDIDATE_LIMIT_V2,
        waypoint_position_count: positions.len(),
        leg_duration_pair_count,
        candidate_count,
        certified_candidate_count,
        rejection_counts,
        selected_candidate_identity,
        selected_candidate,
        candidates,
        identity: String::new(),
    };
    evidence.identity = digest(&WaypointSearchIdentity {
        max_candidates: evidence.max_candidates,
        waypoint_position_count: evidence.waypoint_position_count,
        leg_duration_pair_count: evidence.leg_duration_pair_count,
        candidate_count: evidence.candidate_count,
        certified_candidate_count: evidence.certified_candidate_count,
        rejection_counts: &evidence.rejection_counts,
        selected_candidate_identity: evidence.selected_candidate_identity.as_ref(),
        selected_candidate: evidence.selected_candidate.as_ref(),
        candidates: &evidence.candidates,
    });
    evidence
}

#[derive(Serialize)]
struct RidgeCanaryIdentity<'a> {
    schema_id: &'a str,
    schema_version: u32,
    source_case_id: &'a str,
    flat_control: &'a FlatTwinEvidenceV2,
    nominal_candidate_identity: &'a str,
    correction_envelope: &'a LocalCorrectionEnvelopeV2,
    mesa: &'a MesaGeometryV2,
    blocking_lane_valid: bool,
    direct_status: MissionStatusV2,
    direct_nominal_candidate: &'a RidgeDirectDiagnosticV2,
    direct_global_replans: &'a [RidgeDirectDiagnosticV2],
    waypoint_search: &'a WaypointSearchEvidenceV2,
}

fn evaluate_ridge_canary_case_internal(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
) -> Result<RidgeCanaryEvidenceV2, ExperimentalRidgeCandidateErrorV2> {
    let feature = ridge_feature(case);
    let flat_control = flat_twin_evidence(policy, vehicle, case);
    let nominal = flat_control.nominal_candidate.clone();
    let correction_envelope = correction_envelope(policy, vehicle, &feature, &nominal);
    let mesa = mesa_geometry(policy, vehicle, case, &feature, &correction_envelope)?;
    let derived_case = DirectBridgeProbeV2 {
        id: format!("{}_derived_mesa", case.id),
        source: case.source.clone(),
        target: case.target.clone(),
        terrain_points_m: mesa.terrain_points_m.clone(),
        initial_position_m: case.initial_position_m,
        initial_velocity_mps: case.initial_velocity_mps,
    };
    derived_case
        .validate(policy, vehicle)
        .map_err(|_| ExperimentalRidgeCandidateErrorV2::DerivedMesaGeometryInvalid)?;
    let duration_steps = canary_bridge_duration_steps(policy);
    let mut derived_candidates: Vec<_> = candidate_steps(
        policy,
        derived_case.target.center_x_m - derived_case.source.center_x_m,
    )
    .into_iter()
    .map(|(multiplier, steps)| {
        evaluate_direct_candidate_with_duration_steps(
            policy,
            vehicle,
            &derived_case,
            multiplier,
            steps,
            &duration_steps,
        )
    })
    .collect();
    derived_candidates.sort_by(|left, right| {
        left.virtual_arc
            .steps
            .cmp(&right.virtual_arc.steps)
            .then_with(|| {
                left.duration_multiplier
                    .total_cmp(&right.duration_multiplier)
            })
    });
    let direct_nominal_candidate = derived_candidates
        .iter()
        .find(|candidate| {
            candidate.virtual_arc.steps == nominal.virtual_arc.steps
                && (candidate.duration_multiplier - nominal.duration_multiplier).abs()
                    <= ENDPOINT_TOLERANCE
        })
        .ok_or(ExperimentalRidgeCandidateErrorV2::DerivedMesaGeometryInvalid)?;
    let direct_global_replans = derived_candidates
        .iter()
        .filter(|candidate| candidate.virtual_arc.steps != nominal.virtual_arc.steps)
        .map(ridge_direct_diagnostic)
        .collect::<Vec<_>>();
    let blocking_lane_valid =
        correction_envelope_is_valid(&correction_envelope) && mesa.blocks_full_correction_corridor;
    let nominal_lane_rejected =
        direct_nominal_candidate.classification != CertificationV2::Certified;
    let direct_status = if nominal_lane_rejected && blocking_lane_valid {
        MissionStatusV2::Red
    } else {
        MissionStatusV2::Green
    };
    let direct_nominal_diagnostic = ridge_direct_diagnostic(direct_nominal_candidate);
    let waypoint_search = waypoint_search(policy, vehicle, case, &mesa);
    let mut evidence = RidgeCanaryEvidenceV2 {
        schema_id: "conservative_ballistic_ridge_canary_v2".to_owned(),
        schema_version: 2,
        source_case_id: case.id.clone(),
        flat_control,
        nominal_candidate_identity: nominal.identity.clone(),
        correction_envelope,
        mesa,
        blocking_lane_valid,
        direct_status,
        direct_nominal_candidate: direct_nominal_diagnostic,
        direct_global_replans,
        waypoint_search,
        identity: String::new(),
    };
    evidence.identity = digest(&RidgeCanaryIdentity {
        schema_id: &evidence.schema_id,
        schema_version: evidence.schema_version,
        source_case_id: &evidence.source_case_id,
        flat_control: &evidence.flat_control,
        nominal_candidate_identity: &evidence.nominal_candidate_identity,
        correction_envelope: &evidence.correction_envelope,
        mesa: &evidence.mesa,
        blocking_lane_valid: evidence.blocking_lane_valid,
        direct_status: evidence.direct_status,
        direct_nominal_candidate: &evidence.direct_nominal_candidate,
        direct_global_replans: &evidence.direct_global_replans,
        waypoint_search: &evidence.waypoint_search,
    });
    Ok(evidence)
}

/// Evaluate the bounded ridge canary for an arbitrary validated probe. This
/// public entry point is useful for deterministic negative-control tests (for
/// example, replacing the feature terrain with its flat twin).
pub fn evaluate_ridge_canary_case_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
) -> Result<RidgeCanaryEvidenceV2, String> {
    policy.validate()?;
    vehicle.validate()?;
    case.validate(policy, vehicle)?;
    evaluate_ridge_canary_case_internal(policy, vehicle, case).map_err(|error| error.to_string())
}

/// Evaluate one arbitrary validated v2 direct-bridge probe with the same
/// analytical machinery used by the frozen fixture evaluator.
pub fn evaluate_direct_bridge_case_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
) -> Result<DirectBridgeProbeResultV2, String> {
    policy.validate()?;
    vehicle.validate()?;
    case.validate(policy, vehicle)?;
    Ok(evaluate_case(policy, vehicle, case))
}

/// Return the shared research waypoint spacing derived from the unrotated hull
/// envelope. This exposes proposal spacing to evaluators without generating or
/// modifying waypoint positions.
pub fn waypoint_position_step_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
) -> Result<Vec2, String> {
    policy.validate()?;
    vehicle.validate()?;
    let envelope = rotated_hull_envelope(None, vehicle, policy.minimum_clearance_m);
    let step = Vec2::new(
        envelope.horizontal_extent_m.max(policy.minimum_clearance_m),
        envelope.vertical_extent_m.max(policy.minimum_clearance_m),
    );
    if !step.x.is_finite() || !step.y.is_finite() {
        return Err("waypoint position step must be finite".to_owned());
    }
    Ok(step)
}

/// Search a caller-supplied, finite set of one-waypoint proposals. Exhaustion
/// means only that none of these proposals certified; it is not a physical
/// impossibility result.
pub fn evaluate_one_waypoint_case_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    waypoint_positions_m: &[Vec2],
) -> Result<WaypointSearchEvidenceV2, String> {
    policy.validate()?;
    vehicle.validate()?;
    case.validate(policy, vehicle)?;
    if !(1..=4).contains(&waypoint_positions_m.len()) {
        return Err("waypoint_positions_m must contain between one and four positions".to_owned());
    }

    let release_x_m = release_reference(case, policy, vehicle).x;
    let touchdown_x_m = touchdown_reference(case, vehicle).x;
    let domain_start_x_m = case
        .terrain_points_m
        .first()
        .expect("validated probe has terrain domain")
        .x;
    let domain_end_x_m = case
        .terrain_points_m
        .last()
        .expect("validated probe has terrain domain")
        .x;
    let legal_left_x_m = release_x_m.max(domain_start_x_m.min(domain_end_x_m));
    let legal_right_x_m = touchdown_x_m.min(domain_start_x_m.max(domain_end_x_m));

    let mut positions = waypoint_positions_m.to_vec();
    for (index, position) in positions.iter().enumerate() {
        if !position.x.is_finite() || !position.y.is_finite() {
            return Err(format!("waypoint position {index} must be finite"));
        }
    }
    positions.sort_by(|left, right| {
        left.x
            .total_cmp(&right.x)
            .then_with(|| left.y.total_cmp(&right.y))
    });
    for (index, position) in positions.iter().enumerate() {
        if position.x <= legal_left_x_m || position.x >= legal_right_x_m {
            return Err(format!(
                "waypoint position {index} must be strictly within the release-touchdown ground-track and terrain domain"
            ));
        }
        if positions.iter().take(index).any(|previous| {
            (previous.x - position.x).abs() <= ENDPOINT_TOLERANCE
                && (previous.y - position.y).abs() <= ENDPOINT_TOLERANCE
        }) {
            return Err("waypoint_positions_m must not contain duplicate positions".to_owned());
        }
    }

    let mut prepared_positions = Vec::with_capacity(positions.len());
    for position in positions.iter().copied() {
        let source_span_m = position.x - release_x_m;
        let target_span_m = touchdown_x_m - position.x;
        let mut pairs = Vec::with_capacity(16);
        for source_multiplier in policy.duration_multipliers.iter().copied() {
            let source_steps =
                candidate_steps_for_multiplier(policy, source_span_m, source_multiplier)?;
            for target_multiplier in policy.duration_multipliers.iter().copied() {
                let target_steps =
                    candidate_steps_for_multiplier(policy, target_span_m, target_multiplier)?;
                let duration_pair_steps = source_steps
                    .checked_add(target_steps)
                    .ok_or_else(|| "waypoint leg duration tick sum overflowed".to_owned())?;
                pairs.push((
                    duration_pair_steps,
                    source_multiplier,
                    target_multiplier,
                    source_steps,
                    target_steps,
                ));
            }
        }
        pairs.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.3.cmp(&right.3))
                .then_with(|| left.4.cmp(&right.4))
        });
        prepared_positions.push((position, pairs));
    }

    let max_candidates = positions.len() * 16;
    let leg_duration_pair_count = max_candidates;
    let mut rejection_counts = std::collections::BTreeMap::new();
    let mut selected_candidate = None;
    let mut candidate_count = 0;
    let probe_terrain = terrain(case);
    'positions: for (position, pairs) in &prepared_positions {
        for &(_, source_multiplier, target_multiplier, _, _) in pairs {
            candidate_count += 1;
            let candidate = evaluate_waypoint_candidate(
                policy,
                vehicle,
                case,
                &probe_terrain,
                *position,
                source_multiplier,
                target_multiplier,
            );
            if candidate.classification == CertificationV2::Certified {
                selected_candidate = Some(candidate);
                break 'positions;
            }
            for reason in &candidate.reasons {
                *rejection_counts.entry(*reason).or_insert(0) += 1;
            }
        }
    }

    let certified_candidate_count = usize::from(selected_candidate.is_some());
    let candidates = selected_candidate.clone().into_iter().collect::<Vec<_>>();
    let selected_candidate_identity = selected_candidate
        .as_ref()
        .map(|candidate| candidate.identity.clone());
    let rejection_counts = rejection_counts.into_iter().collect::<Vec<_>>();
    let mut evidence = WaypointSearchEvidenceV2 {
        max_candidates,
        waypoint_position_count: positions.len(),
        leg_duration_pair_count,
        candidate_count,
        certified_candidate_count,
        rejection_counts,
        selected_candidate_identity,
        selected_candidate,
        candidates,
        identity: String::new(),
    };
    #[derive(Serialize)]
    struct OneWaypointCaseIdentityV2<'a> {
        policy: &'a DirectBridgePolicyV2,
        vehicle: &'a VehicleInputV2,
        case: &'a DirectBridgeProbeV2,
        waypoint_positions_m: &'a [Vec2],
        max_candidates: usize,
        waypoint_position_count: usize,
        leg_duration_pair_count: usize,
        candidate_count: usize,
        certified_candidate_count: usize,
        rejection_counts: &'a [(DirectBridgeReasonV2, usize)],
        selected_candidate_identity: Option<&'a String>,
        selected_candidate: Option<&'a WaypointCandidateV2>,
        candidates: &'a [WaypointCandidateV2],
    }
    evidence.identity = digest(&OneWaypointCaseIdentityV2 {
        policy,
        vehicle,
        case,
        waypoint_positions_m: &positions,
        max_candidates: evidence.max_candidates,
        waypoint_position_count: evidence.waypoint_position_count,
        leg_duration_pair_count: evidence.leg_duration_pair_count,
        candidate_count: evidence.candidate_count,
        certified_candidate_count: evidence.certified_candidate_count,
        rejection_counts: &evidence.rejection_counts,
        selected_candidate_identity: evidence.selected_candidate_identity.as_ref(),
        selected_candidate: evidence.selected_candidate.as_ref(),
        candidates: &evidence.candidates,
    });
    Ok(evidence)
}

pub fn evaluate_ridge_canary_fixture_v2(
    fixture: &DirectBridgeFixtureV2,
) -> Result<RidgeCanaryEvidenceV2, String> {
    fixture.validate()?;
    let case = fixture
        .cases
        .iter()
        .find(|case| case.id == "ridge_probe")
        .expect("validated fixture has ridge_probe");
    evaluate_ridge_canary_case_v2(&fixture.policy, &fixture.vehicle, case)
}

/// Evaluate a validated v2 fixture without invoking the production planner,
/// controller, or simulation.
pub fn evaluate_fixture_v2(
    fixture: &DirectBridgeFixtureV2,
) -> Result<DirectBridgeEvaluationV2, String> {
    fixture.validate()?;
    let mut cases: Vec<_> = fixture.cases.iter().collect();
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    let results = cases
        .into_iter()
        .map(|case| evaluate_case(&fixture.policy, &fixture.vehicle, case))
        .collect();
    let ridge_case = fixture
        .cases
        .iter()
        .find(|case| case.id == "ridge_probe")
        .expect("validated fixture has ridge_probe");
    let ridge_canary =
        evaluate_ridge_canary_case_internal(&fixture.policy, &fixture.vehicle, ridge_case)
            .map_err(|error| error.to_string())?;
    let mut evaluation = DirectBridgeEvaluationV2 {
        fixture_identity: digest(fixture),
        policy_identity: digest(&fixture.policy),
        vehicle_identity: digest(&fixture.vehicle),
        results,
        ridge_canary,
        identity: String::new(),
    };
    evaluation.identity = digest(&EvaluationIdentity {
        fixture_identity: &evaluation.fixture_identity,
        policy_identity: &evaluation.policy_identity,
        vehicle_identity: &evaluation.vehicle_identity,
        results: &evaluation.results,
        ridge_canary: &evaluation.ridge_canary,
    });
    Ok(evaluation)
}

#[derive(Serialize)]
struct EvaluationIdentity<'a> {
    fixture_identity: &'a str,
    policy_identity: &'a str,
    vehicle_identity: &'a str,
    results: &'a [DirectBridgeProbeResultV2],
    ridge_canary: &'a RidgeCanaryEvidenceV2,
}

/// Evaluate the frozen v2 probe corpus.
pub fn evaluate_embedded_fixture_v2() -> DirectBridgeEvaluationV2 {
    let fixture = load_embedded_fixture_v2();
    evaluate_fixture_v2(&fixture).expect("embedded v2 direct bridge fixture evaluates")
}

/// Reject reordered, altered, or otherwise tampered evaluation output by
/// recomputing the canonical embedded fixture evaluation.
pub fn validate_embedded_evaluation_v2(
    evaluation: &DirectBridgeEvaluationV2,
) -> Result<(), String> {
    let expected = evaluate_embedded_fixture_v2();
    if evaluation != &expected {
        return Err(
            "v2 direct bridge evaluation does not recompute from the embedded fixture".to_owned(),
        );
    }
    Ok(())
}

fn report_identity_v2(report: &DirectBridgeReportV2) -> String {
    digest(&ReportIdentityV2 {
        schema_id: &report.schema_id,
        schema_version: report.schema_version,
        fixture: &report.fixture,
        evaluation: &report.evaluation,
    })
}

#[derive(Serialize)]
struct ReportIdentityV2<'a> {
    schema_id: &'a str,
    schema_version: u32,
    fixture: &'a DirectBridgeFixtureV2,
    evaluation: &'a DirectBridgeEvaluationV2,
}

/// Build the complete static report projection from the embedded frozen v2
/// contract. Evaluation is performed exactly once for this artifact.
pub fn build_report_artifact_v2() -> DirectBridgeReportV2 {
    let fixture = load_embedded_fixture_v2();
    let mut evaluation = evaluate_fixture_v2(&fixture)
        .expect("embedded v2 direct bridge fixture evaluates for report artifact");
    // Per-tick bridge samples are deterministic derivatives of the stored
    // affine coefficients. Keep the reloadable report analytical and compact;
    // renderers can reconstruct any desired display sample exactly.
    for candidate in evaluation
        .results
        .iter_mut()
        .flat_map(|result| result.candidates.iter_mut())
    {
        clear_candidate_samples(candidate);
    }
    clear_candidate_samples(&mut evaluation.ridge_canary.flat_control.nominal_candidate);
    if let Some(candidate) = evaluation
        .ridge_canary
        .waypoint_search
        .selected_candidate
        .as_mut()
    {
        clear_waypoint_candidate_samples(candidate);
    }
    for candidate in &mut evaluation.ridge_canary.waypoint_search.candidates {
        clear_waypoint_candidate_samples(candidate);
    }
    let mut report = DirectBridgeReportV2 {
        schema_id: REPORT_SCHEMA_ID_V2.to_owned(),
        schema_version: REPORT_SCHEMA_VERSION_V2,
        fixture,
        evaluation,
        identity: String::new(),
    };
    report.identity = report_identity_v2(&report);
    report
}

fn clear_candidate_samples(candidate: &mut DirectBridgeCandidateV2) {
    if let Some(bridge) = candidate.source_bridge.as_mut() {
        bridge.samples.clear();
    }
    if let Some(bridge) = candidate.terminal_bridge.as_mut() {
        bridge.samples.clear();
    }
}

fn clear_waypoint_candidate_samples(candidate: &mut WaypointCandidateV2) {
    for bridge in [
        candidate.source_bridge.as_mut(),
        candidate.intermediate_bridge.as_mut(),
        candidate.terminal_bridge.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        bridge.samples.clear();
    }
}

fn embedded_report_artifact_v2() -> &'static DirectBridgeReportV2 {
    static REPORT: std::sync::OnceLock<DirectBridgeReportV2> = std::sync::OnceLock::new();
    REPORT.get_or_init(build_report_artifact_v2)
}

/// Reject an altered report by comparing it with the canonical projection
/// rebuilt from the frozen embedded fixture. This covers nested fixture,
/// evaluation, selected-identity, counter, coefficient, ordering, and digest
/// data. Per-tick bridge samples are reconstructed from the compact affine
/// coefficients and are not report payload.
pub fn validate_report_artifact_v2(report: &DirectBridgeReportV2) -> Result<(), String> {
    if report.schema_id != REPORT_SCHEMA_ID_V2 {
        return Err(format!(
            "v2 report schema_id must equal {REPORT_SCHEMA_ID_V2}"
        ));
    }
    if report.schema_version != REPORT_SCHEMA_VERSION_V2 {
        return Err(format!(
            "v2 report schema_version must equal {REPORT_SCHEMA_VERSION_V2}"
        ));
    }
    let expected = embedded_report_artifact_v2();
    if report.fixture != expected.fixture {
        return Err("v2 report fixture does not match the embedded frozen contract".to_owned());
    }
    if report.evaluation != expected.evaluation {
        return Err(
            "v2 report evaluation does not recompute from the embedded frozen contract".to_owned(),
        );
    }
    if report.identity != expected.identity || report.identity != report_identity_v2(report) {
        return Err("v2 report identity does not bind its canonical contents".to_owned());
    }
    Ok(())
}

/// Build the compact, feature-gated candidate projection used by the frozen
/// ridge controller shadow.  The production planner and its `plan()` entry
/// point are not consulted or modified.
pub fn evaluate_embedded_experimental_ridge_candidate_v2()
-> Result<ExperimentalRidgeCandidateProjectionV2, ExperimentalRidgeCandidateErrorV2> {
    static PROJECTION: std::sync::OnceLock<
        Result<ExperimentalRidgeCandidateProjectionV2, ExperimentalRidgeCandidateErrorV2>,
    > = std::sync::OnceLock::new();
    PROJECTION
        .get_or_init(|| {
            let report = embedded_report_artifact_v2();
            validate_report_artifact_v2(report)
                .map_err(|_| ExperimentalRidgeCandidateErrorV2::EmbeddedReportInvalid)?;
            let projection = experimental_ridge_projection_from_report_v2(report)?;
            validate_experimental_ridge_projection_structure_v2(&projection)?;
            Ok(projection)
        })
        .clone()
}

/// Reject altered candidate evidence before it reaches an evaluator.  The
/// structural pass returns specific certificate/crossing failures; the final
/// canonical comparison catches all other changes to the frozen projection.
pub fn validate_embedded_experimental_ridge_candidate_v2(
    projection: &ExperimentalRidgeCandidateProjectionV2,
) -> Result<(), ExperimentalRidgeCandidateErrorV2> {
    validate_experimental_ridge_projection_structure_v2(projection)?;
    let expected = evaluate_embedded_experimental_ridge_candidate_v2()?;
    if projection != &expected {
        return Err(ExperimentalRidgeCandidateErrorV2::ProjectionMismatch);
    }
    Ok(())
}

impl ExperimentalRidgeCaseInputV1 {
    /// Construct one identity-bound generic ridge input.  This is the only
    /// public constructor that accepts raw case contents; all analytical and
    /// runtime evidence is derived by the evaluator/projector APIs below.
    pub fn new(
        policy: DirectBridgePolicyV2,
        vehicle: VehicleInputV2,
        probe: DirectBridgeProbeV2,
    ) -> Result<Self, ExperimentalRidgeCandidateErrorV2> {
        let mut input = Self {
            schema_id: EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_ID_V1.to_owned(),
            schema_version: EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_VERSION_V1,
            policy,
            vehicle,
            probe,
            identity: String::new(),
        };
        // Validate raw contents before the infallible JSON digest.  In
        // particular, serde rejects non-finite floats, so digesting first
        // would convert an ordinary malformed public input into a panic.
        validate_experimental_ridge_case_input_contents_v1(
            &input.policy,
            &input.vehicle,
            &input.probe,
        )?;
        input.identity = experimental_ridge_case_input_identity_v1(&input);
        Ok(input)
    }

    /// Validate the versioned input boundary before any analytical work.  A
    /// single `probe` field makes the one-case invariant structural rather
    /// than an ID-based selection rule.
    pub fn validate(&self) -> Result<(), ExperimentalRidgeCandidateErrorV2> {
        if self.schema_id != EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_ID_V1
            || self.schema_version != EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_VERSION_V1
        {
            return Err(ExperimentalRidgeCandidateErrorV2::GenericInputInvalid);
        }
        validate_experimental_ridge_case_input_contents_v1(
            &self.policy,
            &self.vehicle,
            &self.probe,
        )?;
        if self.identity != experimental_ridge_case_input_identity_v1(self) {
            return Err(ExperimentalRidgeCandidateErrorV2::GenericInputIdentityMismatch);
        }
        Ok(())
    }
}

impl ExperimentalRidgeCaseManifestV1 {
    /// Construct an identity-bound manifest from the complete ordered input
    /// cases.  No analytical evaluation is performed here.
    pub fn new(
        cases: Vec<ExperimentalRidgeCaseInputV1>,
    ) -> Result<Self, ExperimentalRidgeCaseManifestErrorV1> {
        let mut manifest = Self {
            schema_id: EXPERIMENTAL_RIDGE_CASE_MANIFEST_SCHEMA_ID_V1.to_owned(),
            schema_version: EXPERIMENTAL_RIDGE_CASE_MANIFEST_SCHEMA_VERSION_V1,
            cases,
            identity: String::new(),
        };
        manifest.validate_contents()?;
        manifest.identity = experimental_ridge_case_manifest_identity_v1(&manifest);
        Ok(manifest)
    }

    /// Validate the frozen two-case manifest before handing its raw inputs to
    /// a later analytical stage.  Case IDs are checked only for manifest
    /// integrity; they are not used by the generic input evaluator.
    pub fn validate(&self) -> Result<(), ExperimentalRidgeCaseManifestErrorV1> {
        self.validate_contents()?;
        if self.identity != experimental_ridge_case_manifest_identity_v1(self) {
            return Err(ExperimentalRidgeCaseManifestErrorV1::IdentityMismatch);
        }
        Ok(())
    }

    fn validate_contents(&self) -> Result<(), ExperimentalRidgeCaseManifestErrorV1> {
        if self.schema_id != EXPERIMENTAL_RIDGE_CASE_MANIFEST_SCHEMA_ID_V1
            || self.schema_version != EXPERIMENTAL_RIDGE_CASE_MANIFEST_SCHEMA_VERSION_V1
        {
            return Err(ExperimentalRidgeCaseManifestErrorV1::SchemaMismatch);
        }
        if self.cases.len() != EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1.len() {
            return Err(ExperimentalRidgeCaseManifestErrorV1::CaseCountMismatch);
        }
        let mut ids = std::collections::BTreeSet::new();
        for (case, expected_id) in self
            .cases
            .iter()
            .zip(EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1)
        {
            if case.probe.id != expected_id {
                return Err(ExperimentalRidgeCaseManifestErrorV1::CaseOrderMismatch);
            }
            if !ids.insert(case.probe.id.clone()) {
                return Err(ExperimentalRidgeCaseManifestErrorV1::DuplicateCaseId);
            }
            case.validate()
                .map_err(|_| ExperimentalRidgeCaseManifestErrorV1::InputInvalid)?;
        }
        Ok(())
    }
}

/// Parse and validate a versioned held-out raw-input manifest.  This function
/// performs only JSON decoding, raw-input validation, and identity checking;
/// it never evaluates an analytical candidate or builds a runtime route.
pub fn parse_experimental_ridge_case_manifest_v1(
    raw: &str,
) -> Result<ExperimentalRidgeCaseManifestV1, ExperimentalRidgeCaseManifestErrorV1> {
    let manifest: ExperimentalRidgeCaseManifestV1 =
        serde_json::from_str(raw).map_err(|_| ExperimentalRidgeCaseManifestErrorV1::Json)?;
    manifest.validate()?;
    Ok(manifest)
}

/// Load the committed H2 raw-input manifest.  The embedded fixture is
/// validated before it is returned; callers receive only input data and its
/// identity chain.
pub fn load_experimental_ridge_case_manifest_v1() -> ExperimentalRidgeCaseManifestV1 {
    parse_experimental_ridge_case_manifest_v1(HELDOUT_INPUT_MANIFEST_V1)
        .expect("valid experimental ridge held-out input manifest v1")
}

fn validate_experimental_ridge_case_input_contents_v1(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    probe: &DirectBridgeProbeV2,
) -> Result<(), ExperimentalRidgeCandidateErrorV2> {
    policy
        .validate()
        .map_err(|_| ExperimentalRidgeCandidateErrorV2::GenericInputInvalid)?;
    vehicle
        .validate()
        .map_err(|_| ExperimentalRidgeCandidateErrorV2::GenericInputInvalid)?;
    if vehicle.derated_max_acceleration_mps2(policy) <= 0.0 {
        return Err(ExperimentalRidgeCandidateErrorV2::GenericInputInvalid);
    }
    probe
        .validate(policy, vehicle)
        .map_err(|_| ExperimentalRidgeCandidateErrorV2::GenericInputInvalid)
}

/// Evaluate one arbitrary, identity-bound ridge case.  The canary, mesa,
/// candidate decisions, and crossing evidence are all rebuilt from this input
/// rather than accepted from the caller.
pub fn evaluate_experimental_ridge_case_projection_v1(
    input: &ExperimentalRidgeCaseInputV1,
) -> Result<ExperimentalRidgeCaseProjectionV1, ExperimentalRidgeCandidateErrorV2> {
    input.validate()?;
    experimental_ridge_case_projection_from_validated_input_v1(input)
}

/// Reject a generic projection unless it is structurally valid, carries a
/// valid input identity, and exact-compares to a fresh analytical
/// recomputation from the contained input fields.
pub fn validate_experimental_ridge_case_projection_v1(
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> Result<(), ExperimentalRidgeCandidateErrorV2> {
    if projection.schema_id != EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_ID_V1
        || projection.schema_version != EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_VERSION_V1
    {
        return Err(ExperimentalRidgeCandidateErrorV2::GenericProjectionSchemaMismatch);
    }
    if projection.identity != experimental_ridge_case_projection_identity_v1(projection) {
        return Err(ExperimentalRidgeCandidateErrorV2::GenericProjectionIdentityMismatch);
    }
    let input = ExperimentalRidgeCaseInputV1 {
        schema_id: EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_ID_V1.to_owned(),
        schema_version: EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_VERSION_V1,
        policy: projection.policy.clone(),
        vehicle: projection.vehicle.clone(),
        probe: projection.probe.clone(),
        identity: projection.input_identity.clone(),
    };
    input.validate()?;
    validate_experimental_ridge_projection_structure_components_v2(
        &projection.policy,
        &projection.probe,
        &projection.flat_candidate_identities,
        &projection.waypoint_candidate_identities,
        &projection.flat_control,
        &projection.derived_mesa,
    )?;
    let expected = experimental_ridge_case_projection_from_validated_input_v1(&input)?;
    if projection != &expected {
        return Err(ExperimentalRidgeCandidateErrorV2::GenericProjectionMismatch);
    }
    Ok(())
}

fn experimental_ridge_case_projection_from_validated_input_v1(
    input: &ExperimentalRidgeCaseInputV1,
) -> Result<ExperimentalRidgeCaseProjectionV1, ExperimentalRidgeCandidateErrorV2> {
    let canary = evaluate_ridge_canary_case_internal(&input.policy, &input.vehicle, &input.probe)?;
    let components = experimental_ridge_projection_components_v2(
        &input.policy,
        &input.vehicle,
        &input.probe,
        &canary,
    )?;
    let mut projection = ExperimentalRidgeCaseProjectionV1 {
        schema_id: EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_ID_V1.to_owned(),
        schema_version: EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_VERSION_V1,
        input_identity: input.identity.clone(),
        analytical_canary_identity: components.analytical_canary_identity,
        policy: components.policy,
        vehicle: components.vehicle,
        probe: components.case,
        mesa: components.mesa,
        flat_candidate_identities: components.flat_candidate_identities,
        ridge_direct_candidate_identities: components.ridge_direct_candidate_identities,
        derived_nominal_direct: components.derived_nominal_direct,
        derived_non_nominal_direct_diagnostics: components.derived_non_nominal_direct_diagnostics,
        derived_blocker_valid: components.derived_blocker_valid,
        waypoint_certified_candidate_count: components.waypoint_certified_candidate_count,
        waypoint_search_identity: components.waypoint_search_identity,
        waypoint_candidate_identities: components.waypoint_candidate_identities,
        flat_control: components.flat_control,
        derived_mesa: components.derived_mesa,
        identity: String::new(),
    };
    canonicalize_experimental_ridge_candidate_outcome_samples_v2(&mut projection.flat_control);
    canonicalize_experimental_ridge_candidate_outcome_samples_v2(&mut projection.derived_mesa);
    projection.identity = experimental_ridge_case_projection_identity_v1(&projection);
    Ok(projection)
}

/// Candidate bridges materialize their affine samples only as a runtime
/// convenience.  V1 projections are serialized evidence, so canonicalize
/// that transient state before both identity construction and equality-based
/// recomputation validation.
fn canonicalize_experimental_ridge_candidate_outcome_samples_v2(
    outcome: &mut ExperimentalRidgeCandidateOutcomeV2,
) {
    match outcome {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => {
            clear_candidate_samples(candidate);
        }
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } => {
            clear_waypoint_candidate_samples(candidate);
        }
        ExperimentalRidgeCandidateOutcomeV2::Unsupported { .. } => {}
    }
}

/// Project the validated experimental V2 candidate decisions into the
/// controller-facing route contract.  This is deliberately separate from the
/// production [`crate::plan`] entry point and is only available with the
/// conservative-ballistic-report feature (or in tests).
pub fn project_experimental_ridge_runtime_v2(
    projection: &ExperimentalRidgeCandidateProjectionV2,
) -> Result<ExperimentalRidgeRuntimeProjectionV2, ExperimentalRidgeRuntimeProjectionErrorV2> {
    // The canonical validator is intentionally first: no route is built from
    // altered, incomplete, or reordered analytical evidence.
    validate_embedded_experimental_ridge_candidate_v2(projection)
        .map_err(ExperimentalRidgeRuntimeProjectionErrorV2::CandidateEvidence)?;

    let (flat_control, derived_mesa) = project_ridge_runtime_outcomes_v2(
        &projection.policy,
        &projection.vehicle,
        &projection.case,
        &projection.mesa,
        &projection.flat_control,
        &projection.derived_mesa,
    )?;

    let mut runtime = ExperimentalRidgeRuntimeProjectionV2 {
        schema_id: EXPERIMENTAL_RIDGE_RUNTIME_PROJECTION_SCHEMA_ID_V2.to_owned(),
        schema_version: EXPERIMENTAL_RIDGE_RUNTIME_PROJECTION_SCHEMA_VERSION_V2,
        analytical_report_identity: projection.analytical_report_identity.clone(),
        analytical_canary_identity: projection.analytical_canary_identity.clone(),
        policy: projection.policy.clone(),
        vehicle: projection.vehicle.clone(),
        case: projection.case.clone(),
        mesa: projection.mesa.clone(),
        flat_control,
        derived_mesa,
        identity: String::new(),
    };
    runtime.identity = runtime_projection_identity_v2(&runtime);
    Ok(runtime)
}

/// Build the cached embedded V2 runtime projection used by downstream
/// evaluators.  The candidate projection itself remains the canonical source
/// of analytical evidence and is validated before route construction.
pub fn evaluate_embedded_experimental_ridge_runtime_v2()
-> Result<ExperimentalRidgeRuntimeProjectionV2, ExperimentalRidgeRuntimeProjectionErrorV2> {
    static PROJECTION: std::sync::OnceLock<
        Result<ExperimentalRidgeRuntimeProjectionV2, ExperimentalRidgeRuntimeProjectionErrorV2>,
    > = std::sync::OnceLock::new();
    PROJECTION
        .get_or_init(|| {
            let candidate = evaluate_embedded_experimental_ridge_candidate_v2()
                .map_err(ExperimentalRidgeRuntimeProjectionErrorV2::CandidateEvidence)?;
            project_experimental_ridge_runtime_v2(&candidate)
        })
        .clone()
}

/// Validate a runtime projection against the candidate projection that
/// produced it.  Rebuilding the projection catches tampering in the route,
/// tangent, bounds, authority, handoff evidence, and either stable identity.
pub fn validate_experimental_ridge_runtime_v2(
    candidate: &ExperimentalRidgeCandidateProjectionV2,
    runtime: &ExperimentalRidgeRuntimeProjectionV2,
) -> Result<(), ExperimentalRidgeRuntimeProjectionErrorV2> {
    let expected = project_experimental_ridge_runtime_v2(candidate)?;
    if runtime != &expected {
        return Err(ExperimentalRidgeRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch);
    }
    Ok(())
}

/// Project a generic, recomputation-validated V1 case into the same bounded
/// controller-facing route components used by the frozen V2 canary.  This is
/// not a production planner entry point.
pub fn project_experimental_ridge_case_runtime_v1(
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> Result<ExperimentalRidgeCaseRuntimeProjectionV1, ExperimentalRidgeRuntimeProjectionErrorV2> {
    validate_experimental_ridge_case_projection_v1(projection)
        .map_err(ExperimentalRidgeRuntimeProjectionErrorV2::CandidateEvidence)?;
    let (flat_control, derived_mesa) = project_ridge_runtime_outcomes_v2(
        &projection.policy,
        &projection.vehicle,
        &projection.probe,
        &projection.mesa,
        &projection.flat_control,
        &projection.derived_mesa,
    )?;
    let mut runtime = ExperimentalRidgeCaseRuntimeProjectionV1 {
        schema_id: EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V1.to_owned(),
        schema_version: EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V1,
        input_identity: projection.input_identity.clone(),
        analytical_canary_identity: projection.analytical_canary_identity.clone(),
        policy: projection.policy.clone(),
        vehicle: projection.vehicle.clone(),
        probe: projection.probe.clone(),
        mesa: projection.mesa.clone(),
        flat_control,
        derived_mesa,
        identity: String::new(),
    };
    runtime.identity = experimental_ridge_case_runtime_projection_identity_v1(&runtime);
    Ok(runtime)
}

/// Reject an altered generic runtime artifact by rebuilding it from the
/// validated generic candidate projection.
pub fn validate_experimental_ridge_case_runtime_v1(
    candidate: &ExperimentalRidgeCaseProjectionV1,
    runtime: &ExperimentalRidgeCaseRuntimeProjectionV1,
) -> Result<(), ExperimentalRidgeRuntimeProjectionErrorV2> {
    if runtime.schema_id != EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V1
        || runtime.schema_version != EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V1
        || runtime.identity != experimental_ridge_case_runtime_projection_identity_v1(runtime)
    {
        return Err(ExperimentalRidgeRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch);
    }
    let expected = project_experimental_ridge_case_runtime_v1(candidate)?;
    if runtime != &expected {
        return Err(ExperimentalRidgeRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch);
    }
    Ok(())
}

/// Project a generic, recomputation-validated V1 analytical case through the
/// V2 runtime handoff selector.  The analytical candidate projection remains
/// V1; only this runtime boundary is versioned so the historical crossing-only
/// V1 result remains immutable.
pub fn project_experimental_ridge_case_runtime_v2(
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> Result<ExperimentalRidgeCaseRuntimeProjectionV2, ExperimentalRidgeCaseRuntimeProjectionErrorV2>
{
    validate_experimental_ridge_case_projection_v1(projection)
        .map_err(ExperimentalRidgeCaseRuntimeProjectionErrorV2::CandidateEvidence)?;
    let (flat_control, derived_mesa) = project_ridge_case_runtime_outcomes_v2(
        &projection.policy,
        &projection.vehicle,
        &projection.probe,
        &projection.mesa,
        &projection.flat_control,
        &projection.derived_mesa,
    )?;
    let mut runtime = ExperimentalRidgeCaseRuntimeProjectionV2 {
        schema_id: EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V2.to_owned(),
        schema_version: EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V2,
        input_identity: projection.input_identity.clone(),
        analytical_canary_identity: projection.analytical_canary_identity.clone(),
        policy: projection.policy.clone(),
        vehicle: projection.vehicle.clone(),
        probe: projection.probe.clone(),
        mesa: projection.mesa.clone(),
        flat_control,
        derived_mesa,
        identity: String::new(),
    };
    runtime.identity = experimental_ridge_case_runtime_projection_identity_v2(&runtime);
    Ok(runtime)
}

/// Evaluate one generic input and project it through the V2 runtime boundary.
/// This convenience API performs the same analytical recomputation as the
/// explicit candidate-then-runtime sequence, without changing the V1 input or
/// candidate schemas.
pub fn evaluate_experimental_ridge_case_runtime_v2(
    input: &ExperimentalRidgeCaseInputV1,
) -> Result<ExperimentalRidgeCaseRuntimeProjectionV2, ExperimentalRidgeCaseRuntimeProjectionErrorV2>
{
    let projection = evaluate_experimental_ridge_case_projection_v1(input)
        .map_err(ExperimentalRidgeCaseRuntimeProjectionErrorV2::CandidateEvidence)?;
    project_experimental_ridge_case_runtime_v2(&projection)
}

/// Validate a generic V2 runtime artifact by rebuilding it from its
/// identity-bound analytical candidate projection.  This catches tampering in
/// either route outcome, every handoff attempt, the selected state, seam
/// metadata, or the projection identity.
pub fn validate_experimental_ridge_case_runtime_v2(
    candidate: &ExperimentalRidgeCaseProjectionV1,
    runtime: &ExperimentalRidgeCaseRuntimeProjectionV2,
) -> Result<(), ExperimentalRidgeCaseRuntimeProjectionErrorV2> {
    if runtime.schema_id != EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V2
        || runtime.schema_version != EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V2
        || runtime.identity != experimental_ridge_case_runtime_projection_identity_v2(runtime)
    {
        return Err(
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch,
        );
    }
    let expected = project_experimental_ridge_case_runtime_v2(candidate)?;
    if runtime != &expected {
        return Err(
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch,
        );
    }
    Ok(())
}

fn runtime_request_components_v2(
    policy: &DirectBridgePolicyV2,
    vehicle_input: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    terrain: TerrainDefinition,
) -> Result<RoutePlanningRequest, ExperimentalRidgeRuntimeProjectionErrorV2> {
    let vehicle = VehicleSpec {
        geometry: VehicleGeometry {
            hull_width_m: vehicle_input.geometry.hull_width_m,
            hull_height_m: vehicle_input.geometry.hull_height_m,
            touchdown_half_span_m: vehicle_input.geometry.touchdown_half_span_m,
            touchdown_base_offset_m: vehicle_input.geometry.touchdown_base_offset_m,
        },
        dry_mass_kg: vehicle_input.dry_mass_kg,
        initial_fuel_kg: vehicle_input.initial_fuel_kg,
        max_fuel_kg: vehicle_input.max_fuel_kg,
        max_thrust_n: vehicle_input.max_thrust_n,
        max_fuel_burn_kgps: vehicle_input.max_fuel_burn_kgps,
        min_throttle_frac: vehicle_input.min_throttle_frac,
        max_rotation_rate_radps: vehicle_input.max_rotation_rate_radps,
        safe_touchdown_normal_speed_mps: vehicle_input.safe_touchdown_normal_speed_mps,
        safe_touchdown_tangential_speed_mps: vehicle_input.safe_touchdown_tangential_speed_mps,
        safe_touchdown_attitude_error_rad: vehicle_input.safe_touchdown_attitude_error_rad,
        safe_touchdown_angular_rate_radps: vehicle_input.safe_touchdown_angular_rate_radps,
    };
    let request = RoutePlanningRequest {
        world: WorldSpec {
            gravity_mps2: policy.gravity_mps2,
            terrain,
            landing_pads: vec![
                LandingPadSpec {
                    id: "source".to_owned(),
                    center_x_m: case.source.center_x_m,
                    surface_y_m: case.source.surface_y_m,
                    width_m: case.source.width_m,
                },
                LandingPadSpec {
                    id: "target".to_owned(),
                    center_x_m: case.target.center_x_m,
                    surface_y_m: case.target.surface_y_m,
                    width_m: case.target.width_m,
                },
            ],
        },
        vehicle,
        initial_state: VehicleInitialState {
            position_m: case.initial_position_m,
            velocity_mps: case.initial_velocity_mps,
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        source_pad_id: "source".to_owned(),
        target_pad_id: "target".to_owned(),
        policy: RoutePlanningPolicy::v1(),
    };
    request
        .validate()
        .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::InvalidRuntimeRequest)?;
    Ok(request)
}

#[derive(Clone)]
struct RuntimeWaypointEvidenceCaseV2 {
    selected_state: KinematicStateV2,
    route: TransferRouteSpec,
    authority: WaypointAuthorityDiagnostics,
    handoff_kinematics: ExperimentalRidgeRuntimeWaypointHandoffKinematicsV2,
    handoff_assessment: ExperimentalRidgeRuntimeWaypointHandoffAssessmentV2,
    structural_validation: ExperimentalRidgeRuntimeRouteStructureV2,
}

fn map_case_runtime_error_v2(
    error: ExperimentalRidgeRuntimeProjectionErrorV2,
) -> ExperimentalRidgeCaseRuntimeProjectionErrorV2 {
    match error {
        ExperimentalRidgeRuntimeProjectionErrorV2::CandidateEvidence(error) => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::CandidateEvidence(error)
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::InvalidRuntimeRequest => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::InvalidRuntimeRequest
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::WaypointRouteGeometryInvalid => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::WaypointRouteGeometryInvalid
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::WaypointTangentInvalid => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::WaypointTangentInvalid
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::WaypointAuthorityInvalid => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::WaypointAuthorityInvalid
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::WaypointAuthorityBelowMinimum => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::WaypointAuthorityBelowMinimum
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::RouteStructuralValidationFailed => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::RouteStructuralValidationFailed
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::HandoffKinematicsInvalid => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::HandoffKinematicsInvalid
        }
        // The historical crossing-only wrapper is the only caller that can
        // observe this old error.  The shared construction helper never
        // emits it, so a future violation of that invariant fails closed at
        // the new boundary without introducing a duplicate plain-contract
        // variant.
        ExperimentalRidgeRuntimeProjectionErrorV2::HandoffContractFailed => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch
        }
        ExperimentalRidgeRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch => {
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch
        }
    }
}

fn project_ridge_case_runtime_outcomes_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    mesa: &MesaGeometryV2,
    flat_candidate: &ExperimentalRidgeCandidateOutcomeV2,
    derived_mesa_candidate: &ExperimentalRidgeCandidateOutcomeV2,
) -> Result<
    (
        ExperimentalRidgeCaseRuntimeOutcomeV2,
        ExperimentalRidgeCaseRuntimeOutcomeV2,
    ),
    ExperimentalRidgeCaseRuntimeProjectionErrorV2,
> {
    let flat_case = flat_twin_case(case);
    let flat_request = runtime_request_components_v2(policy, vehicle, case, terrain(&flat_case))
        .map_err(map_case_runtime_error_v2)?;
    let mesa_request = runtime_request_components_v2(
        policy,
        vehicle,
        case,
        TerrainDefinition::Heightfield {
            points_m: mesa.terrain_points_m.clone(),
        },
    )
    .map_err(map_case_runtime_error_v2)?;
    Ok((
        project_case_runtime_outcome_v2(flat_candidate, &flat_request)?,
        project_case_runtime_outcome_v2(derived_mesa_candidate, &mesa_request)?,
    ))
}

fn project_case_runtime_outcome_v2(
    outcome: &ExperimentalRidgeCandidateOutcomeV2,
    request: &RoutePlanningRequest,
) -> Result<ExperimentalRidgeCaseRuntimeOutcomeV2, ExperimentalRidgeCaseRuntimeProjectionErrorV2> {
    match outcome {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => {
            let route =
                canonical_direct_runtime_route_v2(request).map_err(map_case_runtime_error_v2)?;
            let structural_validation = runtime_route_structure_v2(&route, RouteTopology::Direct)
                .map_err(map_case_runtime_error_v2)?;
            let mut runtime = ExperimentalRidgeCaseRuntimeOutcomeV2::Direct {
                candidate_identity: candidate.identity.clone(),
                route,
                structural_validation,
                identity: String::new(),
            };
            case_runtime_outcome_set_identity_v2(&mut runtime);
            Ok(runtime)
        }
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
            candidate,
            crossing,
        } => project_case_waypoint_runtime_v2(request, candidate, crossing),
        ExperimentalRidgeCandidateOutcomeV2::Unsupported {
            reason,
            rejection_reasons,
        } => {
            let mut runtime = ExperimentalRidgeCaseRuntimeOutcomeV2::Unsupported {
                reason: *reason,
                rejection_reasons: rejection_reasons.clone(),
                identity: String::new(),
            };
            case_runtime_outcome_set_identity_v2(&mut runtime);
            Ok(runtime)
        }
    }
}

fn project_case_waypoint_runtime_v2(
    request: &RoutePlanningRequest,
    candidate: &WaypointCandidateV2,
    crossing: &ExactIntermediateBridgeCrossingV2,
) -> Result<ExperimentalRidgeCaseRuntimeOutcomeV2, ExperimentalRidgeCaseRuntimeProjectionErrorV2> {
    // The first attempt is always the historical exact crossing.  It is
    // retained as a complete attempt even when its handoff envelope rejects
    // the state and the certified bridge exit is selected below.
    let primary_evidence =
        build_runtime_waypoint_evidence_case_v2(request, crossing.selected_state)
            .map_err(map_case_runtime_error_v2)?;
    let primary_attempt = runtime_handoff_attempt_v2(
        ExperimentalRidgeRuntimeHandoffSelectionKindV2::PrimaryCrossing,
        crossing.selected_applied_steps,
        None,
        primary_evidence,
    );
    let mut attempts = vec![primary_attempt];
    let selected_attempt_index = if attempts[0].handoff_assessment.contract_pass {
        0
    } else {
        let exit = verify_target_leg_acquisition_apex_seam_v2(candidate)?;
        let fallback_evidence = build_runtime_waypoint_evidence_case_v2(request, exit.state)
            .map_err(map_case_runtime_error_v2)?;
        let fallback_attempt = runtime_handoff_attempt_v2(
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::IntermediateBridgeExit,
            candidate
                .intermediate_bridge
                .as_ref()
                .expect("validated intermediate bridge")
                .steps,
            Some(exit.arc_step),
            fallback_evidence,
        );
        attempts.push(fallback_attempt);
        if !attempts[1].handoff_assessment.contract_pass {
            let mut failure = ExperimentalRidgeRuntimeHandoffFailureV2 {
                candidate_identity: candidate.identity.clone(),
                crossing: crossing.clone(),
                attempts,
                identity: String::new(),
            };
            failure.identity = runtime_handoff_failure_identity_v2(&failure);
            return Err(
                ExperimentalRidgeCaseRuntimeProjectionErrorV2::HandoffAttemptsFailed(Box::new(
                    failure,
                )),
            );
        }
        1
    };

    let mut handoff_selection = ExperimentalRidgeRuntimeHandoffSelectionV2 {
        candidate_identity: candidate.identity.clone(),
        crossing: crossing.clone(),
        attempts,
        selected_attempt_index,
        identity: String::new(),
    };
    handoff_selection.identity = runtime_handoff_selection_identity_v2(&handoff_selection);

    let Some(selected_attempt) = handoff_selection.selected_attempt() else {
        return Err(
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch,
        );
    };

    let mut runtime = ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
        candidate_identity: candidate.identity.clone(),
        route: selected_attempt.route.clone(),
        crossing: crossing.clone(),
        authority: selected_attempt.authority.clone(),
        handoff_kinematics: selected_attempt.handoff_kinematics,
        handoff_assessment: selected_attempt.handoff_assessment.clone(),
        structural_validation: selected_attempt.structural_validation.clone(),
        handoff_selection,
        identity: String::new(),
    };
    case_runtime_outcome_set_identity_v2(&mut runtime);
    Ok(runtime)
}

fn build_runtime_waypoint_evidence_case_v2(
    request: &RoutePlanningRequest,
    selected_state: KinematicStateV2,
) -> Result<RuntimeWaypointEvidenceCaseV2, ExperimentalRidgeRuntimeProjectionErrorV2> {
    let geometry = normalized_geometry(request)
        .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::WaypointRouteGeometryInvalid)?;
    let capture_radius_m = (geometry.direct_distance_m * 0.08).clamp(35.0, 95.0);
    let horizontal_sign = geometry.horizontal_sign;
    let source = request
        .source_pad()
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::InvalidRuntimeRequest)?;
    let normalized_waypoint = Vec2::new(
        f64::from(horizontal_sign) * (selected_state.position_m.x - source.center_x_m),
        selected_state.position_m.y,
    );
    let (profile, _) = build_endpoint_profile(request, geometry.direct_horizontal_span_m)
        .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::WaypointRouteGeometryInvalid)?;
    let shaped =
        endpoint_shaped_centerline(request, &geometry, &profile, &[normalized_waypoint])
            .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::WaypointRouteGeometryInvalid)?;
    let waypoint_index = shaped
        .iter()
        .position(|point| *point == normalized_waypoint)
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::WaypointRouteGeometryInvalid)?;
    if waypoint_index == 0 || waypoint_index + 1 >= shaped.len() {
        return Err(ExperimentalRidgeRuntimeProjectionErrorV2::WaypointRouteGeometryInvalid);
    }
    let inbound = runtime_unit_vector_v2(shaped[waypoint_index] - shaped[waypoint_index - 1])
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::WaypointTangentInvalid)?;
    let outbound = runtime_unit_vector_v2(shaped[waypoint_index + 1] - shaped[waypoint_index])
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::WaypointTangentInvalid)?;
    let normalized_tangent = runtime_unit_vector_v2(inbound + outbound)
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::WaypointTangentInvalid)?;
    let tangent = Vec2::new(
        f64::from(horizontal_sign) * normalized_tangent.x,
        normalized_tangent.y,
    );
    let preliminary = TransferWaypointSpec {
        id: "ridge-bridge-handoff-0".to_owned(),
        position_m: selected_state.position_m,
        handoff_tangent_unit: Some(tangent),
        capture_radius_m,
        max_cross_track_m: capture_radius_m,
        max_outbound_heading_error_rad: request.policy.max_outbound_heading_error_rad,
        min_outbound_progress_mps: request.policy.min_outbound_progress_mps,
        max_outbound_cross_speed_mps: Some(request.policy.max_outbound_cross_speed_mps),
        min_speed_mps: request.policy.min_handoff_speed_mps,
        max_speed_mps: request.policy.max_handoff_speed_mps,
        min_vertical_speed_mps: None,
        max_vertical_speed_mps: None,
    };
    let authority = compute_waypoint_authority(
        request,
        shaped[waypoint_index - 1],
        shaped[waypoint_index],
        shaped[waypoint_index + 1],
        capture_radius_m,
    )
    .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::WaypointAuthorityInvalid)?;
    let max_speed_mps = authority
        .handoff_speed_cap_mps
        .min(request.policy.max_handoff_speed_mps);
    if max_speed_mps + 1.0e-9 < request.policy.min_handoff_speed_mps {
        return Err(ExperimentalRidgeRuntimeProjectionErrorV2::WaypointAuthorityBelowMinimum);
    }
    let waypoint = TransferWaypointSpec {
        max_speed_mps,
        ..preliminary
    };
    let route = TransferRouteSpec {
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        route_angle_deg: geometry.route_angle_deg,
        route_radius_m: geometry.direct_distance_m,
        waypoints: vec![waypoint.clone()],
    };
    route
        .validate()
        .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::RouteStructuralValidationFailed)?;
    let handoff_kinematics =
        runtime_waypoint_handoff_kinematics_v2(request, &route.waypoints[0], selected_state)?;
    let handoff_assessment = route.waypoints[0].assess_handoff(handoff_kinematics);
    let structural_validation = runtime_route_structure_v2(&route, RouteTopology::Waypoint)?;
    Ok(RuntimeWaypointEvidenceCaseV2 {
        selected_state,
        route,
        authority,
        handoff_kinematics: handoff_kinematics.into(),
        handoff_assessment: runtime_waypoint_handoff_assessment_v2(&handoff_assessment),
        structural_validation,
    })
}

fn runtime_handoff_attempt_v2(
    selection_kind: ExperimentalRidgeRuntimeHandoffSelectionKindV2,
    applied_steps: u64,
    target_leg_arc_step: Option<u64>,
    evidence: RuntimeWaypointEvidenceCaseV2,
) -> ExperimentalRidgeRuntimeHandoffAttemptV2 {
    let mut attempt = ExperimentalRidgeRuntimeHandoffAttemptV2 {
        selection_kind,
        applied_steps,
        target_leg_arc_step,
        selected_state: evidence.selected_state,
        route: evidence.route,
        authority: evidence.authority,
        handoff_kinematics: evidence.handoff_kinematics,
        handoff_assessment: evidence.handoff_assessment,
        structural_validation: evidence.structural_validation,
        identity: String::new(),
    };
    attempt.identity = runtime_handoff_attempt_identity_v2(&attempt);
    attempt
}

fn verify_target_leg_acquisition_apex_seam_v2(
    candidate: &WaypointCandidateV2,
) -> Result<HandoffV2, ExperimentalRidgeCaseRuntimeProjectionErrorV2> {
    let bridge = candidate.intermediate_bridge.as_ref().ok_or(
        ExperimentalRidgeCaseRuntimeProjectionErrorV2::TargetLegAcquisitionApexSeamInvalid,
    )?;
    let exit = candidate.intermediate_exit_handoff.ok_or(
        ExperimentalRidgeCaseRuntimeProjectionErrorV2::TargetLegAcquisitionApexSeamInvalid,
    )?;
    if bridge.classification != CertificationV2::Certified
        || bridge.steps == 0
        || bridge.end_state != exit.state
        || exit.arc_step != candidate.target_leg.apex_step
        || exit.arc_step > candidate.target_leg.steps
    {
        return Err(
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::TargetLegAcquisitionApexSeamInvalid,
        );
    }
    let expected_apex_state = candidate
        .target_leg
        .state_at(candidate.target_leg.apex_step);
    if expected_apex_state != exit.state
        || expected_apex_state.position_m != candidate.target_leg.apex_position_m
    {
        return Err(
            ExperimentalRidgeCaseRuntimeProjectionErrorV2::TargetLegAcquisitionApexSeamInvalid,
        );
    }
    Ok(exit)
}

/// Shared runtime decision projection for the frozen embedded V2 canary and
/// the generic V1 input boundary.  Only the validated analytical outcomes
/// select a runtime route; no case label or embedded-report field participates.
fn project_ridge_runtime_outcomes_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    mesa: &MesaGeometryV2,
    flat_candidate: &ExperimentalRidgeCandidateOutcomeV2,
    derived_mesa_candidate: &ExperimentalRidgeCandidateOutcomeV2,
) -> Result<
    (
        ExperimentalRidgeRuntimeOutcomeV2,
        ExperimentalRidgeRuntimeOutcomeV2,
    ),
    ExperimentalRidgeRuntimeProjectionErrorV2,
> {
    let flat_case = flat_twin_case(case);
    let flat_request = runtime_request_components_v2(policy, vehicle, case, terrain(&flat_case))?;
    let mesa_request = runtime_request_components_v2(
        policy,
        vehicle,
        case,
        TerrainDefinition::Heightfield {
            points_m: mesa.terrain_points_m.clone(),
        },
    )?;
    Ok((
        project_runtime_outcome_v2(flat_candidate, &flat_request)?,
        project_runtime_outcome_v2(derived_mesa_candidate, &mesa_request)?,
    ))
}

fn project_runtime_outcome_v2(
    outcome: &ExperimentalRidgeCandidateOutcomeV2,
    request: &RoutePlanningRequest,
) -> Result<ExperimentalRidgeRuntimeOutcomeV2, ExperimentalRidgeRuntimeProjectionErrorV2> {
    match outcome {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => {
            let route = canonical_direct_runtime_route_v2(request)?;
            let structural_validation = runtime_route_structure_v2(&route, RouteTopology::Direct)?;
            let mut runtime = ExperimentalRidgeRuntimeOutcomeV2::Direct {
                candidate_identity: candidate.identity.clone(),
                route,
                structural_validation,
                identity: String::new(),
            };
            runtime_outcome_set_identity_v2(&mut runtime);
            Ok(runtime)
        }
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
            candidate,
            crossing,
        } => project_waypoint_runtime_v2(request, candidate, crossing),
        ExperimentalRidgeCandidateOutcomeV2::Unsupported {
            reason,
            rejection_reasons,
        } => {
            let mut runtime = ExperimentalRidgeRuntimeOutcomeV2::Unsupported {
                reason: *reason,
                rejection_reasons: rejection_reasons.clone(),
                identity: String::new(),
            };
            runtime_outcome_set_identity_v2(&mut runtime);
            Ok(runtime)
        }
    }
}

fn canonical_direct_runtime_route_v2(
    request: &RoutePlanningRequest,
) -> Result<TransferRouteSpec, ExperimentalRidgeRuntimeProjectionErrorV2> {
    let geometry = normalized_geometry(request)
        .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::InvalidRuntimeRequest)?;
    let route = TransferRouteSpec {
        source_pad_id: request.source_pad_id.clone(),
        target_pad_id: request.target_pad_id.clone(),
        route_angle_deg: geometry.route_angle_deg,
        route_radius_m: geometry.direct_distance_m,
        waypoints: Vec::new(),
    };
    route
        .validate()
        .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::RouteStructuralValidationFailed)?;
    Ok(route)
}

fn project_waypoint_runtime_v2(
    request: &RoutePlanningRequest,
    candidate: &WaypointCandidateV2,
    crossing: &ExactIntermediateBridgeCrossingV2,
) -> Result<ExperimentalRidgeRuntimeOutcomeV2, ExperimentalRidgeRuntimeProjectionErrorV2> {
    let evidence = build_runtime_waypoint_evidence_case_v2(request, crossing.selected_state)?;
    if !evidence.handoff_assessment.contract_pass {
        return Err(ExperimentalRidgeRuntimeProjectionErrorV2::HandoffContractFailed);
    }
    let mut runtime = ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint {
        candidate_identity: candidate.identity.clone(),
        route: evidence.route,
        crossing: crossing.clone(),
        authority: evidence.authority,
        handoff_kinematics: evidence.handoff_kinematics,
        handoff_assessment: evidence.handoff_assessment,
        structural_validation: evidence.structural_validation,
        identity: String::new(),
    };
    runtime_outcome_set_identity_v2(&mut runtime);
    Ok(runtime)
}

fn runtime_route_structure_v2(
    route: &TransferRouteSpec,
    topology: RouteTopology,
) -> Result<ExperimentalRidgeRuntimeRouteStructureV2, ExperimentalRidgeRuntimeProjectionErrorV2> {
    route
        .validate()
        .map_err(|_| ExperimentalRidgeRuntimeProjectionErrorV2::RouteStructuralValidationFailed)?;
    Ok(ExperimentalRidgeRuntimeRouteStructureV2 {
        topology,
        waypoint_count: route.waypoints.len(),
        transfer_route_valid: true,
    })
}

fn runtime_unit_vector_v2(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-12).then(|| vector * (1.0 / length))
}

fn runtime_waypoint_handoff_kinematics_v2(
    request: &RoutePlanningRequest,
    waypoint: &TransferWaypointSpec,
    state: KinematicStateV2,
) -> Result<WaypointHandoffKinematics, ExperimentalRidgeRuntimeProjectionErrorV2> {
    let source = request
        .source_pad()
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::InvalidRuntimeRequest)?;
    let target = request
        .target_pad()
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::InvalidRuntimeRequest)?;
    let anchor_m = Vec2::new(source.center_x_m, source.surface_y_m);
    let next_target_m = Vec2::new(target.center_x_m, target.surface_y_m);
    let leg_unit = runtime_unit_vector_v2(waypoint.position_m - anchor_m)
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::HandoffKinematicsInvalid)?;
    let handoff_tangent_unit = waypoint
        .handoff_tangent_unit
        .ok_or(ExperimentalRidgeRuntimeProjectionErrorV2::HandoffKinematicsInvalid)?;
    if runtime_unit_vector_v2(next_target_m - waypoint.position_m).is_none() {
        return Err(ExperimentalRidgeRuntimeProjectionErrorV2::HandoffKinematicsInvalid);
    }
    let to_waypoint_m = state.position_m - waypoint.position_m;
    let speed_mps = state.velocity_mps.length();
    let velocity_unit = if speed_mps > 1.0e-9 {
        state.velocity_mps * (1.0 / speed_mps)
    } else {
        Vec2::new(0.0, 0.0)
    };
    let dot = |lhs: Vec2, rhs: Vec2| lhs.x.mul_add(rhs.x, lhs.y * rhs.y);
    let cross = |lhs: Vec2, rhs: Vec2| lhs.x.mul_add(rhs.y, -(lhs.y * rhs.x));
    Ok(WaypointHandoffKinematics {
        distance_m: to_waypoint_m.length(),
        cross_track_m: cross(to_waypoint_m, leg_unit).abs(),
        plane_progress_m: dot(to_waypoint_m, leg_unit),
        outbound_heading_error_rad: if speed_mps > 1.0e-9 {
            dot(velocity_unit, handoff_tangent_unit)
                .clamp(-1.0, 1.0)
                .acos()
        } else {
            std::f64::consts::PI
        },
        outbound_progress_mps: dot(state.velocity_mps, handoff_tangent_unit),
        outbound_cross_speed_mps: cross(state.velocity_mps, handoff_tangent_unit).abs(),
        speed_mps,
        vertical_speed_mps: state.velocity_mps.y,
    })
}

fn runtime_waypoint_handoff_assessment_v2(
    assessment: &pd_core::WaypointHandoffAssessment,
) -> ExperimentalRidgeRuntimeWaypointHandoffAssessmentV2 {
    ExperimentalRidgeRuntimeWaypointHandoffAssessmentV2 {
        triggered: assessment.triggered,
        capture_window_open: assessment.capture_window_open,
        deadline_reached: assessment.deadline_reached,
        spatial_pass: assessment.spatial_pass,
        envelope_pass: assessment.envelope_pass,
        contract_pass: assessment.contract_pass(),
        violations: assessment
            .violations
            .iter()
            .map(|violation| violation.as_str().to_owned())
            .collect(),
    }
}

fn runtime_outcome_set_identity_v2(outcome: &mut ExperimentalRidgeRuntimeOutcomeV2) {
    let identity = runtime_outcome_identity_v2(outcome);
    match outcome {
        ExperimentalRidgeRuntimeOutcomeV2::Direct { identity: slot, .. }
        | ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint { identity: slot, .. }
        | ExperimentalRidgeRuntimeOutcomeV2::Unsupported { identity: slot, .. } => {
            *slot = identity;
        }
    }
}

fn runtime_outcome_identity_v2(outcome: &ExperimentalRidgeRuntimeOutcomeV2) -> String {
    match outcome {
        ExperimentalRidgeRuntimeOutcomeV2::Direct {
            candidate_identity,
            route,
            structural_validation,
            ..
        } => digest(&("direct", candidate_identity, route, structural_validation)),
        ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            route,
            crossing,
            authority,
            handoff_kinematics,
            handoff_assessment,
            structural_validation,
            ..
        } => digest(&(
            "one_waypoint",
            candidate_identity,
            route,
            crossing,
            authority,
            handoff_kinematics,
            handoff_assessment,
            structural_validation,
        )),
        ExperimentalRidgeRuntimeOutcomeV2::Unsupported {
            reason,
            rejection_reasons,
            ..
        } => digest(&("unsupported", reason, rejection_reasons)),
    }
}

fn runtime_projection_identity_v2(projection: &ExperimentalRidgeRuntimeProjectionV2) -> String {
    digest(&(
        projection.schema_id.as_str(),
        projection.schema_version,
        &projection.analytical_report_identity,
        &projection.analytical_canary_identity,
        &projection.policy,
        &projection.vehicle,
        &projection.case,
        &projection.mesa,
        &projection.flat_control,
        &projection.derived_mesa,
    ))
}

fn experimental_ridge_case_input_identity_v1(input: &ExperimentalRidgeCaseInputV1) -> String {
    digest(&ExperimentalRidgeCaseInputIdentityV1 {
        schema_id: &input.schema_id,
        schema_version: input.schema_version,
        policy: &input.policy,
        vehicle: &input.vehicle,
        probe: &input.probe,
    })
}

fn experimental_ridge_case_manifest_identity_v1(
    manifest: &ExperimentalRidgeCaseManifestV1,
) -> String {
    digest(&ExperimentalRidgeCaseManifestIdentityV1 {
        schema_id: &manifest.schema_id,
        schema_version: manifest.schema_version,
        cases: &manifest.cases,
    })
}

#[derive(Serialize)]
struct ExperimentalRidgeCaseManifestIdentityV1<'a> {
    schema_id: &'a str,
    schema_version: u32,
    cases: &'a [ExperimentalRidgeCaseInputV1],
}

#[derive(Serialize)]
struct ExperimentalRidgeCaseInputIdentityV1<'a> {
    schema_id: &'a str,
    schema_version: u32,
    policy: &'a DirectBridgePolicyV2,
    vehicle: &'a VehicleInputV2,
    probe: &'a DirectBridgeProbeV2,
}

fn experimental_ridge_case_projection_identity_v1(
    projection: &ExperimentalRidgeCaseProjectionV1,
) -> String {
    digest(&ExperimentalRidgeCaseProjectionIdentityV1 {
        schema_id: &projection.schema_id,
        schema_version: projection.schema_version,
        input_identity: &projection.input_identity,
        analytical_canary_identity: &projection.analytical_canary_identity,
        policy: &projection.policy,
        vehicle: &projection.vehicle,
        probe: &projection.probe,
        mesa: &projection.mesa,
        flat_candidate_identities: &projection.flat_candidate_identities,
        ridge_direct_candidate_identities: &projection.ridge_direct_candidate_identities,
        derived_nominal_direct: &projection.derived_nominal_direct,
        derived_non_nominal_direct_diagnostics: &projection.derived_non_nominal_direct_diagnostics,
        derived_blocker_valid: projection.derived_blocker_valid,
        waypoint_certified_candidate_count: projection.waypoint_certified_candidate_count,
        waypoint_search_identity: &projection.waypoint_search_identity,
        waypoint_candidate_identities: &projection.waypoint_candidate_identities,
        flat_control: &projection.flat_control,
        derived_mesa: &projection.derived_mesa,
    })
}

fn runtime_handoff_attempt_identity_v2(
    attempt: &ExperimentalRidgeRuntimeHandoffAttemptV2,
) -> String {
    digest(&(
        attempt.selection_kind,
        attempt.applied_steps,
        attempt.target_leg_arc_step,
        attempt.selected_state,
        &attempt.route,
        &attempt.authority,
        &attempt.handoff_kinematics,
        &attempt.handoff_assessment,
        &attempt.structural_validation,
    ))
}

fn runtime_handoff_selection_identity_v2(
    selection: &ExperimentalRidgeRuntimeHandoffSelectionV2,
) -> String {
    digest(&(
        selection.candidate_identity.as_str(),
        &selection.crossing,
        &selection.attempts,
        selection.selected_attempt_index,
    ))
}

fn runtime_handoff_failure_identity_v2(
    failure: &ExperimentalRidgeRuntimeHandoffFailureV2,
) -> String {
    digest(&(
        failure.candidate_identity.as_str(),
        &failure.crossing,
        &failure.attempts,
    ))
}

fn case_runtime_outcome_set_identity_v2(outcome: &mut ExperimentalRidgeCaseRuntimeOutcomeV2) {
    let identity = case_runtime_outcome_identity_v2(outcome);
    match outcome {
        ExperimentalRidgeCaseRuntimeOutcomeV2::Direct { identity: slot, .. }
        | ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint { identity: slot, .. }
        | ExperimentalRidgeCaseRuntimeOutcomeV2::Unsupported { identity: slot, .. } => {
            *slot = identity;
        }
    }
}

fn case_runtime_outcome_identity_v2(outcome: &ExperimentalRidgeCaseRuntimeOutcomeV2) -> String {
    match outcome {
        ExperimentalRidgeCaseRuntimeOutcomeV2::Direct {
            candidate_identity,
            route,
            structural_validation,
            ..
        } => digest(&("direct", candidate_identity, route, structural_validation)),
        ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            route,
            crossing,
            handoff_selection,
            authority,
            handoff_kinematics,
            handoff_assessment,
            structural_validation,
            ..
        } => digest(&(
            "one_waypoint",
            candidate_identity,
            route,
            crossing,
            handoff_selection,
            authority,
            handoff_kinematics,
            handoff_assessment,
            structural_validation,
        )),
        ExperimentalRidgeCaseRuntimeOutcomeV2::Unsupported {
            reason,
            rejection_reasons,
            ..
        } => digest(&("unsupported", reason, rejection_reasons)),
    }
}

fn experimental_ridge_case_runtime_projection_identity_v2(
    projection: &ExperimentalRidgeCaseRuntimeProjectionV2,
) -> String {
    digest(&(
        projection.schema_id.as_str(),
        projection.schema_version,
        projection.input_identity.as_str(),
        projection.analytical_canary_identity.as_str(),
        &projection.policy,
        &projection.vehicle,
        &projection.probe,
        &projection.mesa,
        &projection.flat_control,
        &projection.derived_mesa,
    ))
}

#[derive(Serialize)]
struct ExperimentalRidgeCaseProjectionIdentityV1<'a> {
    schema_id: &'a str,
    schema_version: u32,
    input_identity: &'a str,
    analytical_canary_identity: &'a str,
    policy: &'a DirectBridgePolicyV2,
    vehicle: &'a VehicleInputV2,
    probe: &'a DirectBridgeProbeV2,
    mesa: &'a MesaGeometryV2,
    flat_candidate_identities: &'a [String],
    ridge_direct_candidate_identities: &'a [String],
    derived_nominal_direct: &'a RidgeDirectDiagnosticV2,
    derived_non_nominal_direct_diagnostics: &'a [RidgeDirectDiagnosticV2],
    derived_blocker_valid: bool,
    waypoint_certified_candidate_count: usize,
    waypoint_search_identity: &'a str,
    waypoint_candidate_identities: &'a [String],
    flat_control: &'a ExperimentalRidgeCandidateOutcomeV2,
    derived_mesa: &'a ExperimentalRidgeCandidateOutcomeV2,
}

fn experimental_ridge_case_runtime_projection_identity_v1(
    projection: &ExperimentalRidgeCaseRuntimeProjectionV1,
) -> String {
    digest(&ExperimentalRidgeCaseRuntimeProjectionIdentityV1 {
        schema_id: &projection.schema_id,
        schema_version: projection.schema_version,
        input_identity: &projection.input_identity,
        analytical_canary_identity: &projection.analytical_canary_identity,
        policy: &projection.policy,
        vehicle: &projection.vehicle,
        probe: &projection.probe,
        mesa: &projection.mesa,
        flat_control: &projection.flat_control,
        derived_mesa: &projection.derived_mesa,
    })
}

#[derive(Serialize)]
struct ExperimentalRidgeCaseRuntimeProjectionIdentityV1<'a> {
    schema_id: &'a str,
    schema_version: u32,
    input_identity: &'a str,
    analytical_canary_identity: &'a str,
    policy: &'a DirectBridgePolicyV2,
    vehicle: &'a VehicleInputV2,
    probe: &'a DirectBridgeProbeV2,
    mesa: &'a MesaGeometryV2,
    flat_control: &'a ExperimentalRidgeRuntimeOutcomeV2,
    derived_mesa: &'a ExperimentalRidgeRuntimeOutcomeV2,
}

#[derive(Clone)]
struct ExperimentalRidgeProjectionComponentsV2 {
    policy: DirectBridgePolicyV2,
    vehicle: VehicleInputV2,
    case: DirectBridgeProbeV2,
    mesa: MesaGeometryV2,
    analytical_canary_identity: String,
    flat_candidate_identities: Vec<String>,
    ridge_direct_candidate_identities: Vec<String>,
    derived_nominal_direct: RidgeDirectDiagnosticV2,
    derived_non_nominal_direct_diagnostics: Vec<RidgeDirectDiagnosticV2>,
    derived_blocker_valid: bool,
    waypoint_certified_candidate_count: usize,
    waypoint_search_identity: String,
    waypoint_candidate_identities: Vec<String>,
    flat_control: ExperimentalRidgeCandidateOutcomeV2,
    derived_mesa: ExperimentalRidgeCandidateOutcomeV2,
}

/// The sole analytical decision constructor shared by the frozen embedded V2
/// API and the generic input-only V1 API.  It intentionally accepts a
/// recomputed canary rather than a report or caller-supplied outcome.
fn experimental_ridge_projection_components_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    canary: &RidgeCanaryEvidenceV2,
) -> Result<ExperimentalRidgeProjectionComponentsV2, ExperimentalRidgeCandidateErrorV2> {
    let nominal = canary.flat_control.nominal_candidate.clone();
    let flat_control = if canary.flat_control.status == MissionStatusV2::Green {
        ExperimentalRidgeCandidateOutcomeV2::Direct {
            candidate: Box::new(nominal.clone()),
        }
    } else {
        ExperimentalRidgeCandidateOutcomeV2::Unsupported {
            reason: ExperimentalRidgeUnsupportedReasonV2::NoCertifiedNominalDirect,
            rejection_reasons: nominal.reasons.clone(),
        }
    };
    let derived_mesa = project_derived_mesa_outcome_v2(policy, vehicle, case, canary)?;
    let ridge_direct_candidate_identities =
        std::iter::once(canary.direct_nominal_candidate.candidate_identity.clone())
            .chain(
                canary
                    .direct_global_replans
                    .iter()
                    .map(|candidate| candidate.candidate_identity.clone()),
            )
            .collect();
    Ok(ExperimentalRidgeProjectionComponentsV2 {
        policy: policy.clone(),
        vehicle: vehicle.clone(),
        case: case.clone(),
        mesa: canary.mesa.clone(),
        analytical_canary_identity: canary.identity.clone(),
        flat_candidate_identities: canary.flat_control.candidate_identities.clone(),
        ridge_direct_candidate_identities,
        derived_nominal_direct: canary.direct_nominal_candidate.clone(),
        derived_non_nominal_direct_diagnostics: canary.direct_global_replans.clone(),
        derived_blocker_valid: canary.blocking_lane_valid,
        waypoint_certified_candidate_count: canary.waypoint_search.certified_candidate_count,
        waypoint_search_identity: canary.waypoint_search.identity.clone(),
        waypoint_candidate_identities: canary
            .waypoint_search
            .candidates
            .iter()
            .map(|candidate| candidate.identity.clone())
            .collect(),
        flat_control,
        derived_mesa,
    })
}

fn experimental_ridge_projection_from_report_v2(
    report: &DirectBridgeReportV2,
) -> Result<ExperimentalRidgeCandidateProjectionV2, ExperimentalRidgeCandidateErrorV2> {
    let canary = &report.evaluation.ridge_canary;
    let case = report
        .fixture
        .cases
        .iter()
        .find(|case| case.id == canary.source_case_id)
        .ok_or(ExperimentalRidgeCandidateErrorV2::EmbeddedReportInvalid)?;
    let components = experimental_ridge_projection_components_v2(
        &report.fixture.policy,
        &report.fixture.vehicle,
        case,
        canary,
    )?;
    Ok(ExperimentalRidgeCandidateProjectionV2 {
        policy: components.policy,
        vehicle: components.vehicle,
        case: components.case,
        mesa: components.mesa,
        analytical_report_identity: report.identity.clone(),
        analytical_canary_identity: components.analytical_canary_identity,
        flat_candidate_identities: components.flat_candidate_identities,
        ridge_direct_candidate_identities: components.ridge_direct_candidate_identities,
        waypoint_search_identity: components.waypoint_search_identity,
        waypoint_candidate_identities: components.waypoint_candidate_identities,
        flat_control: components.flat_control,
        derived_mesa: components.derived_mesa,
    })
}

fn project_derived_mesa_outcome_v2(
    policy: &DirectBridgePolicyV2,
    vehicle: &VehicleInputV2,
    case: &DirectBridgeProbeV2,
    canary: &RidgeCanaryEvidenceV2,
) -> Result<ExperimentalRidgeCandidateOutcomeV2, ExperimentalRidgeCandidateErrorV2> {
    let derived_case = DirectBridgeProbeV2 {
        id: format!("{}_derived_mesa", case.id),
        source: case.source.clone(),
        target: case.target.clone(),
        terrain_points_m: canary.mesa.terrain_points_m.clone(),
        initial_position_m: case.initial_position_m,
        initial_velocity_mps: case.initial_velocity_mps,
    };
    let nominal_diagnostic = &canary.direct_nominal_candidate;
    let derived_nominal = evaluate_direct_candidate_with_duration_steps(
        policy,
        vehicle,
        &derived_case,
        nominal_diagnostic.duration_multiplier,
        nominal_diagnostic.arc_steps,
        &canary_bridge_duration_steps(policy),
    );
    if ridge_direct_diagnostic(&derived_nominal) != *nominal_diagnostic {
        return Err(ExperimentalRidgeCandidateErrorV2::DerivedNominalEvidenceMismatch);
    }
    if derived_nominal.classification == CertificationV2::Certified {
        return Ok(ExperimentalRidgeCandidateOutcomeV2::Direct {
            candidate: Box::new(derived_nominal),
        });
    }
    if !canary.blocking_lane_valid {
        return Ok(ExperimentalRidgeCandidateOutcomeV2::Unsupported {
            reason: ExperimentalRidgeUnsupportedReasonV2::NominalLaneNotRobustlyBlocked,
            rejection_reasons: canary.direct_nominal_candidate.reasons.clone(),
        });
    }
    let Some(candidate) = canary.waypoint_search.selected_candidate.clone() else {
        if canary.waypoint_search.selected_candidate_identity.is_some()
            || canary.waypoint_search.certified_candidate_count != 0
            || !canary.waypoint_search.candidates.is_empty()
        {
            return Err(ExperimentalRidgeCandidateErrorV2::WaypointCertificateMissing);
        }
        return Ok(ExperimentalRidgeCandidateOutcomeV2::Unsupported {
            reason: ExperimentalRidgeUnsupportedReasonV2::FiniteWaypointSearchExhausted,
            rejection_reasons: canary
                .waypoint_search
                .rejection_counts
                .iter()
                .map(|(reason, _)| *reason)
                .collect(),
        });
    };
    if canary
        .waypoint_search
        .selected_candidate_identity
        .as_deref()
        != Some(candidate.identity.as_str())
        || canary.waypoint_search.certified_candidate_count != 1
        || canary.waypoint_search.candidates.first() != Some(&candidate)
    {
        return Err(ExperimentalRidgeCandidateErrorV2::WaypointSelectionIdentityMismatch);
    }
    validate_waypoint_certificate_v2(policy, &candidate)?;
    let horizontal_sign = if case.target.center_x_m > case.source.center_x_m {
        1
    } else {
        -1
    };
    let crossing = select_exact_intermediate_bridge_crossing_v2(&candidate, horizontal_sign)?;
    Ok(ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
        candidate: Box::new(candidate),
        crossing: Box::new(crossing),
    })
}

fn validate_experimental_ridge_projection_structure_v2(
    projection: &ExperimentalRidgeCandidateProjectionV2,
) -> Result<(), ExperimentalRidgeCandidateErrorV2> {
    validate_experimental_ridge_projection_structure_components_v2(
        &projection.policy,
        &projection.case,
        &projection.flat_candidate_identities,
        &projection.waypoint_candidate_identities,
        &projection.flat_control,
        &projection.derived_mesa,
    )
}

fn validate_experimental_ridge_projection_structure_components_v2(
    policy: &DirectBridgePolicyV2,
    case: &DirectBridgeProbeV2,
    flat_candidate_identities: &[String],
    waypoint_candidate_identities: &[String],
    flat_control: &ExperimentalRidgeCandidateOutcomeV2,
    derived_mesa: &ExperimentalRidgeCandidateOutcomeV2,
) -> Result<(), ExperimentalRidgeCandidateErrorV2> {
    match flat_control {
        ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => {
            if candidate.identity != candidate_identity(candidate)
                || !flat_candidate_identities
                    .iter()
                    .any(|identity| identity == &candidate.identity)
            {
                return Err(ExperimentalRidgeCandidateErrorV2::FlatNominalIdentityMismatch);
            }
            if candidate.classification != CertificationV2::Certified {
                return Err(ExperimentalRidgeCandidateErrorV2::FlatNominalCertificateInvalid);
            }
        }
        ExperimentalRidgeCandidateOutcomeV2::Unsupported { .. } => {}
        ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { .. } => {
            return Err(ExperimentalRidgeCandidateErrorV2::FlatNominalCertificateInvalid);
        }
    }
    if let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
        candidate,
        crossing,
    } = derived_mesa
    {
        if waypoint_candidate_identities.first() != Some(&candidate.identity) {
            return Err(ExperimentalRidgeCandidateErrorV2::WaypointSelectionIdentityMismatch);
        }
        validate_waypoint_certificate_v2(policy, candidate)?;
        let horizontal_sign = if case.target.center_x_m > case.source.center_x_m {
            1
        } else {
            -1
        };
        let expected = select_exact_intermediate_bridge_crossing_v2(candidate, horizontal_sign)?;
        if **crossing != expected {
            return Err(ExperimentalRidgeCandidateErrorV2::IntermediateCrossingMismatch);
        }
    }
    Ok(())
}

fn validate_waypoint_certificate_v2(
    policy: &DirectBridgePolicyV2,
    candidate: &WaypointCandidateV2,
) -> Result<(), ExperimentalRidgeCandidateErrorV2> {
    let intermediate_bridge = candidate
        .intermediate_bridge
        .as_ref()
        .ok_or(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeMissing)?;
    let intermediate_entry = candidate
        .intermediate_entry_handoff
        .ok_or(ExperimentalRidgeCandidateErrorV2::IntermediateHandoffMissing)?;
    let intermediate_exit = candidate
        .intermediate_exit_handoff
        .ok_or(ExperimentalRidgeCandidateErrorV2::IntermediateHandoffMissing)?;
    if intermediate_bridge.start_state != intermediate_entry.state
        || intermediate_bridge.end_state != intermediate_exit.state
    {
        return Err(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeJoinMismatch);
    }
    let bridges = [
        candidate.source_bridge.as_ref(),
        Some(intermediate_bridge),
        candidate.terminal_bridge.as_ref(),
    ];
    let environments = [
        candidate.source_environment.as_ref(),
        candidate.intermediate_environment.as_ref(),
        candidate.terminal_environment.as_ref(),
    ];
    if candidate.source_handoff.is_none()
        || candidate.terminal_handoff.is_none()
        || bridges.iter().any(Option::is_none)
        || environments.iter().any(Option::is_none)
        || candidate.route_progress.is_none()
    {
        return Err(ExperimentalRidgeCandidateErrorV2::WaypointCertificateMissing);
    }
    if candidate.identity != waypoint_candidate_identity(candidate)
        || bridges.into_iter().flatten().any(|bridge| {
            bridge.identity
                != digest(&BridgeIdentity {
                    kind: bridge.kind,
                    start_state: bridge.start_state,
                    end_state: bridge.end_state,
                    steps: bridge.steps,
                    initial_net_acceleration_mps2: bridge.initial_net_acceleration_mps2,
                    net_acceleration_step_mps2: bridge.net_acceleration_step_mps2,
                    endpoint_position_error_m: bridge.endpoint_position_error_m,
                    endpoint_velocity_error_mps: bridge.endpoint_velocity_error_mps,
                    fuel_burn_kg: bridge.fuel_burn_kg,
                    classification: bridge.classification,
                    reasons: &bridge.reasons,
                    margins: bridge.margins,
                })
        })
        || candidate.classification != CertificationV2::Certified
        || bridges
            .into_iter()
            .flatten()
            .any(|bridge| bridge.classification != CertificationV2::Certified)
        || environments.into_iter().flatten().any(|environment| {
            [
                environment.clearance_margin,
                environment.initial_attitude_margin,
                environment.final_attitude_margin,
                environment.final_angular_rate_margin,
            ]
            .into_iter()
            .any(|margin| !margin.passes(policy))
        })
        || candidate
            .route_progress
            .as_ref()
            .is_none_or(|progress| !progress.passes || progress.segments.iter().any(|s| !s.passes))
        || candidate.margins.minimum_normalized() + 1.0e-12 < policy.declared_robustness_margin
    {
        return Err(ExperimentalRidgeCandidateErrorV2::WaypointCertificateInvalid);
    }
    Ok(())
}

/// Select the first exact intermediate-bridge state whose directed x reaches
/// or passes the virtual-anchor plane.  This is analytical certificate
/// projection only; no runtime waypoint policy is applied here.
fn select_exact_intermediate_bridge_crossing_v2(
    candidate: &WaypointCandidateV2,
    horizontal_sign: i8,
) -> Result<ExactIntermediateBridgeCrossingV2, ExperimentalRidgeCandidateErrorV2> {
    let bridge = candidate
        .intermediate_bridge
        .as_ref()
        .ok_or(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeMissing)?;
    let intermediate_entry_handoff = candidate
        .intermediate_entry_handoff
        .ok_or(ExperimentalRidgeCandidateErrorV2::IntermediateHandoffMissing)?;
    let intermediate_exit_handoff = candidate
        .intermediate_exit_handoff
        .ok_or(ExperimentalRidgeCandidateErrorV2::IntermediateHandoffMissing)?;
    if bridge.start_state != intermediate_entry_handoff.state
        || bridge.end_state != intermediate_exit_handoff.state
    {
        return Err(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeJoinMismatch);
    }
    if bridge.steps == 0 {
        return Err(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeHasNoSteps);
    }
    let directed_offset = |state: KinematicStateV2| {
        f64::from(horizontal_sign) * (state.position_m.x - candidate.waypoint_position_m.x)
    };
    if directed_offset(bridge.state_at(0)) >= 0.0 {
        return Err(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeStartsAtOrBeyondAnchor);
    }
    for selected_applied_steps in 1..=bridge.steps {
        let previous_applied_steps = selected_applied_steps - 1;
        let previous_state = bridge.state_at(previous_applied_steps);
        let selected_state = bridge.state_at(selected_applied_steps);
        let previous_directed_offset_m = directed_offset(previous_state);
        let selected_directed_offset_m = directed_offset(selected_state);
        if previous_directed_offset_m < 0.0 && selected_directed_offset_m >= 0.0 {
            return Ok(ExactIntermediateBridgeCrossingV2 {
                selection_rule: "first exact intermediate-bridge state whose directed x reaches or passes the virtual-anchor x; retain the preceding discrete state as the crossing bracket".to_owned(),
                candidate_identity: candidate.identity.clone(),
                intermediate_bridge_identity: bridge.identity.clone(),
                virtual_anchor_m: candidate.waypoint_position_m,
                intermediate_entry_handoff,
                intermediate_exit_handoff,
                intermediate_bridge_total_applied_steps: bridge.steps,
                certified_prefix_end_applied_steps: selected_applied_steps,
                certified_suffix_start_applied_steps: selected_applied_steps,
                previous_applied_steps,
                selected_applied_steps,
                previous_state,
                selected_state,
                previous_directed_offset_m,
                selected_directed_offset_m,
                strict_directed_crossing: true,
            });
        }
    }
    Err(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeDoesNotCrossAnchor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn fixture() -> DirectBridgeFixtureV2 {
        load_embedded_fixture_v2()
    }

    fn evaluation() -> DirectBridgeEvaluationV2 {
        static EVALUATION: OnceLock<DirectBridgeEvaluationV2> = OnceLock::new();
        EVALUATION.get_or_init(evaluate_embedded_fixture_v2).clone()
    }

    fn report() -> DirectBridgeReportV2 {
        // Share the validation canonical report so report projection tests do
        // not pay for two equivalent full analytical evaluations.
        embedded_report_artifact_v2().clone()
    }

    fn generic_ridge_input() -> ExperimentalRidgeCaseInputV1 {
        let fixture = fixture();
        let mut probe = fixture
            .cases
            .into_iter()
            .find(|case| case.id == "ridge_probe")
            .expect("embedded ridge probe");
        // Exercise the generic boundary with already-seen geometry while
        // proving that it has no embedded case-ID selection behavior.
        probe.id = "ridge_probe_generic_input".to_owned();
        ExperimentalRidgeCaseInputV1::new(fixture.policy, fixture.vehicle, probe)
            .expect("renamed embedded ridge input validates")
    }

    fn heldout_ridge_input(case_id: &str) -> ExperimentalRidgeCaseInputV1 {
        load_experimental_ridge_case_manifest_v1()
            .cases
            .into_iter()
            .find(|input| input.probe.id == case_id)
            .unwrap_or_else(|| panic!("held-out ridge input {case_id}"))
    }

    fn clear_result(evaluation: &DirectBridgeEvaluationV2) -> &DirectBridgeProbeResultV2 {
        evaluation
            .results
            .iter()
            .find(|result| result.id == "clear_direct_probe")
            .expect("embedded clear probe result")
    }

    #[test]
    fn fixture_contract_parses_and_intervals_are_exact_ticks() {
        let fixture = fixture();
        assert_eq!(fixture.policy.handoff_interval_ticks(), 30);
        assert_eq!(fixture.policy.bridge_interval_ticks(), 60);
        assert_eq!(fixture.cases.len(), 4);
    }

    #[test]
    fn heldout_input_manifest_loads_with_frozen_order_and_geometry() {
        let manifest = load_experimental_ridge_case_manifest_v1();
        manifest.validate().unwrap();
        assert_eq!(
            manifest
                .cases
                .iter()
                .map(|case| case.probe.id.as_str())
                .collect::<Vec<_>>(),
            EXPERIMENTAL_RIDGE_HELDOUT_CASE_IDS_V1
        );
        assert_eq!(manifest.cases.len(), 2);
        assert_eq!(manifest.cases[0].probe.terrain_points_m[1].x, 1884.0);
        assert_eq!(manifest.cases[1].probe.terrain_points_m[1].x, 2600.76);
        assert_eq!(manifest.cases[0].probe.terrain_points_m[2].y, 1200.0);
        assert_eq!(manifest.cases[1].probe.terrain_points_m[3].y, 1200.0);
        assert_eq!(manifest.cases[0].probe.source.center_x_m, 18.0);
        assert_eq!(manifest.cases[0].probe.target.center_x_m, 4000.0);
        assert_eq!(
            manifest.cases[0].probe.initial_position_m,
            Vec2::new(18.0, 5.0)
        );
        assert_eq!(
            manifest.cases[0].probe.initial_velocity_mps,
            Vec2::new(0.0, 0.0)
        );
        assert_eq!(manifest.cases[0].policy, manifest.cases[1].policy);
        assert_eq!(manifest.cases[0].vehicle, manifest.cases[1].vehicle);
        assert_eq!(manifest.cases[0].identity, "fnv1a64:1906ea1c9d5b51eb");
        assert_eq!(manifest.cases[1].identity, "fnv1a64:a63ff1716b1cf24f");
        assert_eq!(manifest.identity, "fnv1a64:19217b5811b4f25f");
        let reloaded: ExperimentalRidgeCaseManifestV1 =
            serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
        assert_eq!(reloaded, manifest);
        reloaded.validate().unwrap();
    }

    #[test]
    fn heldout_input_manifest_rejects_tamper_and_reordering() {
        let manifest = load_experimental_ridge_case_manifest_v1();

        let mut identity_tampered = manifest.clone();
        identity_tampered.identity.push_str("-tampered");
        assert_eq!(
            identity_tampered.validate(),
            Err(ExperimentalRidgeCaseManifestErrorV1::IdentityMismatch)
        );

        let mut input_tampered = manifest.clone();
        input_tampered.cases[0].probe.terrain_points_m[1].x += 1.0;
        assert_eq!(
            input_tampered.validate(),
            Err(ExperimentalRidgeCaseManifestErrorV1::InputInvalid)
        );

        let mut reordered = manifest;
        reordered.cases.swap(0, 1);
        assert_eq!(
            reordered.validate(),
            Err(ExperimentalRidgeCaseManifestErrorV1::CaseOrderMismatch)
        );

        let mut extra = load_experimental_ridge_case_manifest_v1();
        extra.cases.push(extra.cases[0].clone());
        assert_eq!(
            extra.validate(),
            Err(ExperimentalRidgeCaseManifestErrorV1::CaseCountMismatch)
        );
    }

    #[test]
    fn fixture_rejects_nonflat_terrain_inside_a_declared_pad() {
        let mut fixture = fixture();
        fixture.cases[0].terrain_points_m = vec![
            Vec2::new(-40.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(18.0, 1.0),
            Vec2::new(36.0, 0.0),
            Vec2::new(2540.0, 0.0),
        ];
        let error = fixture.validate().unwrap_err();
        assert!(error.contains("source pad terrain footprint must be flat"));
    }

    #[test]
    fn robustness_normalization_is_dimensionless_for_each_component_scale() {
        let upper = MarginV2::upper(0.15, 0.075);
        let lower = MarginV2::lower(0.325, 0.25);
        let clearance = MarginV2::lower_scaled(0.375, 0.0, 5.0);
        assert!((upper.normalized - 0.5).abs() <= f64::EPSILON);
        assert!((lower.normalized - 0.3).abs() <= f64::EPSILON);
        assert!((clearance.normalized - 0.075).abs() <= f64::EPSILON);
    }

    #[test]
    fn report_artifact_has_the_frozen_schema_and_neutral_probe_ids() {
        let report = report();
        assert_eq!(report.schema_id, REPORT_SCHEMA_ID_V2);
        assert_eq!(report.schema_version, REPORT_SCHEMA_VERSION_V2);
        assert_eq!(
            report
                .fixture
                .cases
                .iter()
                .map(|case| case.id.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from([
                "clear_direct_probe",
                "long_range_probe",
                "long_span_probe",
                "ridge_probe",
            ])
        );
    }

    #[test]
    fn report_artifact_serialization_identity_and_roundtrip_are_deterministic() {
        let report = report();
        assert_eq!(report.identity, report_identity_v2(&report));
        assert!(report.evaluation.results.iter().all(|result| {
            result.candidates.iter().all(|candidate| {
                candidate
                    .source_bridge
                    .as_ref()
                    .is_none_or(|bridge| bridge.samples.is_empty())
                    && candidate
                        .terminal_bridge
                        .as_ref()
                        .is_none_or(|bridge| bridge.samples.is_empty())
            })
        }));
        let first = serde_json::to_vec(&report).unwrap();
        let second = serde_json::to_vec(&report).unwrap();
        assert_eq!(first, second);
        assert!(
            first
                .windows(b"\"samples\"".len())
                .any(|window| window == b"\"samples\"")
        );
        assert!(
            report
                .evaluation
                .ridge_canary
                .waypoint_search
                .selected_candidate
                .as_ref()
                .is_some_and(|candidate| {
                    [
                        candidate.source_bridge.as_ref(),
                        candidate.intermediate_bridge.as_ref(),
                        candidate.terminal_bridge.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .all(|bridge| bridge.samples.is_empty())
                })
        );
        let roundtrip: DirectBridgeReportV2 = serde_json::from_slice(&first).unwrap();
        assert_eq!(roundtrip, report);
        validate_report_artifact_v2(&roundtrip).unwrap();
    }

    #[test]
    fn report_artifact_rejects_reorder_and_nested_tampering() {
        let report = report();

        let mut schema_tampered = report.clone();
        schema_tampered.schema_id.push_str("-tampered");
        assert!(validate_report_artifact_v2(&schema_tampered).is_err());

        let mut version_tampered = report.clone();
        version_tampered.schema_version += 1;
        assert!(validate_report_artifact_v2(&version_tampered).is_err());

        let mut fixture_reordered = report.clone();
        fixture_reordered.fixture.cases.swap(0, 1);
        assert!(validate_report_artifact_v2(&fixture_reordered).is_err());

        let mut fixture_tampered = report.clone();
        fixture_tampered.fixture.policy.minimum_clearance_m += 1.0;
        assert!(validate_report_artifact_v2(&fixture_tampered).is_err());

        let mut evaluation_reordered = report.clone();
        evaluation_reordered.evaluation.results.swap(0, 1);
        assert!(validate_report_artifact_v2(&evaluation_reordered).is_err());

        let mut selection_tampered = report.clone();
        selection_tampered.evaluation.results[0]
            .selected_candidate_identity
            .push_str("-tampered");
        assert!(validate_report_artifact_v2(&selection_tampered).is_err());

        let mut counter_tampered = report.clone();
        counter_tampered.evaluation.results[0].candidates[0].source_bridge_attempt_count += 1;
        assert!(validate_report_artifact_v2(&counter_tampered).is_err());

        let mut coefficient_tampered = report.clone();
        let candidate = coefficient_tampered
            .evaluation
            .results
            .iter_mut()
            .flat_map(|result| result.candidates.iter_mut())
            .find(|candidate| candidate.source_bridge.is_some())
            .expect("frozen report has selected source bridge evidence");
        candidate
            .source_bridge
            .as_mut()
            .unwrap()
            .initial_net_acceleration_mps2
            .y += 1.0;
        assert!(validate_report_artifact_v2(&coefficient_tampered).is_err());

        let mut canary_tampered = report.clone();
        canary_tampered.evaluation.ridge_canary.mesa.top_y_m += 1.0;
        assert!(validate_report_artifact_v2(&canary_tampered).is_err());

        let mut waypoint_tampered = report.clone();
        waypoint_tampered
            .evaluation
            .ridge_canary
            .waypoint_search
            .selected_candidate
            .as_mut()
            .expect("frozen report has a waypoint witness")
            .waypoint_position_m
            .x += 1.0;
        assert!(validate_report_artifact_v2(&waypoint_tampered).is_err());

        let mut progress_tampered = report.clone();
        progress_tampered
            .evaluation
            .ridge_canary
            .waypoint_search
            .selected_candidate
            .as_mut()
            .and_then(|candidate| candidate.route_progress.as_mut())
            .expect("frozen report has route-progress evidence")
            .total_backtracking_distance_m += 1.0;
        assert!(validate_report_artifact_v2(&progress_tampered).is_err());

        let mut identity_tampered = report;
        identity_tampered.identity.push_str("-tampered");
        assert!(validate_report_artifact_v2(&identity_tampered).is_err());
    }

    #[test]
    fn experimental_candidate_projection_passes_through_exact_flat_nominal() {
        let report = report();
        let projection = evaluate_embedded_experimental_ridge_candidate_v2().unwrap();
        let candidate = match &projection.flat_control {
            ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } => candidate,
            outcome => panic!("expected direct flat control, got {outcome:?}"),
        };
        assert_eq!(
            candidate.as_ref(),
            &report
                .evaluation
                .ridge_canary
                .flat_control
                .nominal_candidate
        );
        assert_eq!(
            projection.flat_candidate_identities,
            report
                .evaluation
                .ridge_canary
                .flat_control
                .candidate_identities
        );
        validate_embedded_experimental_ridge_candidate_v2(&projection).unwrap();
    }

    #[test]
    fn experimental_ridge_repair_carries_first_exact_bridge_crossing() {
        let projection = evaluate_embedded_experimental_ridge_candidate_v2().unwrap();
        assert!(
            evaluation()
                .ridge_canary
                .direct_global_replans
                .iter()
                .any(|candidate| candidate.classification == CertificationV2::Certified),
            "a higher-duration global direct replan must remain diagnostic and not displace repair"
        );
        let (candidate, crossing) = match &projection.derived_mesa {
            ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
                candidate,
                crossing,
            } => (candidate, crossing),
            outcome => panic!("expected one-waypoint ridge repair, got {outcome:?}"),
        };
        let bridge = candidate
            .intermediate_bridge
            .as_ref()
            .expect("certified intermediate bridge");
        assert_eq!(crossing.previous_applied_steps, 971);
        assert_eq!(crossing.selected_applied_steps, 972);
        assert_eq!(crossing.previous_state, bridge.state_at(971));
        assert_eq!(crossing.selected_state, bridge.state_at(972));
        assert!(crossing.previous_directed_offset_m < 0.0);
        assert!(crossing.selected_directed_offset_m >= 0.0);
        assert_ne!(
            crossing.selected_state.position_m,
            candidate.waypoint_position_m
        );
        assert!(
            (0..crossing.previous_applied_steps).all(|step| {
                bridge.state_at(step).position_m.x < candidate.waypoint_position_m.x
            })
        );
        let mut outside_bridge = candidate.as_ref().clone();
        outside_bridge.waypoint_position_m.x = bridge.end_state.position_m.x + 1.0;
        assert_eq!(
            select_exact_intermediate_bridge_crossing_v2(&outside_bridge, 1),
            Err(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeDoesNotCrossAnchor)
        );
    }

    #[test]
    fn experimental_candidate_projection_is_repeatable_with_stable_ordering() {
        let first = evaluate_embedded_experimental_ridge_candidate_v2().unwrap();
        let second = evaluate_embedded_experimental_ridge_candidate_v2().unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.flat_candidate_identities,
            second.flat_candidate_identities
        );
        assert_eq!(
            first.ridge_direct_candidate_identities,
            second.ridge_direct_candidate_identities
        );
        assert_eq!(
            first.waypoint_candidate_identities,
            second.waypoint_candidate_identities
        );
        assert_eq!(first.analytical_report_identity, report().identity);
    }

    #[test]
    fn experimental_derived_decision_uses_exact_nominal_certificate_predicates() {
        let fixture = fixture();
        let ridge_case = fixture
            .cases
            .iter()
            .find(|case| case.id == "ridge_probe")
            .unwrap();
        let mut invalid_blocking = evaluation().ridge_canary;
        assert_eq!(
            invalid_blocking.direct_nominal_candidate.classification,
            CertificationV2::NotCertified
        );
        invalid_blocking.blocking_lane_valid = false;
        invalid_blocking.direct_status = MissionStatusV2::Green;
        let unsupported = project_derived_mesa_outcome_v2(
            &fixture.policy,
            &fixture.vehicle,
            ridge_case,
            &invalid_blocking,
        )
        .unwrap();
        assert!(matches!(
            unsupported,
            ExperimentalRidgeCandidateOutcomeV2::Unsupported {
                reason: ExperimentalRidgeUnsupportedReasonV2::NominalLaneNotRobustlyBlocked,
                ..
            }
        ));

        let mut flat_case = ridge_case.clone();
        flat_case.terrain_points_m = vec![Vec2::new(-40.0, 0.0), Vec2::new(4040.0, 0.0)];
        let flat_canary =
            evaluate_ridge_canary_case_v2(&fixture.policy, &fixture.vehicle, &flat_case).unwrap();
        let direct = project_derived_mesa_outcome_v2(
            &fixture.policy,
            &fixture.vehicle,
            &flat_case,
            &flat_canary,
        )
        .unwrap();
        let ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } = direct else {
            panic!("exact certified derived nominal must produce Direct");
        };
        assert_eq!(candidate.classification, CertificationV2::Certified);
        assert_eq!(
            candidate.identity,
            flat_canary.direct_nominal_candidate.candidate_identity
        );
        assert_eq!(
            ridge_direct_diagnostic(&candidate),
            flat_canary.direct_nominal_candidate
        );
    }

    #[test]
    fn experimental_candidate_projection_fails_closed_but_keeps_exhaustion_typed() {
        let projection = evaluate_embedded_experimental_ridge_candidate_v2().unwrap();

        let mut missing_certificate = projection.clone();
        let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } =
            &mut missing_certificate.derived_mesa
        else {
            panic!("expected waypoint candidate");
        };
        candidate.source_bridge = None;
        assert_eq!(
            validate_embedded_experimental_ridge_candidate_v2(&missing_certificate),
            Err(ExperimentalRidgeCandidateErrorV2::WaypointCertificateMissing)
        );

        let mut tampered_certificate = projection.clone();
        let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } =
            &mut tampered_certificate.derived_mesa
        else {
            panic!("expected waypoint candidate");
        };
        candidate.classification = CertificationV2::NotCertified;
        assert_eq!(
            validate_embedded_experimental_ridge_candidate_v2(&tampered_certificate),
            Err(ExperimentalRidgeCandidateErrorV2::WaypointCertificateInvalid)
        );

        let mut broken_join = projection.clone();
        let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } =
            &mut broken_join.derived_mesa
        else {
            panic!("expected waypoint candidate");
        };
        candidate
            .intermediate_bridge
            .as_mut()
            .expect("intermediate bridge")
            .end_state
            .position_m
            .x += 1.0;
        assert_eq!(
            validate_embedded_experimental_ridge_candidate_v2(&broken_join),
            Err(ExperimentalRidgeCandidateErrorV2::IntermediateBridgeJoinMismatch)
        );

        let mut broken_crossing = projection.clone();
        let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { crossing, .. } =
            &mut broken_crossing.derived_mesa
        else {
            panic!("expected waypoint candidate");
        };
        crossing.selected_applied_steps += 1;
        assert_eq!(
            validate_embedded_experimental_ridge_candidate_v2(&broken_crossing),
            Err(ExperimentalRidgeCandidateErrorV2::IntermediateCrossingMismatch)
        );

        let fixture = fixture();
        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == "ridge_probe")
            .unwrap();
        let mut exhausted = evaluation().ridge_canary;
        exhausted.waypoint_search.selected_candidate = None;
        exhausted.waypoint_search.selected_candidate_identity = None;
        exhausted.waypoint_search.certified_candidate_count = 0;
        exhausted.waypoint_search.candidates.clear();
        let outcome =
            project_derived_mesa_outcome_v2(&fixture.policy, &fixture.vehicle, case, &exhausted)
                .unwrap();
        assert!(matches!(
            outcome,
            ExperimentalRidgeCandidateOutcomeV2::Unsupported {
                reason: ExperimentalRidgeUnsupportedReasonV2::FiniteWaypointSearchExhausted,
                ..
            }
        ));

        let mut malformed_exhaustion = exhausted;
        malformed_exhaustion
            .waypoint_search
            .selected_candidate_identity = Some("missing-certificate".to_owned());
        assert_eq!(
            project_derived_mesa_outcome_v2(
                &fixture.policy,
                &fixture.vehicle,
                case,
                &malformed_exhaustion,
            ),
            Err(ExperimentalRidgeCandidateErrorV2::WaypointCertificateMissing)
        );
    }

    #[test]
    fn exact_bridge_reproduces_both_endpoint_components() {
        let fixture = fixture();
        let bridge = exact_discrete_bridge_v2(
            &fixture.policy,
            &fixture.vehicle,
            BridgeKindV2::Source,
            KinematicStateV2 {
                position_m: Vec2::new(12.0, 8.0),
                velocity_mps: Vec2::new(3.0, -1.0),
            },
            KinematicStateV2 {
                position_m: Vec2::new(260.0, 180.0),
                velocity_mps: Vec2::new(25.0, 5.0),
            },
            240,
        )
        .unwrap();
        assert!(bridge.endpoint_position_error_m <= ENDPOINT_TOLERANCE);
        assert!(bridge.endpoint_velocity_error_mps <= ENDPOINT_TOLERANCE);
        let final_state = bridge.samples.last().unwrap().state_m;
        assert_eq!(bridge.state_at(bridge.steps), final_state);
        assert!(
            distance(final_state.position_m, bridge.end_state.position_m) <= ENDPOINT_TOLERANCE
        );
        assert!(
            distance(final_state.velocity_mps, bridge.end_state.velocity_mps) <= ENDPOINT_TOLERANCE
        );
        assert!(
            !bridge
                .reasons
                .contains(&DirectBridgeReasonV2::BridgeEndpoint)
        );

        let benign = exact_discrete_bridge_v2(
            &fixture.policy,
            &fixture.vehicle,
            BridgeKindV2::Source,
            KinematicStateV2 {
                position_m: Vec2::new(18.0, 10.0),
                velocity_mps: Vec2::new(0.0, 0.0),
            },
            KinematicStateV2 {
                position_m: Vec2::new(18.0, 10.0),
                velocity_mps: Vec2::new(0.0, 0.0),
            },
            120,
        )
        .unwrap();
        assert_eq!(benign.classification, CertificationV2::Certified);
    }

    #[test]
    fn compact_bridge_assessment_matches_materialized_trace() {
        let fixture = fixture();
        let case = &fixture.cases[0];
        let terrain = terrain(case);
        let validated_terrain = ValidatedHeightfieldV2::new(&terrain).unwrap();
        let start_state = KinematicStateV2 {
            position_m: Vec2::new(18.0, 50.0),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let end_state = KinematicStateV2 {
            position_m: Vec2::new(18.0, 50.0),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let mut attempts = AttemptSummaryV2::default();
        let compact = compact_bridge_assessment(
            &fixture.policy,
            &fixture.vehicle,
            &validated_terrain,
            BridgeKindV2::Source,
            start_state,
            end_state,
            240,
            Some(&case.source),
            &mut attempts,
        )
        .expect("high bridge clears compact terrain screening");
        let materialized = exact_discrete_bridge_v2(
            &fixture.policy,
            &fixture.vehicle,
            BridgeKindV2::Source,
            start_state,
            end_state,
            240,
        )
        .unwrap();
        let environment = screen_bridge_environment(
            &terrain,
            &materialized,
            Some(&case.source),
            &fixture.policy,
            &fixture.vehicle,
        );
        assert_compact_bridge_matches(&compact, &materialized, &environment);
    }

    #[test]
    fn closed_form_bridge_stays_exact_and_compact_at_long_horizon() {
        let fixture = fixture();
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-1_000.0, 0.0), Vec2::new(1_000.0, 0.0)],
        };
        let validated_terrain = ValidatedHeightfieldV2::new(&terrain).unwrap();
        let start_state = KinematicStateV2 {
            position_m: Vec2::new(0.0, 200.0),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let end_state = KinematicStateV2 {
            position_m: Vec2::new(300.0, 200.0),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let pad = PadInputV2 {
            center_x_m: 0.0,
            surface_y_m: 0.0,
            width_m: 36.0,
        };
        let materialized = exact_discrete_bridge_v2(
            &fixture.policy,
            &fixture.vehicle,
            BridgeKindV2::Source,
            start_state,
            end_state,
            6_000,
        )
        .unwrap();
        assert!(materialized.endpoint_position_error_m <= ENDPOINT_TOLERANCE);
        assert!(materialized.endpoint_velocity_error_mps <= ENDPOINT_TOLERANCE);

        let mut attempts = AttemptSummaryV2::default();
        let compact = compact_bridge_assessment(
            &fixture.policy,
            &fixture.vehicle,
            &validated_terrain,
            BridgeKindV2::Source,
            start_state,
            end_state,
            6_000,
            Some(&pad),
            &mut attempts,
        )
        .expect("long-horizon bridge clears compact terrain screening");
        assert_eq!(attempts.streamed_tick_count, 6_000);
        let environment = screen_bridge_environment(
            &terrain,
            &materialized,
            Some(&pad),
            &fixture.policy,
            &fixture.vehicle,
        );
        assert_compact_bridge_matches(&compact, &materialized, &environment);
    }

    #[test]
    fn validated_heightfield_clearance_matches_core_point_query() {
        let fixture = fixture();
        let assert_equivalent = |terrain: TerrainDefinition,
                                 center_m: Vec2,
                                 envelope: CorridorEnvelope| {
            let validated = ValidatedHeightfieldV2::new(&terrain).unwrap();
            match (
                terrain.exact_point_clearance(center_m, envelope),
                validated.exact_point_clearance(center_m, envelope),
            ) {
                (Ok(core), Some(fast)) => assert!(scalar_matches(core.minimum_clearance_m, fast)),
                (Err(_), None) => {}
                (core, fast) => panic!(
                    "validated terrain query diverged from core: core={core:?}, fast={fast:?}"
                ),
            }
        };

        let flat = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0)],
        };
        // Flat terrain and a point-spanning interval.
        assert_equivalent(
            flat.clone(),
            Vec2::new(0.0, 10.0),
            CorridorEnvelope::new(2.0, 3.0),
        );
        let ridge = TerrainDefinition::Heightfield {
            points_m: vec![
                Vec2::new(-10.0, 0.0),
                Vec2::new(-1.0, 4.0),
                Vec2::new(1.0, 9.0),
                Vec2::new(10.0, 0.0),
            ],
        };
        // A contained ridge vertex and a span crossing several vertices.
        assert_equivalent(
            ridge.clone(),
            Vec2::new(0.0, 14.0),
            CorridorEnvelope::new(2.0, 1.0),
        );
        assert_equivalent(
            ridge.clone(),
            Vec2::new(1.0, 14.0),
            CorridorEnvelope::new(0.0, 1.0),
        );
        assert_equivalent(
            ridge.clone(),
            Vec2::new(0.0, 14.0),
            CorridorEnvelope::new(5.0, 1.0),
        );
        // A rotated hull envelope exercises non-integral extents.
        assert_equivalent(
            ridge,
            Vec2::new(0.0, 20.0),
            rotated_hull_envelope(
                Some(Vec2::new(2.0, 3.0)),
                &fixture.vehicle,
                fixture.policy.minimum_clearance_m,
            ),
        );
        // The strict domain failure has the same fast-path outcome.
        assert_equivalent(flat, Vec2::new(9.0, 10.0), CorridorEnvelope::new(2.0, 0.0));
    }

    #[test]
    fn virtual_ballistic_arc_uses_exact_semi_implicit_equations() {
        let fixture = fixture();
        let arc = VirtualBallisticArcV2::new(
            &fixture.policy,
            Vec2::new(18.0, 10.0),
            Vec2::new(2500.0, 5.0),
            2_700,
        );
        let end = arc.state_at(arc.steps);
        assert!(distance(end.position_m, arc.end_m) <= ENDPOINT_TOLERANCE);
        assert!(distance(end.velocity_mps, arc.arrival_velocity_mps) <= ENDPOINT_TOLERANCE);
        assert_eq!(arc.gravity_mps2, fixture.policy.gravity_mps2);
        let one = arc.state_at(1);
        assert_eq!(
            one.velocity_mps,
            arc.departure_velocity_mps + Vec2::new(0.0, -fixture.policy.gravity_mps2 * arc.dt_s)
        );
        let mut altered_policy = fixture.policy.clone();
        altered_policy.gravity_mps2 = 8.0;
        let altered = VirtualBallisticArcV2::new(
            &altered_policy,
            Vec2::new(18.0, 10.0),
            Vec2::new(2500.0, 5.0),
            2_700,
        );
        assert_ne!(arc.identity, altered.identity);
    }

    #[test]
    fn waypoint_coast_slew_uses_vehicle_rotation_rate() {
        let fixture = fixture();
        let source_direction = Some(Vec2::new(0.0, 1.0));
        let terminal_direction = Some(Vec2::new(1.0, 0.0));
        let margin = coast_slew_margin(1.0, source_direction, terminal_direction, &fixture.vehicle);
        let expected_required_time_s =
            std::f64::consts::FRAC_PI_2 / fixture.vehicle.max_rotation_rate_radps;
        assert!((margin.raw - (1.0 - expected_required_time_s)).abs() <= ENDPOINT_TOLERANCE);

        let mut slower_vehicle = fixture.vehicle.clone();
        slower_vehicle.max_rotation_rate_radps *= 0.5;
        let slower_margin =
            coast_slew_margin(1.0, source_direction, terminal_direction, &slower_vehicle);
        assert!(slower_margin.raw < margin.raw);
    }

    #[test]
    fn bridge_reports_coupled_thrust_and_minimum_throttle_screens() {
        let fixture = fixture();
        let overloaded = exact_discrete_bridge_v2(
            &fixture.policy,
            &fixture.vehicle,
            BridgeKindV2::Source,
            KinematicStateV2 {
                position_m: Vec2::new(0.0, 5.0),
                velocity_mps: Vec2::new(0.0, 0.0),
            },
            KinematicStateV2 {
                position_m: Vec2::new(1_000.0, 5.0),
                velocity_mps: Vec2::new(0.0, 0.0),
            },
            2,
        )
        .unwrap();
        assert!(
            overloaded
                .reasons
                .contains(&DirectBridgeReasonV2::CoupledThrust)
        );

        let mut high_minimum = fixture.vehicle.clone();
        high_minimum.min_throttle_frac = 0.9;
        let low_throttle = exact_discrete_bridge_v2(
            &fixture.policy,
            &high_minimum,
            BridgeKindV2::Terminal,
            KinematicStateV2 {
                position_m: Vec2::new(0.0, 10.0),
                velocity_mps: Vec2::new(0.0, 0.0),
            },
            KinematicStateV2 {
                position_m: Vec2::new(0.0, 10.0),
                velocity_mps: Vec2::new(0.0, 0.0),
            },
            120,
        )
        .unwrap();
        assert!(
            low_throttle
                .reasons
                .contains(&DirectBridgeReasonV2::MinimumThrottle)
        );
    }

    #[test]
    fn evaluator_has_four_deterministically_ordered_duration_candidates() {
        let first = evaluation();
        let second = evaluation();
        assert_eq!(first, second);
        for result in &first.results {
            assert_eq!(result.candidates.len(), 4);
            assert!(result.candidates.windows(2).all(|pair| {
                pair[0].virtual_arc.steps < pair[1].virtual_arc.steps
                    && pair[0].duration_multiplier < pair[1].duration_multiplier
            }));
        }
    }

    #[test]
    fn generic_direct_case_v2_matches_every_frozen_fixture_result() {
        let fixture = fixture();
        let expected = evaluation();
        for case in &fixture.cases {
            let actual = evaluate_direct_bridge_case_v2(&fixture.policy, &fixture.vehicle, case)
                .expect("validated frozen probe evaluates");
            let frozen = expected
                .results
                .iter()
                .find(|result| result.id == case.id)
                .expect("fixture evaluator returns each frozen probe");
            assert_eq!(
                &actual, frozen,
                "probe {} changed under generic API",
                case.id
            );
        }
    }

    #[test]
    fn generic_direct_case_v2_is_deterministic() {
        let fixture = fixture();
        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == "clear_direct_probe")
            .expect("embedded clear probe");
        let first = evaluate_direct_bridge_case_v2(&fixture.policy, &fixture.vehicle, case)
            .expect("validated frozen probe evaluates");
        let second = evaluate_direct_bridge_case_v2(&fixture.policy, &fixture.vehicle, case)
            .expect("validated frozen probe evaluates again");
        assert_eq!(first, second);
    }

    #[test]
    fn generic_direct_case_v2_rejects_invalid_case() {
        let fixture = fixture();
        let mut case = fixture.cases[0].clone();
        case.id.clear();
        let error = evaluate_direct_bridge_case_v2(&fixture.policy, &fixture.vehicle, &case)
            .expect_err("empty probe id must fail closed");
        assert!(error.contains("probe id must not be empty"));
    }

    #[test]
    fn waypoint_position_step_v2_uses_shared_unrotated_hull_envelope() {
        let fixture = fixture();
        let actual = waypoint_position_step_v2(&fixture.policy, &fixture.vehicle).unwrap();
        let envelope =
            rotated_hull_envelope(None, &fixture.vehicle, fixture.policy.minimum_clearance_m);
        assert_eq!(
            actual,
            Vec2::new(
                envelope
                    .horizontal_extent_m
                    .max(fixture.policy.minimum_clearance_m),
                envelope
                    .vertical_extent_m
                    .max(fixture.policy.minimum_clearance_m),
            )
        );
    }

    #[test]
    fn one_waypoint_case_v2_rejects_invalid_duplicate_and_out_of_span_positions() {
        let fixture = fixture();
        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == "clear_direct_probe")
            .unwrap();
        let release = release_reference(case, &fixture.policy, &fixture.vehicle);
        let touchdown = touchdown_reference(case, &fixture.vehicle);
        let valid = Vec2::new((release.x + touchdown.x) * 0.5, release.y + 20.0);

        let non_finite = Vec2::new(valid.x, f64::NAN);
        let error =
            evaluate_one_waypoint_case_v2(&fixture.policy, &fixture.vehicle, case, &[non_finite])
                .unwrap_err();
        assert!(error.contains("must be finite"));

        let error =
            evaluate_one_waypoint_case_v2(&fixture.policy, &fixture.vehicle, case, &[valid, valid])
                .unwrap_err();
        assert!(error.contains("duplicate positions"));

        let error = evaluate_one_waypoint_case_v2(
            &fixture.policy,
            &fixture.vehicle,
            case,
            &[Vec2::new(release.x, valid.y)],
        )
        .unwrap_err();
        assert!(error.contains("strictly within"));
    }

    #[test]
    fn one_waypoint_case_v2_is_deterministic_for_generic_probe_and_positions() {
        let fixture = fixture();
        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == "clear_direct_probe")
            .unwrap();
        let release = release_reference(case, &fixture.policy, &fixture.vehicle);
        let touchdown = touchdown_reference(case, &fixture.vehicle);
        let positions = [Vec2::new((release.x + touchdown.x) * 0.5, release.y + 20.0)];
        let first =
            evaluate_one_waypoint_case_v2(&fixture.policy, &fixture.vehicle, case, &positions)
                .expect("valid generic waypoint search");
        let second =
            evaluate_one_waypoint_case_v2(&fixture.policy, &fixture.vehicle, case, &positions)
                .expect("same generic waypoint search");
        assert_eq!(first, second);
        assert_eq!(first.waypoint_position_count, 1);
        assert_eq!(first.leg_duration_pair_count, 16);
        assert!(first.candidate_count <= first.max_candidates);
        assert_eq!(
            first.certified_candidate_count,
            usize::from(first.selected_candidate.is_some())
        );
        if let Some(selected) = &first.selected_candidate {
            assert_eq!(selected.classification, CertificationV2::Certified);
            assert_eq!(
                first.selected_candidate_identity.as_deref(),
                Some(selected.identity.as_str())
            );
        } else {
            assert_eq!(first.certified_candidate_count, 0);
            assert!(first.candidates.is_empty());
        }
    }

    #[test]
    fn frozen_ridge_canary_and_waypoint_search_identities_are_pinned() {
        let canary = &evaluation().ridge_canary;
        assert_eq!(canary.identity, "fnv1a64:43f7e8bb46696733");
        assert_eq!(canary.waypoint_search.identity, "fnv1a64:402076b715be3322");
    }

    #[test]
    fn ridge_canary_uses_flat_shortest_nominal_and_keeps_global_replans_diagnostic() {
        let canary = &evaluation().ridge_canary;
        assert_eq!(canary.schema_id, "conservative_ballistic_ridge_canary_v2");
        assert_eq!(canary.schema_version, 2);
        assert_eq!(canary.flat_control.status, MissionStatusV2::Green);
        let first_certified = canary
            .flat_control
            .candidate_classifications
            .iter()
            .position(|classification| *classification == CertificationV2::Certified)
            .expect("flat twin has a certified candidate");
        assert_eq!(
            canary.flat_control.nominal_duration_multiplier,
            canary.flat_control.candidate_duration_multipliers[first_certified]
        );
        assert!(
            canary.flat_control.candidate_classifications[..first_certified]
                .iter()
                .all(|classification| *classification == CertificationV2::NotCertified)
        );
        assert_eq!(
            canary.nominal_candidate_identity,
            canary.flat_control.nominal_candidate_identity
        );
        assert_eq!(canary.direct_status, MissionStatusV2::Red);
        assert!(canary.blocking_lane_valid);
        assert!(
            canary
                .direct_nominal_candidate
                .reasons
                .contains(&DirectBridgeReasonV2::TerrainClearance)
        );
        assert!(
            canary
                .direct_global_replans
                .iter()
                .any(|candidate| candidate.classification == CertificationV2::Certified)
        );
        assert!(canary.direct_global_replans.iter().all(|candidate| {
            candidate.candidate_identity != canary.nominal_candidate_identity
        }));
    }

    #[test]
    fn correction_envelope_and_derived_mesa_block_the_declared_lane() {
        let canary = &evaluation().ridge_canary;
        let envelope = &canary.correction_envelope;
        assert_eq!(
            envelope.commitment_phase,
            CorrectionCommitmentPhaseV2::PostSourceHandoff
        );
        assert_eq!(envelope.source_handoff.state, envelope.commitment_state);
        assert!(envelope.correction_horizon_s > envelope.crossing_time_s);
        assert!(envelope.derated_max_acceleration_mps2 > 0.0);
        assert!(envelope.available_correction_fuel_kg > 0.0);
        assert!(envelope.samples.len() >= 3);
        assert!(envelope.samples.first().unwrap().correction_time_s >= 0.0);
        assert!(envelope.samples.windows(2).all(|pair| {
            pair[0].arc_step <= pair[1].arc_step
                && pair[0].correction_time_s <= pair[1].correction_time_s
        }));
        assert!(envelope.samples.iter().any(|sample| {
            sample.time_shift_allowance_s > 0.0
                && sample.nominal_time_shift_drift_bounds_m.upper_m.x > 0.0
        }));
        let crossing_cut = &envelope.crossing_cut;
        assert!(crossing_cut.eligible_sample_count > 0);
        assert!(crossing_cut.first_arc_step.unwrap() <= envelope.nominal_crossing_arc_step);
        assert!(crossing_cut.last_arc_step.unwrap() >= envelope.nominal_crossing_arc_step);
        assert!(crossing_cut.bounds.lower_m.x <= crossing_cut.feature_center_x_m);
        assert!(crossing_cut.bounds.upper_m.x >= crossing_cut.feature_center_x_m);
        let mesa = &canary.mesa;
        assert!(mesa.blocks_full_correction_corridor);
        assert!(mesa.correction_blocking_margin_m.passes(&fixture().policy));
        assert!(mesa.top_left_x_m <= crossing_cut.bounds.lower_m.x + ENDPOINT_TOLERANCE);
        assert!(mesa.top_right_x_m >= crossing_cut.bounds.upper_m.x - ENDPOINT_TOLERANCE);
        assert!(mesa.top_y_m >= crossing_cut.bounds.upper_m.y);
    }

    #[test]
    fn ridge_canary_finds_a_derived_one_waypoint_witness_with_all_three_bridges() {
        let canary = &evaluation().ridge_canary;
        let search = &canary.waypoint_search;
        assert!(search.candidate_count > 0);
        assert!(search.candidate_count <= search.max_candidates);
        assert!(search.waypoint_position_count > 0);
        assert!(search.leg_duration_pair_count > 0);
        assert_eq!(search.certified_candidate_count, 1);
        let candidate = search
            .selected_candidate
            .as_ref()
            .expect("derived waypoint witness");
        assert_eq!(candidate.classification, CertificationV2::Certified);
        assert_eq!(
            search.selected_candidate_identity.as_deref(),
            Some(candidate.identity.as_str())
        );
        assert_eq!(
            candidate.intermediate_bridge.as_ref().unwrap().kind,
            BridgeKindV2::Intermediate
        );
        for bridge in [
            candidate.source_bridge.as_ref(),
            candidate.intermediate_bridge.as_ref(),
            candidate.terminal_bridge.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(bridge.classification, CertificationV2::Certified);
            assert!(bridge.endpoint_position_error_m <= ENDPOINT_TOLERANCE);
            assert!(bridge.endpoint_velocity_error_mps <= ENDPOINT_TOLERANCE);
        }
        assert!(
            candidate.source_handoff.unwrap().arc_step
                < candidate.intermediate_entry_handoff.unwrap().arc_step
        );
        assert!(
            candidate.intermediate_exit_handoff.unwrap().arc_step
                < candidate.terminal_handoff.unwrap().arc_step
        );
        assert!(
            candidate.margins.minimum_normalized() >= fixture().policy.declared_robustness_margin
        );
        let route_progress = candidate
            .route_progress
            .as_ref()
            .expect("selected witness has route-progress evidence");
        assert!(route_progress.passes);
        assert_eq!(route_progress.segments.len(), 5);
        assert!(route_progress.segments.iter().all(|segment| segment.passes));
        let intermediate_progress = route_progress
            .segments
            .iter()
            .find(|segment| segment.segment == RouteProgressSegmentV2::IntermediateBridge)
            .expect("intermediate progress evidence");
        assert!(intermediate_progress.minimum_tangent_velocity_mps > 0.0);
        assert!(intermediate_progress.backtracking_distance_m <= ENDPOINT_TOLERANCE);
        assert!(search.rejection_counts.iter().any(|(reason, count)| {
            *reason == DirectBridgeReasonV2::RouteProgress && *count > 0
        }));

        let fixture = fixture();
        let ridge_case = fixture
            .cases
            .iter()
            .find(|case| case.id == canary.source_case_id)
            .expect("ridge canary source case");
        let positions =
            waypoint_positions(&fixture.policy, &fixture.vehicle, ridge_case, &canary.mesa);
        let release_x_m = release_reference(ridge_case, &fixture.policy, &fixture.vehicle).x;
        let touchdown_x_m = touchdown_reference(ridge_case, &fixture.vehicle).x;
        assert!(positions.iter().all(|position| {
            position.x > release_x_m.min(touchdown_x_m) + ENDPOINT_TOLERANCE
                && position.x < release_x_m.max(touchdown_x_m) - ENDPOINT_TOLERANCE
        }));
        assert!(positions.iter().all(|position| {
            position.x >= ridge_case.terrain_points_m.first().unwrap().x
                && position.x <= ridge_case.terrain_points_m.last().unwrap().x
        }));
    }

    #[test]
    fn flat_negative_control_has_no_derived_blocker_or_waypoint_requirement() {
        let fixture = fixture();
        let original = fixture
            .cases
            .iter()
            .find(|case| case.id == "ridge_probe")
            .unwrap();
        let mut flat_case = original.clone();
        flat_case.terrain_points_m = vec![Vec2::new(-40.0, 0.0), Vec2::new(4040.0, 0.0)];
        let canary =
            evaluate_ridge_canary_case_v2(&fixture.policy, &fixture.vehicle, &flat_case).unwrap();
        assert_eq!(canary.flat_control.status, MissionStatusV2::Green);
        assert_eq!(canary.direct_status, MissionStatusV2::Green);
        assert!(!canary.blocking_lane_valid);
        assert!(!canary.mesa.blocks_full_correction_corridor);
        assert_eq!(canary.waypoint_search.candidate_count, 0);
        assert!(canary.waypoint_search.selected_candidate.is_none());
    }

    #[test]
    fn selected_complete_certificate_joins_exact_handoffs_and_coast() {
        let evaluation = evaluation();
        let candidate = clear_result(&evaluation)
            .candidates
            .iter()
            .find(|candidate| candidate.classification == CertificationV2::Certified)
            .expect("clear direct probe has a complete certificate");
        let source_handoff = candidate.source_handoff.expect("selected source handoff");
        let terminal_handoff = candidate
            .terminal_handoff
            .expect("selected terminal handoff");
        let source_bridge = candidate.source_bridge.as_ref().expect("source bridge");
        let terminal_bridge = candidate.terminal_bridge.as_ref().expect("terminal bridge");
        let coast = candidate.selected_coast.as_ref().expect("coast evidence");
        assert!(source_handoff.arc_step < terminal_handoff.arc_step);
        assert!(
            distance(
                source_bridge.end_state.position_m,
                source_handoff.state.position_m
            ) <= ENDPOINT_TOLERANCE
        );
        assert!(
            distance(
                source_bridge.end_state.velocity_mps,
                source_handoff.state.velocity_mps
            ) <= ENDPOINT_TOLERANCE
        );
        assert!(
            distance(
                terminal_bridge.start_state.position_m,
                terminal_handoff.state.position_m
            ) <= ENDPOINT_TOLERANCE
        );
        assert!(
            distance(
                terminal_bridge.start_state.velocity_mps,
                terminal_handoff.state.velocity_mps
            ) <= ENDPOINT_TOLERANCE
        );
        assert!(coast.coast_slew_margin.passes(&fixture().policy));
    }

    #[test]
    fn pad_corridor_uses_nominal_plane_and_free_flight_uses_hull_clearance() {
        let fixture = fixture();
        let terrain = TerrainDefinition::Heightfield {
            points_m: vec![Vec2::new(-100.0, 0.0), Vec2::new(100.0, 0.0)],
        };
        let validated_terrain = ValidatedHeightfieldV2::new(&terrain).unwrap();
        let upright_state = KinematicStateV2 {
            position_m: Vec2::new(0.0, fixture.vehicle.geometry.touchdown_base_offset_m),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let pad = PadInputV2 {
            center_x_m: 0.0,
            surface_y_m: 0.0,
            width_m: 36.0,
        };
        let query_both = |state, direction, support_pad| {
            let materialized = clearance_for_state(
                &terrain,
                state,
                direction,
                support_pad,
                &fixture.policy,
                &fixture.vehicle,
            );
            let compact = clearance_for_state_fast(
                &validated_terrain,
                state,
                direction,
                support_pad,
                &fixture.policy,
                &fixture.vehicle,
            );
            assert_eq!(compact, materialized);
            materialized
        };

        // Exact upright contact on a declared pad is legal at the nominal
        // center plane.
        let (contact_margin, in_pad_corridor) =
            query_both(upright_state, Some(Vec2::new(0.0, 1.0)), Some(&pad));
        assert!(in_pad_corridor);
        assert!(contact_margin.passes(&fixture.policy));

        // A small legal tilt stays in contact when its entire touchdown
        // footprint remains on the declared pad. Its corner need not be
        // reinterpreted as airborne terrain penetration.
        let tilt_rad: f64 = 5.0e-5;
        let tilted_direction = Some(Vec2::new(tilt_rad.sin(), tilt_rad.cos()));
        let (tilted_margin, tilted_in_pad_corridor) =
            query_both(upright_state, tilted_direction, Some(&pad));
        assert!(tilted_in_pad_corridor);
        assert!(tilted_margin.passes(&fixture.policy));

        // A vehicle above the same flat pad remains in the declared vertical
        // launch/landing corridor while its full footprint is over the pad.
        // It clears the pad plane directly rather than applying the general
        // free-flight reserve before there is room to leave the surface.
        let above_pad = KinematicStateV2 {
            position_m: Vec2::new(0.0, fixture.vehicle.geometry.touchdown_base_offset_m + 0.1),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let (above_margin, above_in_pad_corridor) =
            query_both(above_pad, tilted_direction, Some(&pad));
        assert!(above_in_pad_corridor);
        assert!(above_margin.passes(&fixture.policy));

        // The same nominal contact height just beyond the pad footprint is
        // free flight, so it must satisfy the full hull-plus-buffer terrain
        // screen and rejects here.
        let outside_pad = KinematicStateV2 {
            position_m: Vec2::new(22.0, fixture.vehicle.geometry.touchdown_base_offset_m),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let (airborne_margin, airborne_in_pad_corridor) =
            query_both(outside_pad, tilted_direction, Some(&pad));
        assert!(!airborne_in_pad_corridor);
        assert!(!airborne_margin.passes(&fixture.policy));

        // Even over the full pad footprint, a center below the legal contact
        // plane is rejected by the pad-corridor clearance screen.
        let below_plane = KinematicStateV2 {
            position_m: Vec2::new(
                0.0,
                fixture.vehicle.geometry.touchdown_base_offset_m - (2.0 * ENDPOINT_TOLERANCE),
            ),
            velocity_mps: Vec2::new(0.0, 0.0),
        };
        let (below_margin, below_in_pad_corridor) =
            query_both(below_plane, tilted_direction, Some(&pad));
        assert!(below_in_pad_corridor);
        assert!(!below_margin.passes(&fixture.policy));
    }

    #[test]
    fn clear_certificate_ends_at_target_with_terminal_attitude_and_rate() {
        let fixture = fixture();
        let evaluation = evaluation();
        let result = clear_result(&evaluation);
        assert!(result.certified_candidate_count > 0);
        let candidate = result
            .candidates
            .iter()
            .find(|candidate| candidate.classification == CertificationV2::Certified)
            .expect("clear direct certificate");
        let terminal = candidate.terminal_bridge.as_ref().expect("terminal bridge");
        assert!(
            distance(terminal.end_state.position_m, result.touchdown_reference_m)
                <= ENDPOINT_TOLERANCE
        );
        assert!(terminal.end_state.velocity_mps.x.abs() <= ENDPOINT_TOLERANCE);
        assert!(
            (terminal.end_state.velocity_mps.y
                + fixture.policy.terminal_target_downward_speed_fraction
                    * fixture.vehicle.safe_touchdown_normal_speed_mps)
                .abs()
                <= ENDPOINT_TOLERANCE
        );
        let terminal_environment = candidate
            .terminal_environment
            .as_ref()
            .expect("terminal screen");
        assert!(
            terminal_environment
                .final_attitude_margin
                .passes(&fixture.policy)
        );
        assert!(
            terminal_environment
                .final_angular_rate_margin
                .passes(&fixture.policy)
        );
    }

    #[test]
    fn evaluation_identity_recomputes_and_rejects_tampering() {
        let evaluation = evaluation();
        validate_embedded_evaluation_v2(&evaluation).unwrap();
        let mut tampered = evaluation;
        tampered.results[0].id.push_str("-tampered");
        assert!(validate_embedded_evaluation_v2(&tampered).is_err());
    }

    #[test]
    fn experimental_runtime_projection_emits_exact_routes_and_handoff_evidence() {
        let candidate_projection = evaluate_embedded_experimental_ridge_candidate_v2().unwrap();
        let runtime = project_experimental_ridge_runtime_v2(&candidate_projection).unwrap();
        assert_eq!(runtime.identity, runtime_projection_identity_v2(&runtime));
        assert_eq!(
            runtime.schema_id,
            EXPERIMENTAL_RIDGE_RUNTIME_PROJECTION_SCHEMA_ID_V2
        );
        assert_eq!(
            runtime.schema_version,
            EXPERIMENTAL_RIDGE_RUNTIME_PROJECTION_SCHEMA_VERSION_V2
        );

        let ExperimentalRidgeRuntimeOutcomeV2::Direct {
            candidate_identity,
            route,
            structural_validation,
            identity,
        } = &runtime.flat_control
        else {
            panic!("flat twin must project to Direct");
        };
        let ExperimentalRidgeCandidateOutcomeV2::Direct { candidate } =
            &candidate_projection.flat_control
        else {
            panic!("flat candidate must be Direct");
        };
        assert_eq!(candidate_identity, &candidate.identity);
        assert!(route.waypoints.is_empty());
        route.validate().unwrap();
        assert_eq!(structural_validation.topology, RouteTopology::Direct);
        assert_eq!(structural_validation.waypoint_count, 0);
        assert!(structural_validation.transfer_route_valid);
        assert_eq!(
            identity,
            &runtime_outcome_identity_v2(&runtime.flat_control)
        );

        let ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint {
            candidate_identity,
            route,
            crossing,
            authority,
            handoff_kinematics,
            handoff_assessment,
            structural_validation,
            identity,
        } = &runtime.derived_mesa
        else {
            panic!("derived mesa must project to OneWaypoint");
        };
        let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
            candidate,
            crossing: candidate_crossing,
        } = &candidate_projection.derived_mesa
        else {
            panic!("derived candidate must be OneWaypoint");
        };
        assert_eq!(candidate_identity, &candidate.identity);
        assert_eq!(crossing, candidate_crossing.as_ref());
        assert_eq!(route.waypoints.len(), 1);
        route.validate().unwrap();
        let waypoint = &route.waypoints[0];
        assert_eq!(waypoint.position_m, crossing.selected_state.position_m);
        assert_eq!(
            waypoint.capture_radius_m,
            (route.route_radius_m * 0.08).clamp(35.0, 95.0)
        );
        assert_eq!(
            waypoint.max_cross_track_m, waypoint.capture_radius_m,
            "capture and cross-track bounds must use the existing policy"
        );
        assert_eq!(
            waypoint.max_speed_mps,
            authority
                .handoff_speed_cap_mps
                .min(RoutePlanningPolicy::v1().max_handoff_speed_mps)
        );
        assert!(handoff_assessment.contract_pass);
        assert!(handoff_assessment.triggered);
        assert!(handoff_assessment.spatial_pass);
        assert!(handoff_assessment.envelope_pass);
        assert!(handoff_kinematics.speed_mps.is_finite());
        assert_eq!(structural_validation.topology, RouteTopology::Waypoint);
        assert_eq!(structural_validation.waypoint_count, 1);
        assert!(structural_validation.transfer_route_valid);
        assert_eq!(
            identity,
            &runtime_outcome_identity_v2(&runtime.derived_mesa)
        );
        validate_experimental_ridge_runtime_v2(&candidate_projection, &runtime).unwrap();
    }

    #[test]
    fn experimental_runtime_projection_is_deterministic_and_fails_closed() {
        let candidate_projection = evaluate_embedded_experimental_ridge_candidate_v2().unwrap();
        let first = project_experimental_ridge_runtime_v2(&candidate_projection).unwrap();
        let second = project_experimental_ridge_runtime_v2(&candidate_projection).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );

        let mut tampered_candidate = candidate_projection.clone();
        let ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { crossing, .. } =
            &mut tampered_candidate.derived_mesa
        else {
            panic!("derived candidate must be OneWaypoint");
        };
        crossing.selected_applied_steps += 1;
        assert_eq!(
            project_experimental_ridge_runtime_v2(&tampered_candidate),
            Err(
                ExperimentalRidgeRuntimeProjectionErrorV2::CandidateEvidence(
                    ExperimentalRidgeCandidateErrorV2::IntermediateCrossingMismatch
                )
            )
        );

        let mut tampered_runtime = first.clone();
        let ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint { route, .. } =
            &mut tampered_runtime.derived_mesa
        else {
            panic!("derived runtime must be OneWaypoint");
        };
        route.waypoints[0].max_speed_mps += 1.0;
        assert_eq!(
            validate_experimental_ridge_runtime_v2(&candidate_projection, &tampered_runtime),
            Err(ExperimentalRidgeRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch)
        );
    }

    #[test]
    fn generic_ridge_case_projection_recomputes_from_one_identity_bound_input() {
        let input = generic_ridge_input();
        assert_eq!(input.schema_id, EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_ID_V1);
        assert_eq!(
            input.schema_version,
            EXPERIMENTAL_RIDGE_CASE_INPUT_SCHEMA_VERSION_V1
        );
        input.validate().unwrap();

        let projection = evaluate_experimental_ridge_case_projection_v1(&input).unwrap();
        assert_eq!(
            projection.schema_id,
            EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_ID_V1
        );
        assert_eq!(
            projection.schema_version,
            EXPERIMENTAL_RIDGE_CASE_PROJECTION_SCHEMA_VERSION_V1
        );
        assert_eq!(projection.input_identity, input.identity);
        assert_eq!(
            projection.identity,
            experimental_ridge_case_projection_identity_v1(&projection)
        );
        validate_experimental_ridge_case_projection_v1(&projection).unwrap();
        let reloaded: ExperimentalRidgeCaseProjectionV1 =
            serde_json::from_slice(&serde_json::to_vec(&projection).unwrap()).unwrap();
        assert_eq!(reloaded, projection);
        assert_eq!(reloaded.identity, projection.identity);
        validate_experimental_ridge_case_projection_v1(&reloaded).unwrap();

        let mut schema_tampered = input.clone();
        schema_tampered.schema_version += 1;
        assert_eq!(
            evaluate_experimental_ridge_case_projection_v1(&schema_tampered),
            Err(ExperimentalRidgeCandidateErrorV2::GenericInputInvalid)
        );
        let mut identity_tampered = input;
        identity_tampered.identity.push_str("-tampered");
        let identity_tampered: ExperimentalRidgeCaseInputV1 =
            serde_json::from_slice(&serde_json::to_vec(&identity_tampered).unwrap()).unwrap();
        assert_eq!(
            evaluate_experimental_ridge_case_projection_v1(&identity_tampered),
            Err(ExperimentalRidgeCandidateErrorV2::GenericInputIdentityMismatch)
        );
    }

    #[test]
    fn generic_ridge_case_constructor_rejects_non_finite_input_without_panicking() {
        let fixture = fixture();
        let probe = fixture
            .cases
            .into_iter()
            .find(|case| case.id == "ridge_probe")
            .expect("embedded ridge probe");
        let mut policy = fixture.policy;
        policy.gravity_mps2 = f64::NAN;
        let result = std::panic::catch_unwind(|| {
            ExperimentalRidgeCaseInputV1::new(policy, fixture.vehicle, probe)
        });
        assert!(matches!(
            result,
            Ok(Err(ExperimentalRidgeCandidateErrorV2::GenericInputInvalid))
        ));
    }

    #[test]
    fn generic_ridge_case_mutation_changes_identities_and_recomputation_rejects_tampering() {
        let input = generic_ridge_input();
        let original = evaluate_experimental_ridge_case_projection_v1(&input).unwrap();

        let mut changed_input = input;
        changed_input.probe.terrain_points_m[2].y += 1.0;
        changed_input.identity = experimental_ridge_case_input_identity_v1(&changed_input);
        let changed = evaluate_experimental_ridge_case_projection_v1(&changed_input).unwrap();
        assert_ne!(changed_input.identity, original.input_identity);
        assert_ne!(changed.identity, original.identity);
        assert_ne!(
            changed.analytical_canary_identity,
            original.analytical_canary_identity
        );

        let mut altered = original;
        altered.mesa.top_y_m += 1.0;
        altered.identity = experimental_ridge_case_projection_identity_v1(&altered);
        assert_eq!(
            validate_experimental_ridge_case_projection_v1(&altered),
            Err(ExperimentalRidgeCandidateErrorV2::GenericProjectionMismatch)
        );

        let mut altered_nominal =
            evaluate_experimental_ridge_case_projection_v1(&generic_ridge_input()).unwrap();
        altered_nominal.derived_nominal_direct.reasons.clear();
        altered_nominal.identity = experimental_ridge_case_projection_identity_v1(&altered_nominal);
        assert_eq!(
            validate_experimental_ridge_case_projection_v1(&altered_nominal),
            Err(ExperimentalRidgeCandidateErrorV2::GenericProjectionMismatch)
        );

        let mut altered_replan =
            evaluate_experimental_ridge_case_projection_v1(&generic_ridge_input()).unwrap();
        altered_replan
            .derived_non_nominal_direct_diagnostics
            .clear();
        altered_replan.identity = experimental_ridge_case_projection_identity_v1(&altered_replan);
        assert_eq!(
            validate_experimental_ridge_case_projection_v1(&altered_replan),
            Err(ExperimentalRidgeCandidateErrorV2::GenericProjectionMismatch)
        );
    }

    #[test]
    fn generic_ridge_case_runtime_projection_rebuilds_and_rejects_route_tampering() {
        let candidate =
            evaluate_experimental_ridge_case_projection_v1(&generic_ridge_input()).unwrap();
        let runtime = project_experimental_ridge_case_runtime_v1(&candidate).unwrap();
        assert_eq!(
            runtime.schema_id,
            EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V1
        );
        assert_eq!(
            runtime.schema_version,
            EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V1
        );
        assert_eq!(
            runtime.identity,
            experimental_ridge_case_runtime_projection_identity_v1(&runtime)
        );
        validate_experimental_ridge_case_runtime_v1(&candidate, &runtime).unwrap();
        let reloaded: ExperimentalRidgeCaseRuntimeProjectionV1 =
            serde_json::from_slice(&serde_json::to_vec(&runtime).unwrap()).unwrap();
        assert_eq!(reloaded, runtime);
        assert_eq!(reloaded.identity, runtime.identity);
        validate_experimental_ridge_case_runtime_v1(&candidate, &reloaded).unwrap();

        let mut altered = runtime;
        let ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint { route, .. } =
            &mut altered.derived_mesa
        else {
            panic!("renamed embedded ridge must retain a one-waypoint repair");
        };
        route.waypoints[0].max_speed_mps += 1.0;
        altered.identity = experimental_ridge_case_runtime_projection_identity_v1(&altered);
        assert_eq!(
            validate_experimental_ridge_case_runtime_v1(&candidate, &altered),
            Err(ExperimentalRidgeRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch)
        );
    }

    #[test]
    fn generic_runtime_projection_v2_keeps_primary_crossing_compatible_with_v1() {
        let input = heldout_ridge_input("ridge_progress_050_probe");
        let candidate = evaluate_experimental_ridge_case_projection_v1(&input).unwrap();
        let v1 = project_experimental_ridge_case_runtime_v1(&candidate).unwrap();
        let v2 = project_experimental_ridge_case_runtime_v2(&candidate).unwrap();

        assert_eq!(
            v2.schema_id,
            EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_ID_V2
        );
        assert_eq!(
            v2.schema_version,
            EXPERIMENTAL_RIDGE_CASE_RUNTIME_PROJECTION_SCHEMA_VERSION_V2
        );
        let ExperimentalRidgeRuntimeOutcomeV2::OneWaypoint {
            route: v1_route,
            crossing: v1_crossing,
            authority: v1_authority,
            handoff_kinematics: v1_kinematics,
            handoff_assessment: v1_assessment,
            ..
        } = &v1.derived_mesa
        else {
            panic!("V1 050 projection must retain its one-waypoint outcome");
        };
        let ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            route: v2_route,
            crossing: v2_crossing,
            authority: v2_authority,
            handoff_kinematics: v2_kinematics,
            handoff_assessment: v2_assessment,
            handoff_selection,
            ..
        } = &v2.derived_mesa
        else {
            panic!("V2 050 projection must retain its one-waypoint outcome");
        };
        assert_eq!(v2_route, v1_route);
        assert_eq!(v2_crossing, v1_crossing);
        assert_eq!(v2_authority, v1_authority);
        assert_eq!(v2_kinematics, v1_kinematics);
        assert_eq!(v2_assessment, v1_assessment);
        assert_eq!(handoff_selection.attempts.len(), 1);
        assert_eq!(
            handoff_selection.attempts[handoff_selection.selected_attempt_index].selection_kind,
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::PrimaryCrossing
        );
        assert_eq!(handoff_selection.selected_attempt_index, 0);
        let selected_attempt = handoff_selection
            .selected_attempt()
            .expect("selected primary attempt");
        assert_eq!(
            selected_attempt.applied_steps,
            v1_crossing.selected_applied_steps
        );
        assert_eq!(selected_attempt.target_leg_arc_step, None);
        assert_eq!(selected_attempt.route, *v1_route);
        assert_eq!(selected_attempt.authority, *v1_authority);
        assert_eq!(selected_attempt.handoff_kinematics, *v1_kinematics);
        assert_eq!(selected_attempt.handoff_assessment, *v1_assessment);
        assert_eq!(
            selected_attempt.selected_state,
            handoff_selection.crossing.selected_state
        );
        validate_experimental_ridge_case_runtime_v2(&candidate, &v2).unwrap();
    }

    #[test]
    fn generic_runtime_projection_v2_falls_back_to_068_intermediate_apex_exit() {
        let input = heldout_ridge_input("ridge_progress_068_probe");
        let candidate = evaluate_experimental_ridge_case_projection_v1(&input).unwrap();
        assert_eq!(
            project_experimental_ridge_case_runtime_v1(&candidate),
            Err(ExperimentalRidgeRuntimeProjectionErrorV2::HandoffContractFailed)
        );
        let expected_target_apex_step = match &candidate.derived_mesa {
            ExperimentalRidgeCandidateOutcomeV2::OneWaypoint { candidate, .. } => {
                candidate.target_leg.apex_step
            }
            _ => panic!("V1 068 candidate must retain a one-waypoint outcome"),
        };
        let runtime = project_experimental_ridge_case_runtime_v2(&candidate).unwrap();
        let ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            route,
            handoff_selection,
            handoff_kinematics,
            handoff_assessment,
            ..
        } = &runtime.derived_mesa
        else {
            panic!("V2 068 projection must retain a one-waypoint outcome");
        };
        assert_eq!(handoff_selection.attempts.len(), 2);
        assert_eq!(handoff_selection.selected_attempt_index, 1);
        assert_eq!(
            handoff_selection.attempts[handoff_selection.selected_attempt_index].selection_kind,
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::IntermediateBridgeExit
        );
        let primary = &handoff_selection.attempts[0];
        assert_eq!(
            primary.selection_kind,
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::PrimaryCrossing
        );
        assert_eq!(primary.applied_steps, 1406);
        assert_eq!(
            primary.handoff_assessment.violations,
            vec!["heading".to_owned()]
        );
        assert!(
            (primary.handoff_kinematics.outbound_heading_error_rad - 0.3879286123780121).abs()
                < 1.0e-12
        );
        assert_eq!(primary.target_leg_arc_step, None);

        let selected = handoff_selection
            .selected_attempt()
            .expect("selected fallback attempt");
        assert_eq!(
            selected.selection_kind,
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::IntermediateBridgeExit
        );
        assert_eq!(selected.applied_steps, 3960);
        assert_eq!(
            selected.target_leg_arc_step,
            Some(expected_target_apex_step)
        );
        assert_eq!(
            selected.selected_state.position_m,
            Vec2::new(2715.400408105346, 1931.4569053495914)
        );
        assert_eq!(
            selected.selected_state.velocity_mps,
            Vec2::new(64.82420144127775, 0.027651643418408867)
        );
        assert_eq!(
            route.waypoints[0].position_m,
            selected.selected_state.position_m
        );
        assert!((selected.authority.handoff_speed_cap_mps - 93.2925267109143).abs() < 1.0e-12);
        assert!(
            (handoff_kinematics.outbound_heading_error_rad - 0.19282817224525745).abs() < 1.0e-12
        );
        assert!((handoff_kinematics.outbound_cross_speed_mps - 12.4226137521142).abs() < 1.0e-12);
        assert!((handoff_kinematics.speed_mps - 64.8242073388695).abs() < 1.0e-12);
        assert!(handoff_assessment.contract_pass);
        assert!(handoff_assessment.violations.is_empty());
        validate_experimental_ridge_case_runtime_v2(&candidate, &runtime).unwrap();
    }

    #[test]
    fn generic_runtime_projection_v2_is_reloadable_deterministic_and_binds_selection_evidence() {
        let input = heldout_ridge_input("ridge_progress_068_probe");
        let candidate = evaluate_experimental_ridge_case_projection_v1(&input).unwrap();
        let first = project_experimental_ridge_case_runtime_v2(&candidate).unwrap();
        let second = project_experimental_ridge_case_runtime_v2(&candidate).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        let reloaded: ExperimentalRidgeCaseRuntimeProjectionV2 =
            serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
        assert_eq!(reloaded, first);
        validate_experimental_ridge_case_runtime_v2(&candidate, &reloaded).unwrap();

        let mut altered = first.clone();
        let ExperimentalRidgeCaseRuntimeOutcomeV2::OneWaypoint {
            handoff_selection, ..
        } = &mut altered.derived_mesa
        else {
            panic!("V2 068 projection must retain a one-waypoint outcome");
        };
        handoff_selection.attempts[handoff_selection.selected_attempt_index].applied_steps += 1;
        altered.identity = experimental_ridge_case_runtime_projection_identity_v2(&altered);
        assert_eq!(
            validate_experimental_ridge_case_runtime_v2(&candidate, &altered),
            Err(ExperimentalRidgeCaseRuntimeProjectionErrorV2::RuntimeProjectionIdentityMismatch)
        );
    }

    #[test]
    fn generic_runtime_projection_v2_error_retains_both_failed_attempts() {
        let input = heldout_ridge_input("ridge_progress_068_probe");
        let projection = evaluate_experimental_ridge_case_projection_v1(&input).unwrap();
        let (candidate, crossing) = match &projection.derived_mesa {
            ExperimentalRidgeCandidateOutcomeV2::OneWaypoint {
                candidate,
                crossing,
            } => (candidate.as_ref(), crossing.as_ref()),
            _ => panic!("V2 068 candidate must retain a one-waypoint outcome"),
        };
        let mut request = runtime_request_components_v2(
            &projection.policy,
            &projection.vehicle,
            &projection.probe,
            TerrainDefinition::Heightfield {
                points_m: projection.mesa.terrain_points_m.clone(),
            },
        )
        .unwrap();
        request.policy.max_outbound_heading_error_rad = 0.1;
        let error = project_case_waypoint_runtime_v2(&request, candidate, crossing).unwrap_err();
        let ExperimentalRidgeCaseRuntimeProjectionErrorV2::HandoffAttemptsFailed(failure) = error
        else {
            panic!("both handoff attempts should be retained at the error boundary");
        };
        assert_eq!(failure.attempts.len(), 2);
        assert_eq!(
            failure.attempts[0].selection_kind,
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::PrimaryCrossing
        );
        assert_eq!(
            failure.attempts[1].selection_kind,
            ExperimentalRidgeRuntimeHandoffSelectionKindV2::IntermediateBridgeExit
        );
        assert_eq!(failure.attempts[0].target_leg_arc_step, None);
        assert_eq!(
            failure.attempts[1].target_leg_arc_step,
            Some(candidate.target_leg.apex_step)
        );
        assert!(!failure.attempts[0].handoff_assessment.contract_pass);
        assert!(!failure.attempts[1].handoff_assessment.contract_pass);
        assert_eq!(
            failure.identity,
            runtime_handoff_failure_identity_v2(&failure)
        );
    }

    #[test]
    fn generic_ridge_case_invalid_derived_mesa_geometry_fails_closed() {
        let mut input = generic_ridge_input();
        // This retains a valid raw probe and policy schema but leaves too
        // little terrain domain after the derived-mesa clearance inset.
        input.policy.minimum_clearance_m = 2_041.0;
        input.identity = experimental_ridge_case_input_identity_v1(&input);
        assert_eq!(
            evaluate_experimental_ridge_case_projection_v1(&input),
            Err(ExperimentalRidgeCandidateErrorV2::DerivedMesaGeometryInvalid)
        );
    }

    #[test]
    fn underpowered_fixture_is_not_certified() {
        let mut fixture = fixture();
        fixture.vehicle.max_thrust_n = 20_000.0;
        let evaluation = evaluate_fixture_v2(&fixture).unwrap();
        assert!(evaluation.results.iter().all(|result| {
            result
                .candidates
                .iter()
                .all(|candidate| candidate.classification == CertificationV2::NotCertified)
        }));
    }
}
