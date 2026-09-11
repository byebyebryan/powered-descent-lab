//! Private CB1 analytical spike.
//!
//! This module is intentionally compiled only with `pd-plan`'s unit tests.
//! It is an input-only property oracle for the four route-necessity canaries;
//! it is not a planner, does not search for waypoint positions, and has no
//! controller or simulation dependencies.  Keeping the experiment private
//! lets CB2 make the public/API decision from evidence rather than from an
//! experimental serialized contract.

use std::cmp::Ordering;

use pd_core::{CorridorEnvelope, TerrainDefinition, Vec2};
use serde::{Deserialize, Serialize};

const FIXTURE: &str = include_str!("../fixtures/conservative_ballistic_cb1_canaries.json");
const SCHEMA_ID: &str = "conservative_ballistic_cb1_canaries_v1";
const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CanaryFixture {
    schema_id: String,
    schema_version: u32,
    policy: FrozenPolicy,
    vehicle: VehicleInput,
    cases: Vec<CanaryCase>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FrozenPolicy {
    physics_hz: u32,
    gravity_mps2: f64,
    duration_multipliers: Vec<f64>,
    source_gate_height_m: f64,
    source_exit_distance_m: f64,
    source_lift_exit_speed_mps: f64,
    intermediate_entry_distance_m: f64,
    intermediate_exit_distance_m: f64,
    terminal_gate_offset_m: f64,
    terminal_gate_height_m: f64,
    terminal_entry_distance_m: f64,
    terminal_state_height_m: f64,
    gate_capture_radius_m: f64,
    corridor_horizontal_extent_m: f64,
    corridor_vertical_extent_m: f64,
    minimum_clearance_m: f64,
    maximum_nonterminal_gate_speed_mps: f64,
    maximum_nonterminal_gate_cross_speed_mps: f64,
    maximum_nonterminal_gate_vertical_speed_mps: f64,
    maximum_terminal_speed_mps: f64,
    maximum_terminal_vertical_speed_mps: f64,
    maximum_terminal_cross_speed_mps: f64,
    terminal_descent_speed_mps: f64,
    maximum_transition_delta_v_mps: f64,
    usable_transition_distance_fraction: f64,
    maximum_mission_time_s: f64,
    mission_time_reserve_s: f64,
    thrust_derate: f64,
    declared_robustness_margin: f64,
    maximum_source_acquisition_burn_time_s: f64,
    maximum_transition_burn_time_s: f64,
    maximum_transition_slew_time_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct VehicleInput {
    geometry: VehicleGeometryInput,
    dry_mass_kg: f64,
    initial_fuel_kg: f64,
    max_fuel_kg: f64,
    max_fuel_burn_kgps: f64,
    max_thrust_n: f64,
    min_throttle_frac: f64,
    max_rotation_rate_radps: f64,
    safe_touchdown_normal_speed_mps: f64,
    safe_touchdown_tangential_speed_mps: f64,
    safe_touchdown_attitude_error_rad: f64,
    safe_touchdown_angular_rate_radps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct VehicleGeometryInput {
    hull_width_m: f64,
    hull_height_m: f64,
    touchdown_half_span_m: f64,
    touchdown_base_offset_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CanaryCase {
    id: String,
    source: PadInput,
    target: PadInput,
    terrain_points_m: Vec<Vec2>,
    initial_position_m: Vec2,
    initial_velocity_mps: Vec2,
    /// A property oracle only.  CB1 evaluates this authored gate directly;
    /// no position candidates are generated or searched here.
    authored_one_gate: Option<Vec2>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct PadInput {
    center_x_m: f64,
    surface_y_m: f64,
    width_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Margin {
    raw: f64,
    normalized: f64,
}

impl Margin {
    fn unbounded() -> Self {
        Self {
            raw: f64::MAX,
            normalized: f64::MAX,
        }
    }

    fn upper(limit: f64, value: f64) -> Self {
        let raw = limit - value;
        Self {
            raw,
            normalized: raw / limit.abs().max(1.0),
        }
    }

    fn lower(value: f64, limit: f64) -> Self {
        let raw = value - limit;
        Self {
            raw,
            normalized: raw / limit.abs().max(1.0),
        }
    }

    fn robust(self, policy: &FrozenPolicy) -> bool {
        self.raw > 0.0 && self.normalized >= policy.declared_robustness_margin
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct ComponentMargins {
    clearance: Margin,
    speed: Margin,
    vertical_speed: Margin,
    cross_speed: Margin,
    transition_delta_v: Margin,
    transition_burn_time: Margin,
    transition_slew_time: Margin,
    transition_distance: Margin,
    fuel: Margin,
    time: Margin,
}

impl ComponentMargins {
    fn minimum_normalized(self) -> f64 {
        [
            self.clearance,
            self.speed,
            self.vertical_speed,
            self.cross_speed,
            self.transition_delta_v,
            self.transition_burn_time,
            self.transition_slew_time,
            self.transition_distance,
            self.fuel,
            self.time,
        ]
        .into_iter()
        .map(|margin| margin.normalized)
        .fold(f64::INFINITY, f64::min)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DecisiveReason {
    Accepted,
    TerrainClearance,
    SourceAcquisition,
    TransitionAuthority,
    TerminalCapture,
    MissionTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct StateGate {
    position_m: Vec2,
    capture_radius_m: f64,
    outbound_tangent_unit: Vec2,
    min_outbound_speed_mps: f64,
    max_outbound_speed_mps: f64,
    max_outbound_cross_speed_mps: f64,
    min_vertical_speed_mps: f64,
    max_vertical_speed_mps: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct BallisticLeg {
    start_m: Vec2,
    end_m: Vec2,
    steps: u64,
    duration_s: f64,
    departure_velocity_mps: Vec2,
    arrival_velocity_mps: Vec2,
    apex_m: Vec2,
    positions_m: Vec<Vec2>,
    minimum_clearance_m: f64,
    clearance_margin: Margin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TransitionAssessment {
    accepted: bool,
    delta_v_mps: f64,
    slew_distance_m: f64,
    braking_distance_m: f64,
    required_distance_m: f64,
    usable_distance_m: f64,
    powered_path: PoweredPath,
    burn_duration_s: f64,
    fuel_burn_kg: f64,
    slew_time_s: f64,
    elapsed_before_s: f64,
    elapsed_after_s: f64,
    fuel_used_before_kg: f64,
    fuel_used_after_kg: f64,
    margins: ComponentMargins,
    decisive_reason: DecisiveReason,
}

struct TransitionInput {
    incoming: Vec2,
    outgoing: Vec2,
    powered_path: PoweredPath,
    elapsed_before_s: f64,
    fuel_used_before_kg: f64,
}

struct TerminalInput {
    incoming: Vec2,
    ballistic_clearance_margin: Margin,
    powered_path: PoweredPath,
    elapsed_time_s: f64,
    fuel_used_kg: f64,
}

/// An explicit powered maneuver region.  Its polyline is the entire spatial
/// budget exposed to the conservative delta-v screen; exact ballistic coasts
/// only meet it at the first/last anchor and never borrow its metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct PoweredPath {
    anchors_m: Vec<Vec2>,
    segment_lengths_m: Vec<f64>,
    path_length_m: f64,
    /// Extra radius around the hull corridor required for this powered zone.
    /// Maneuver paths use the state-gate capture radius; the upright lift is
    /// hull-only because its first anchor is supported pad contact.
    clearance_inflation_m: f64,
    clearance_margin: Margin,
    identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct SourceAssessment {
    accepted: bool,
    lift_exit_velocity_mps: Vec2,
    lift_path: PoweredPath,
    lift_required_distance_m: f64,
    lift_usable_distance_m: f64,
    lift_burn_duration_s: f64,
    lift_fuel_burn_kg: f64,
    redirect: TransitionAssessment,
    elapsed_after_s: f64,
    fuel_used_after_kg: f64,
    margins: ComponentMargins,
    decisive_reason: DecisiveReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TerminalAssessment {
    accepted: bool,
    speed_mps: f64,
    vertical_speed_mps: f64,
    cross_speed_mps: f64,
    transition: TransitionAssessment,
    margins: ComponentMargins,
    decisive_reason: DecisiveReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct DirectCandidate {
    steps: u64,
    leg: BallisticLeg,
    source_acquisition: SourceAssessment,
    terminal_capture: TerminalAssessment,
    accepted: bool,
    decisive_reason: DecisiveReason,
    margins: ComponentMargins,
    identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct OneGateEvidence {
    gate: StateGate,
    source_steps: u64,
    target_steps: u64,
    source_acquisition: SourceAssessment,
    source_leg: BallisticLeg,
    transition: TransitionAssessment,
    target_leg: BallisticLeg,
    terminal_capture: TerminalAssessment,
    accepted: bool,
    decisive_reason: DecisiveReason,
    margins: ComponentMargins,
    identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CanaryResult {
    id: String,
    direct_candidates: Vec<DirectCandidate>,
    accepted_direct_count: usize,
    one_gate_candidates: Vec<OneGateEvidence>,
    accepted_one_gate_count: usize,
    result_identity: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct FixtureEvaluation {
    policy_identity: String,
    results: Vec<CanaryResult>,
    evaluation_identity: String,
}

impl FrozenPolicy {
    fn validate(&self) -> Result<(), String> {
        if self.physics_hz == 0 {
            return Err("physics_hz must be positive".to_owned());
        }
        if !self.gravity_mps2.is_finite() || self.gravity_mps2 <= 0.0 {
            return Err("gravity_mps2 must be positive and finite".to_owned());
        }
        if self.duration_multipliers.is_empty()
            || self
                .duration_multipliers
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err("duration_multipliers must be positive and finite".to_owned());
        }
        for (name, value) in [
            ("source_gate_height_m", self.source_gate_height_m),
            ("source_exit_distance_m", self.source_exit_distance_m),
            (
                "source_lift_exit_speed_mps",
                self.source_lift_exit_speed_mps,
            ),
            (
                "intermediate_entry_distance_m",
                self.intermediate_entry_distance_m,
            ),
            (
                "intermediate_exit_distance_m",
                self.intermediate_exit_distance_m,
            ),
            ("terminal_gate_offset_m", self.terminal_gate_offset_m),
            ("terminal_gate_height_m", self.terminal_gate_height_m),
            ("terminal_entry_distance_m", self.terminal_entry_distance_m),
            ("terminal_state_height_m", self.terminal_state_height_m),
            ("gate_capture_radius_m", self.gate_capture_radius_m),
            (
                "corridor_horizontal_extent_m",
                self.corridor_horizontal_extent_m,
            ),
            (
                "corridor_vertical_extent_m",
                self.corridor_vertical_extent_m,
            ),
            ("minimum_clearance_m", self.minimum_clearance_m),
            (
                "maximum_nonterminal_gate_speed_mps",
                self.maximum_nonterminal_gate_speed_mps,
            ),
            (
                "maximum_nonterminal_gate_cross_speed_mps",
                self.maximum_nonterminal_gate_cross_speed_mps,
            ),
            (
                "maximum_nonterminal_gate_vertical_speed_mps",
                self.maximum_nonterminal_gate_vertical_speed_mps,
            ),
            (
                "maximum_terminal_speed_mps",
                self.maximum_terminal_speed_mps,
            ),
            (
                "maximum_terminal_vertical_speed_mps",
                self.maximum_terminal_vertical_speed_mps,
            ),
            (
                "maximum_terminal_cross_speed_mps",
                self.maximum_terminal_cross_speed_mps,
            ),
            (
                "maximum_transition_delta_v_mps",
                self.maximum_transition_delta_v_mps,
            ),
            (
                "usable_transition_distance_fraction",
                self.usable_transition_distance_fraction,
            ),
            (
                "terminal_descent_speed_mps",
                self.terminal_descent_speed_mps,
            ),
            ("maximum_mission_time_s", self.maximum_mission_time_s),
            ("mission_time_reserve_s", self.mission_time_reserve_s),
            ("thrust_derate", self.thrust_derate),
            (
                "declared_robustness_margin",
                self.declared_robustness_margin,
            ),
            (
                "maximum_source_acquisition_burn_time_s",
                self.maximum_source_acquisition_burn_time_s,
            ),
            (
                "maximum_transition_burn_time_s",
                self.maximum_transition_burn_time_s,
            ),
            (
                "maximum_transition_slew_time_s",
                self.maximum_transition_slew_time_s,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{name} must be positive and finite"));
            }
        }
        if self.terminal_descent_speed_mps > self.maximum_terminal_vertical_speed_mps
            || self.thrust_derate > 1.0
            || self.usable_transition_distance_fraction > 1.0
            || self.declared_robustness_margin >= 1.0
        {
            return Err("policy fractions must be within (0, 1]".to_owned());
        }
        Ok(())
    }
}

impl VehicleInput {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("hull_width_m", self.geometry.hull_width_m),
            ("hull_height_m", self.geometry.hull_height_m),
            ("touchdown_half_span_m", self.geometry.touchdown_half_span_m),
            (
                "touchdown_base_offset_m",
                self.geometry.touchdown_base_offset_m,
            ),
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
                return Err(format!("{name} must be positive and finite"));
            }
        }
        for (name, value) in [
            ("dry_mass_kg", self.dry_mass_kg),
            ("initial_fuel_kg", self.initial_fuel_kg),
            ("max_fuel_kg", self.max_fuel_kg),
            ("max_fuel_burn_kgps", self.max_fuel_burn_kgps),
            ("max_thrust_n", self.max_thrust_n),
            ("max_rotation_rate_radps", self.max_rotation_rate_radps),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{name} must be positive and finite"));
            }
        }
        if self.initial_fuel_kg > self.max_fuel_kg {
            return Err("initial fuel must not exceed max fuel".to_owned());
        }
        if !(0.0..=1.0).contains(&self.min_throttle_frac) {
            return Err("min_throttle_frac must be within [0, 1]".to_owned());
        }
        Ok(())
    }
}

impl CanaryFixture {
    fn validate(&self) -> Result<(), String> {
        if self.schema_id != SCHEMA_ID || self.schema_version != SCHEMA_VERSION {
            return Err("unexpected CB1 fixture schema".to_owned());
        }
        self.policy.validate()?;
        self.vehicle.validate()?;
        let conservative_net_acceleration = self.policy.thrust_derate * self.vehicle.max_thrust_n
            / (self.vehicle.dry_mass_kg + self.vehicle.max_fuel_kg)
            - self.policy.gravity_mps2;
        if !conservative_net_acceleration.is_finite() || conservative_net_acceleration <= 0.0 {
            return Err("conservative net acceleration must be positive and finite".to_owned());
        }
        let mut ids = std::collections::BTreeSet::new();
        for case in &self.cases {
            if !ids.insert(case.id.clone()) {
                return Err(format!("duplicate canary id {}", case.id));
            }
            case.validate(&self.policy, &self.vehicle)?;
        }
        let expected = [
            "clear_direct_control",
            "long_span_capture_split",
            "late_ridge_capture_split",
            "insufficient_authority_control",
        ];
        if self.cases.len() != expected.len() || expected.iter().any(|id| !ids.contains(*id)) {
            return Err("fixture must contain exactly the four CB1 canaries".to_owned());
        }
        Ok(())
    }
}

impl CanaryCase {
    fn validate(&self, policy: &FrozenPolicy, vehicle: &VehicleInput) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("canary id must not be empty".to_owned());
        }
        for (name, value) in [
            ("source center", self.source.center_x_m),
            ("source surface", self.source.surface_y_m),
            ("source width", self.source.width_m),
            ("target center", self.target.center_x_m),
            ("target surface", self.target.surface_y_m),
            ("target width", self.target.width_m),
        ] {
            if !value.is_finite() {
                return Err(format!("{name} must be finite"));
            }
        }
        if self.source.width_m <= 0.0 || self.target.width_m <= 0.0 {
            return Err("pad widths must be positive".to_owned());
        }
        if self.target.center_x_m <= self.source.center_x_m {
            return Err("canary pads must progress forward".to_owned());
        }
        if self.terrain_points_m.len() < 2 {
            return Err("canary terrain needs at least two points".to_owned());
        }
        TerrainDefinition::Heightfield {
            points_m: self.terrain_points_m.clone(),
        }
        .validate()?;
        let terrain = terrain(self);
        let (domain_min_x_m, domain_max_x_m) = (
            self.terrain_points_m.first().expect("validated terrain").x,
            self.terrain_points_m.last().expect("validated terrain").x,
        );
        for (name, pad) in [("source", &self.source), ("target", &self.target)] {
            let left = pad.center_x_m - pad.width_m * 0.5;
            let right = pad.center_x_m + pad.width_m * 0.5;
            if left < domain_min_x_m || right > domain_max_x_m {
                return Err(format!("{name} pad footprint lies outside terrain domain"));
            }
            for x in [left, right] {
                let terrain_y = terrain
                    .sample_height_strict(x)
                    .map_err(|error| error.to_string())?;
                if (terrain_y - pad.surface_y_m).abs() > 1.0e-9 {
                    return Err(format!("{name} pad is not on flat terrain"));
                }
            }
        }
        for point in [self.initial_position_m, self.initial_velocity_mps] {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err("initial state must be finite".to_owned());
            }
        }
        let expected_source = Vec2::new(
            self.source.center_x_m,
            self.source.surface_y_m + vehicle.geometry.touchdown_base_offset_m,
        );
        if (self.initial_position_m - expected_source).length() > 1.0e-9 {
            return Err("initial position must be the source touchdown reference".to_owned());
        }
        if self.initial_velocity_mps.length() > 1.0e-9 {
            return Err("initial velocity must be the source touchdown rest state".to_owned());
        }
        if let Some(gate) = self.authored_one_gate
            && (!gate.x.is_finite()
                || !gate.y.is_finite()
                || gate.x <= self.source.center_x_m
                || gate.x >= self.target.center_x_m - policy.terminal_gate_offset_m)
        {
            return Err("authored gate must be strictly inside route".to_owned());
        }
        Ok(())
    }
}

impl StateGate {
    fn validate(&self) -> Result<(), String> {
        if self.capture_radius_m <= 0.0
            || !self.capture_radius_m.is_finite()
            || self.min_outbound_speed_mps < 0.0
            || self.max_outbound_speed_mps <= self.min_outbound_speed_mps
            || self.max_outbound_cross_speed_mps <= 0.0
            || self.min_vertical_speed_mps > self.max_vertical_speed_mps
        {
            return Err("invalid state gate bounds".to_owned());
        }
        Ok(())
    }
}

fn parse_fixture() -> CanaryFixture {
    let fixture: CanaryFixture = serde_json::from_str(FIXTURE).expect("valid CB1 fixture");
    fixture.validate().expect("valid CB1 fixture contract");
    fixture
}

fn digest<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("CB1 identity serialization");
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn terrain(case: &CanaryCase) -> TerrainDefinition {
    TerrainDefinition::Heightfield {
        points_m: case.terrain_points_m.clone(),
    }
}

fn gate_policy(
    policy: &FrozenPolicy,
    case: &CanaryCase,
    vehicle: &VehicleInput,
) -> (StateGate, StateGate) {
    let source = StateGate {
        position_m: Vec2::new(
            case.source.center_x_m,
            case.source.surface_y_m + policy.source_gate_height_m,
        ),
        capture_radius_m: policy.gate_capture_radius_m,
        outbound_tangent_unit: Vec2::new(1.0, 0.0),
        min_outbound_speed_mps: 0.0,
        max_outbound_speed_mps: policy.maximum_nonterminal_gate_speed_mps,
        max_outbound_cross_speed_mps: policy.maximum_nonterminal_gate_cross_speed_mps,
        min_vertical_speed_mps: -policy.maximum_nonterminal_gate_vertical_speed_mps,
        max_vertical_speed_mps: policy.maximum_nonterminal_gate_vertical_speed_mps,
    };
    let target_x = case.target.center_x_m - policy.terminal_gate_offset_m;
    let target = StateGate {
        position_m: Vec2::new(
            target_x,
            case.target.surface_y_m + policy.terminal_gate_height_m,
        ),
        capture_radius_m: policy.gate_capture_radius_m,
        // Terminal capture is a vertical descent gate; horizontal speed is
        // therefore a real cross-track component of this tangent frame.
        outbound_tangent_unit: Vec2::new(0.0, -1.0),
        min_outbound_speed_mps: 0.0,
        max_outbound_speed_mps: policy.maximum_terminal_speed_mps,
        max_outbound_cross_speed_mps: policy.maximum_terminal_cross_speed_mps,
        min_vertical_speed_mps: -policy.maximum_terminal_vertical_speed_mps,
        max_vertical_speed_mps: policy.maximum_terminal_vertical_speed_mps,
    };
    let _ = vehicle;
    (source, target)
}

fn candidate_steps(policy: &FrozenPolicy, horizontal_span_m: f64) -> Vec<u64> {
    let nominal_s = (2.0 * horizontal_span_m / policy.gravity_mps2).sqrt();
    let mut steps: Vec<u64> = policy
        .duration_multipliers
        .iter()
        .map(|multiplier| {
            (nominal_s * multiplier * f64::from(policy.physics_hz))
                .round()
                .max(1.0) as u64
        })
        .collect();
    steps.sort_unstable();
    steps.dedup();
    steps
}

fn ballistic_leg(
    policy: &FrozenPolicy,
    case: &CanaryCase,
    start: Vec2,
    end: Vec2,
    steps: u64,
) -> Result<BallisticLeg, DecisiveReason> {
    if end.x <= start.x || steps == 0 {
        return Err(DecisiveReason::TerrainClearance);
    }
    let dt = 1.0 / f64::from(policy.physics_hz);
    let n = steps as f64;
    let duration_s = n * dt;
    let departure_velocity_mps = Vec2::new(
        (end.x - start.x) / duration_s,
        (end.y - start.y) / duration_s + 0.5 * policy.gravity_mps2 * dt * (n + 1.0),
    );
    let arrival_velocity_mps = Vec2::new(
        departure_velocity_mps.x,
        departure_velocity_mps.y - policy.gravity_mps2 * duration_s,
    );
    let positions_m: Vec<Vec2> = (0..=steps)
        .map(|step| {
            let k = step as f64;
            Vec2::new(
                start.x + k * dt * departure_velocity_mps.x,
                start.y + k * dt * departure_velocity_mps.y
                    - 0.5 * policy.gravity_mps2 * dt * dt * k * (k + 1.0),
            )
        })
        .collect();
    let apex_m = positions_m
        .iter()
        .copied()
        .max_by(|left, right| left.y.total_cmp(&right.y))
        .unwrap_or(start);
    let terrain = terrain(case);
    let envelope = CorridorEnvelope::new(
        policy.corridor_horizontal_extent_m,
        policy.corridor_vertical_extent_m,
    );
    let mut minimum_clearance_m = f64::INFINITY;
    for pair in positions_m.windows(2) {
        let clearance = terrain
            .exact_corridor_clearance(pair[0], pair[1], envelope, envelope)
            .map_err(|_| DecisiveReason::TerrainClearance)?;
        minimum_clearance_m = minimum_clearance_m.min(clearance.minimum_clearance_m);
    }
    let clearance_margin = Margin::lower(minimum_clearance_m, policy.minimum_clearance_m);
    Ok(BallisticLeg {
        start_m: start,
        end_m: end,
        steps,
        duration_s,
        departure_velocity_mps,
        arrival_velocity_mps,
        apex_m,
        positions_m,
        minimum_clearance_m,
        clearance_margin,
    })
}

fn speed_components(velocity: Vec2, tangent: Vec2) -> (f64, f64, f64) {
    let tangent = normalize(tangent);
    let progress = velocity.x.mul_add(tangent.x, velocity.y * tangent.y);
    let cross = (velocity.x * tangent.y - velocity.y * tangent.x).abs();
    (velocity.length(), progress, cross)
}

fn normalize(vector: Vec2) -> Vec2 {
    let length = vector.length();
    if length <= 1.0e-12 {
        Vec2::new(0.0, 0.0)
    } else {
        vector * (1.0 / length)
    }
}

fn acceleration(policy: &FrozenPolicy, vehicle: &VehicleInput) -> (f64, f64) {
    let mass = vehicle.dry_mass_kg + vehicle.max_fuel_kg;
    let thrust_acceleration = policy.thrust_derate * vehicle.max_thrust_n / mass;
    (
        thrust_acceleration,
        thrust_acceleration - policy.gravity_mps2,
    )
}

fn transition_corridor_clearance(
    policy: &FrozenPolicy,
    case: &CanaryCase,
    start_m: Vec2,
    end_m: Vec2,
    extra_radius_m: f64,
) -> Margin {
    let envelope = CorridorEnvelope::new(
        policy.corridor_horizontal_extent_m + extra_radius_m,
        policy.corridor_vertical_extent_m + extra_radius_m,
    );
    let terrain = terrain(case);
    let clearance = if (end_m.x - start_m.x).abs() <= f64::EPSILON {
        terrain.exact_point_clearance(start_m, envelope)
    } else {
        terrain.exact_corridor_clearance(start_m, end_m, envelope, envelope)
    };
    Margin::lower(
        clearance
            .map(|value| value.minimum_clearance_m)
            .unwrap_or(f64::NEG_INFINITY),
        policy.minimum_clearance_m,
    )
}

fn powered_path(
    policy: &FrozenPolicy,
    case: &CanaryCase,
    anchors_m: Vec<Vec2>,
    clearance_inflation_m: f64,
) -> PoweredPath {
    assert!(
        anchors_m.len() >= 2,
        "powered path needs at least two anchors"
    );
    let segment_lengths_m: Vec<f64> = anchors_m
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).length())
        .collect();
    assert!(
        segment_lengths_m.iter().all(|length| *length > 0.0),
        "powered path anchors must be distinct"
    );
    assert!(
        clearance_inflation_m.is_finite() && clearance_inflation_m >= 0.0,
        "powered path clearance inflation must be finite and non-negative"
    );
    let clearance_margin = anchors_m
        .windows(2)
        .map(|pair| {
            transition_corridor_clearance(policy, case, pair[0], pair[1], clearance_inflation_m)
        })
        .fold(Margin::unbounded(), MarginMin::min_by);
    let mut path = PoweredPath {
        path_length_m: segment_lengths_m.iter().sum(),
        anchors_m,
        segment_lengths_m,
        clearance_inflation_m,
        clearance_margin,
        identity: String::new(),
    };
    path.identity = digest(&PoweredPathIdentity {
        anchors_m: &path.anchors_m,
        segment_lengths_m: &path.segment_lengths_m,
        path_length_m: path.path_length_m,
        clearance_inflation_m: path.clearance_inflation_m,
        clearance_margin: path.clearance_margin,
    });
    path
}

fn source_lift_path(policy: &FrozenPolicy, case: &CanaryCase, source: &StateGate) -> PoweredPath {
    let mut path = powered_path(
        policy,
        case,
        vec![case.initial_position_m, source.position_m],
        0.0,
    );
    // The first anchor is the supported pad-touchdown reference, not an
    // airborne clearance state.  The source pad is separately validated as
    // flat support; free-flight corridor clearance begins at the first height
    // that clears the maneuver envelope and declared margin.  Nothing after
    // that release point is exempted.
    let release_m = Vec2::new(
        case.initial_position_m.x,
        case.source.surface_y_m
            + policy.corridor_vertical_extent_m
            + policy.minimum_clearance_m * (1.0 + 2.0 * policy.declared_robustness_margin),
    );
    path.clearance_margin =
        transition_corridor_clearance(policy, case, release_m, source.position_m, 0.0);
    path.identity = digest(&PoweredPathIdentity {
        anchors_m: &path.anchors_m,
        segment_lengths_m: &path.segment_lengths_m,
        path_length_m: path.path_length_m,
        clearance_inflation_m: path.clearance_inflation_m,
        clearance_margin: path.clearance_margin,
    });
    path
}

#[derive(Serialize)]
struct PoweredPathIdentity<'a> {
    anchors_m: &'a [Vec2],
    segment_lengths_m: &'a [f64],
    path_length_m: f64,
    clearance_inflation_m: f64,
    clearance_margin: Margin,
}

fn valid_powered_path(path: &PoweredPath) -> bool {
    path.identity
        == digest(&PoweredPathIdentity {
            anchors_m: &path.anchors_m,
            segment_lengths_m: &path.segment_lengths_m,
            path_length_m: path.path_length_m,
            clearance_inflation_m: path.clearance_inflation_m,
            clearance_margin: path.clearance_margin,
        })
        && path.anchors_m.len() == path.segment_lengths_m.len() + 1
        && path.segment_lengths_m.iter().all(|length| *length > 0.0)
        && path.clearance_inflation_m.is_finite()
        && path.clearance_inflation_m >= 0.0
        && (path.segment_lengths_m.iter().sum::<f64>() - path.path_length_m).abs() < 1.0e-9
}

fn transition(
    policy: &FrozenPolicy,
    vehicle: &VehicleInput,
    input: TransitionInput,
) -> TransitionAssessment {
    // This is a conservative maneuver-room screen, not command integration:
    // it proves that the explicit, exactly-cleared powered polyline has room
    // for the bounded slew/redirect estimate, not an exact traversal of it.
    let (_thrust_acceleration, net_acceleration) = acceleration(policy, vehicle);
    assert!(valid_powered_path(&input.powered_path));
    let delta_v_mps = (input.outgoing - input.incoming).length();
    let angle = if input.incoming.length() <= 1.0e-9 || input.outgoing.length() <= 1.0e-9 {
        0.0
    } else {
        (normalize(input.incoming).x * normalize(input.outgoing).x
            + normalize(input.incoming).y * normalize(input.outgoing).y)
            .clamp(-1.0, 1.0)
            .acos()
    };
    let slew_time_s = angle / vehicle.max_rotation_rate_radps;
    let slew_distance_m = input.incoming.length().max(input.outgoing.length()) * slew_time_s;
    let braking_distance_m = if net_acceleration > 0.0 {
        delta_v_mps * delta_v_mps / (2.0 * net_acceleration)
    } else {
        f64::INFINITY
    };
    let required_distance_m = slew_distance_m + braking_distance_m;
    let usable_distance_m =
        input.powered_path.path_length_m * policy.usable_transition_distance_fraction;
    let burn_duration_s = if net_acceleration > 0.0 {
        delta_v_mps / net_acceleration
    } else {
        f64::INFINITY
    };
    let fuel_burn_kg = burn_duration_s * vehicle.max_fuel_burn_kgps;
    let elapsed_after_s = input.elapsed_before_s + slew_time_s + burn_duration_s;
    let fuel_used_after_kg = input.fuel_used_before_kg + fuel_burn_kg;
    let margins = ComponentMargins {
        clearance: input.powered_path.clearance_margin,
        speed: Margin::unbounded(),
        vertical_speed: Margin::unbounded(),
        cross_speed: Margin::unbounded(),
        transition_delta_v: Margin::upper(policy.maximum_transition_delta_v_mps, delta_v_mps),
        transition_burn_time: Margin::upper(policy.maximum_transition_burn_time_s, burn_duration_s),
        transition_slew_time: Margin::upper(policy.maximum_transition_slew_time_s, slew_time_s),
        transition_distance: Margin::lower(usable_distance_m, required_distance_m),
        fuel: Margin::lower(vehicle.initial_fuel_kg, fuel_used_after_kg),
        time: Margin::lower(
            policy.maximum_mission_time_s - policy.mission_time_reserve_s,
            elapsed_after_s,
        ),
    };
    let decisive_reason = if !margins.transition_delta_v.robust(policy)
        || !margins.transition_burn_time.robust(policy)
        || !margins.transition_slew_time.robust(policy)
        || !margins.transition_distance.robust(policy)
        || !margins.fuel.robust(policy)
        || !margins.clearance.robust(policy)
    {
        DecisiveReason::TransitionAuthority
    } else if !margins.time.robust(policy) {
        DecisiveReason::MissionTime
    } else {
        DecisiveReason::Accepted
    };
    TransitionAssessment {
        accepted: decisive_reason == DecisiveReason::Accepted,
        delta_v_mps,
        slew_distance_m,
        braking_distance_m,
        required_distance_m,
        usable_distance_m,
        powered_path: input.powered_path,
        burn_duration_s,
        fuel_burn_kg,
        slew_time_s,
        elapsed_before_s: input.elapsed_before_s,
        elapsed_after_s,
        fuel_used_before_kg: input.fuel_used_before_kg,
        fuel_used_after_kg,
        margins,
        decisive_reason,
    }
}

fn source_acquisition(
    policy: &FrozenPolicy,
    vehicle: &VehicleInput,
    case: &CanaryCase,
    source: &StateGate,
    source_exit_m: Vec2,
    outgoing: Vec2,
) -> SourceAssessment {
    let (_thrust_acceleration, net_acceleration) = acceleration(policy, vehicle);
    let lift_path = source_lift_path(policy, case, source);
    let lift_exit_velocity_mps = Vec2::new(0.0, policy.source_lift_exit_speed_mps);
    let lift_required_distance_m = if net_acceleration > 0.0 {
        policy.source_lift_exit_speed_mps.powi(2) / (2.0 * net_acceleration)
    } else {
        f64::INFINITY
    };
    let lift_usable_distance_m =
        lift_path.path_length_m * policy.usable_transition_distance_fraction;
    let lift_burn_duration_s = if net_acceleration > 0.0 {
        policy.source_lift_exit_speed_mps / net_acceleration
    } else {
        f64::INFINITY
    };
    let lift_fuel_burn_kg = lift_burn_duration_s * vehicle.max_fuel_burn_kgps;
    let redirect = transition(
        policy,
        vehicle,
        TransitionInput {
            incoming: lift_exit_velocity_mps,
            outgoing,
            powered_path: powered_path(
                policy,
                case,
                vec![source.position_m, source_exit_m],
                policy.gate_capture_radius_m,
            ),
            elapsed_before_s: lift_burn_duration_s,
            fuel_used_before_kg: lift_fuel_burn_kg,
        },
    );
    let lift_distance_margin = Margin::lower(lift_usable_distance_m, lift_required_distance_m);
    let mut margins = redirect.margins;
    margins.clearance = margins.clearance.min_by(lift_path.clearance_margin);
    margins.transition_distance = margins.transition_distance.min_by(lift_distance_margin);
    margins.transition_burn_time = Margin::upper(
        policy.maximum_source_acquisition_burn_time_s,
        redirect.elapsed_after_s,
    );
    let decisive_reason = if !lift_path.clearance_margin.robust(policy)
        || !lift_distance_margin.robust(policy)
        || !redirect.accepted
        || !margins.transition_burn_time.robust(policy)
    {
        DecisiveReason::SourceAcquisition
    } else if !margins.time.robust(policy) {
        DecisiveReason::MissionTime
    } else {
        DecisiveReason::Accepted
    };
    SourceAssessment {
        accepted: decisive_reason == DecisiveReason::Accepted,
        lift_exit_velocity_mps,
        lift_path,
        lift_required_distance_m,
        lift_usable_distance_m,
        lift_burn_duration_s,
        lift_fuel_burn_kg,
        elapsed_after_s: redirect.elapsed_after_s,
        fuel_used_after_kg: redirect.fuel_used_after_kg,
        redirect,
        margins,
        decisive_reason,
    }
}

fn gate_outbound_assessment(
    gate: &StateGate,
    velocity: Vec2,
    clearance_margin: Margin,
) -> ComponentMargins {
    let (speed, progress, cross) = speed_components(velocity, gate.outbound_tangent_unit);
    let vertical_speed = Margin::lower(velocity.y, gate.min_vertical_speed_mps)
        .min_by(Margin::upper(gate.max_vertical_speed_mps, velocity.y));
    ComponentMargins {
        clearance: clearance_margin,
        speed: Margin::upper(gate.max_outbound_speed_mps, speed),
        vertical_speed,
        cross_speed: Margin::upper(gate.max_outbound_cross_speed_mps, cross),
        transition_delta_v: Margin::lower(progress, gate.min_outbound_speed_mps),
        transition_burn_time: Margin::unbounded(),
        transition_slew_time: Margin::unbounded(),
        transition_distance: Margin::unbounded(),
        fuel: Margin::unbounded(),
        time: Margin::unbounded(),
    }
}

fn terminal_capture(
    policy: &FrozenPolicy,
    vehicle: &VehicleInput,
    case: &CanaryCase,
    gate: &StateGate,
    input: TerminalInput,
) -> TerminalAssessment {
    let terminal_velocity_mps = Vec2::new(0.0, -policy.terminal_descent_speed_mps);
    assert!(valid_powered_path(&input.powered_path));
    assert_eq!(
        input.powered_path.anchors_m.last().copied(),
        Some(Vec2::new(
            case.target.center_x_m,
            case.target.surface_y_m + policy.terminal_state_height_m,
        )),
        "terminal path must end at the declared slow terminal state"
    );
    let transition = transition(
        policy,
        vehicle,
        TransitionInput {
            incoming: input.incoming,
            outgoing: terminal_velocity_mps,
            powered_path: input.powered_path,
            elapsed_before_s: input.elapsed_time_s,
            fuel_used_before_kg: input.fuel_used_kg,
        },
    );
    let (speed, progress, terminal_cross) =
        speed_components(terminal_velocity_mps, gate.outbound_tangent_unit);
    let vertical_speed = Margin::lower(terminal_velocity_mps.y, gate.min_vertical_speed_mps)
        .min_by(Margin::upper(
            gate.max_vertical_speed_mps,
            terminal_velocity_mps.y,
        ))
        .min_by(Margin::lower(progress, gate.min_outbound_speed_mps));
    let mut margins = transition.margins;
    margins.clearance = margins.clearance.min_by(input.ballistic_clearance_margin);
    margins.speed = Margin::upper(gate.max_outbound_speed_mps, speed);
    margins.vertical_speed = vertical_speed;
    margins.cross_speed = Margin::upper(gate.max_outbound_cross_speed_mps, terminal_cross);
    let decisive_reason = if !transition.accepted {
        if !transition.margins.time.robust(policy) {
            DecisiveReason::MissionTime
        } else {
            DecisiveReason::TerminalCapture
        }
    } else if !margins.speed.robust(policy)
        || !margins.vertical_speed.robust(policy)
        || !margins.cross_speed.robust(policy)
    {
        DecisiveReason::TerminalCapture
    } else {
        DecisiveReason::Accepted
    };
    TerminalAssessment {
        accepted: decisive_reason == DecisiveReason::Accepted,
        speed_mps: speed,
        vertical_speed_mps: -terminal_velocity_mps.y,
        cross_speed_mps: terminal_cross,
        transition,
        margins,
        decisive_reason,
    }
}

fn source_exit_anchor(policy: &FrozenPolicy, source: &StateGate) -> Vec2 {
    source.position_m + Vec2::new(policy.source_exit_distance_m, 0.0)
}

fn terminal_powered_path(
    policy: &FrozenPolicy,
    case: &CanaryCase,
    target: &StateGate,
    previous_anchor_m: Vec2,
) -> PoweredPath {
    let approach = normalize(target.position_m - previous_anchor_m);
    assert!(
        approach.x > 0.0,
        "terminal approach must make forward progress"
    );
    let entry_m = target.position_m - approach * policy.terminal_entry_distance_m;
    let terminal_state_m = Vec2::new(
        case.target.center_x_m,
        case.target.surface_y_m + policy.terminal_state_height_m,
    );
    powered_path(
        policy,
        case,
        vec![entry_m, target.position_m, terminal_state_m],
        policy.gate_capture_radius_m,
    )
}

struct DirectLayout {
    source_exit_m: Vec2,
    terminal_path: PoweredPath,
}

fn direct_layout(
    policy: &FrozenPolicy,
    case: &CanaryCase,
    source: &StateGate,
    target: &StateGate,
) -> DirectLayout {
    let source_exit_m = source_exit_anchor(policy, source);
    let terminal_path = terminal_powered_path(policy, case, target, source_exit_m);
    assert!(
        terminal_path.anchors_m[0].x > source_exit_m.x,
        "direct coast must have non-overlapping forward anchors"
    );
    DirectLayout {
        source_exit_m,
        terminal_path,
    }
}

struct OneGateLayout {
    source_exit_m: Vec2,
    redirect_path: PoweredPath,
    terminal_path: PoweredPath,
}

fn one_gate_layout(
    policy: &FrozenPolicy,
    case: &CanaryCase,
    source: &StateGate,
    target: &StateGate,
    gate_position_m: Vec2,
) -> OneGateLayout {
    let source_exit_m = source_exit_anchor(policy, source);
    let terminal_path = terminal_powered_path(policy, case, target, gate_position_m);
    let entry_direction = normalize(gate_position_m - source_exit_m);
    let exit_direction = normalize(terminal_path.anchors_m[0] - gate_position_m);
    assert!(
        entry_direction.x > 0.0 && exit_direction.x > 0.0,
        "one-gate powered zone must have forward neighboring geometry"
    );
    let entry_m = gate_position_m - entry_direction * policy.intermediate_entry_distance_m;
    let exit_m = gate_position_m + exit_direction * policy.intermediate_exit_distance_m;
    assert!(
        source_exit_m.x < entry_m.x
            && entry_m.x < gate_position_m.x
            && gate_position_m.x < exit_m.x
            && exit_m.x < terminal_path.anchors_m[0].x,
        "powered and coast anchors must be strictly forward and disjoint"
    );
    OneGateLayout {
        source_exit_m,
        redirect_path: powered_path(
            policy,
            case,
            vec![entry_m, gate_position_m, exit_m],
            policy.gate_capture_radius_m,
        ),
        terminal_path,
    }
}

fn direct_candidate(
    policy: &FrozenPolicy,
    vehicle: &VehicleInput,
    case: &CanaryCase,
    source: &StateGate,
    target: &StateGate,
    steps: u64,
) -> DirectCandidate {
    let layout = direct_layout(policy, case, source, target);
    let leg = ballistic_leg(
        policy,
        case,
        layout.source_exit_m,
        layout.terminal_path.anchors_m[0],
        steps,
    )
    .expect("CB1 fixture has valid forward direct legs");
    let source_acquisition = source_acquisition(
        policy,
        vehicle,
        case,
        source,
        layout.source_exit_m,
        leg.departure_velocity_mps,
    );
    let source_margins =
        gate_outbound_assessment(source, leg.departure_velocity_mps, leg.clearance_margin);
    let terminal_capture = terminal_capture(
        policy,
        vehicle,
        case,
        target,
        TerminalInput {
            incoming: leg.arrival_velocity_mps,
            ballistic_clearance_margin: leg.clearance_margin,
            powered_path: layout.terminal_path,
            elapsed_time_s: source_acquisition.elapsed_after_s + leg.duration_s,
            fuel_used_kg: source_acquisition.fuel_used_after_kg,
        },
    );
    let mut margins = source_margins;
    for component in [source_acquisition.margins, terminal_capture.margins] {
        margins.clearance = margins.clearance.min_by(component.clearance);
        margins.speed = margins.speed.min_by(component.speed);
        margins.vertical_speed = margins.vertical_speed.min_by(component.vertical_speed);
        margins.cross_speed = margins.cross_speed.min_by(component.cross_speed);
        margins.transition_delta_v = margins
            .transition_delta_v
            .min_by(component.transition_delta_v);
        margins.transition_burn_time = margins
            .transition_burn_time
            .min_by(component.transition_burn_time);
        margins.transition_slew_time = margins
            .transition_slew_time
            .min_by(component.transition_slew_time);
        margins.transition_distance = margins
            .transition_distance
            .min_by(component.transition_distance);
        margins.fuel = margins.fuel.min_by(component.fuel);
        margins.time = margins.time.min_by(component.time);
    }
    margins.clearance = margins.clearance.min_by(leg.clearance_margin);
    let decisive_reason = if !leg.clearance_margin.robust(policy) {
        DecisiveReason::TerrainClearance
    } else if !terminal_capture.accepted {
        terminal_capture.decisive_reason
    } else if !source_acquisition.accepted
        || !source_margins.speed.robust(policy)
        || !source_margins.vertical_speed.robust(policy)
        || !source_margins.cross_speed.robust(policy)
        || !source_margins.transition_delta_v.robust(policy)
    {
        DecisiveReason::SourceAcquisition
    } else if !source_acquisition.margins.time.robust(policy)
        || !terminal_capture.margins.time.robust(policy)
    {
        DecisiveReason::MissionTime
    } else {
        DecisiveReason::Accepted
    };
    let accepted = decisive_reason == DecisiveReason::Accepted;
    let mut result = DirectCandidate {
        steps,
        leg,
        source_acquisition,
        terminal_capture,
        accepted,
        decisive_reason,
        margins,
        identity: String::new(),
    };
    result.identity = digest(&DirectIdentity {
        steps: result.steps,
        leg: &result.leg,
        source_acquisition: &result.source_acquisition,
        terminal_capture: &result.terminal_capture,
        accepted: result.accepted,
        decisive_reason: result.decisive_reason,
        margins: result.margins,
    });
    result
}

#[derive(Serialize)]
struct DirectIdentity<'a> {
    steps: u64,
    leg: &'a BallisticLeg,
    source_acquisition: &'a SourceAssessment,
    terminal_capture: &'a TerminalAssessment,
    accepted: bool,
    decisive_reason: DecisiveReason,
    margins: ComponentMargins,
}

fn one_gate_evidence(
    policy: &FrozenPolicy,
    vehicle: &VehicleInput,
    case: &CanaryCase,
    gate_position: Vec2,
    source_steps: u64,
    target_steps: u64,
) -> OneGateEvidence {
    let (source, target) = gate_policy(policy, case, vehicle);
    source.validate().expect("CB1 source gate validates");
    target.validate().expect("CB1 target gate validates");
    let gate = StateGate {
        position_m: gate_position,
        capture_radius_m: policy.gate_capture_radius_m,
        outbound_tangent_unit: normalize(target.position_m - gate_position),
        min_outbound_speed_mps: 0.0,
        max_outbound_speed_mps: policy.maximum_nonterminal_gate_speed_mps,
        max_outbound_cross_speed_mps: policy.maximum_nonterminal_gate_cross_speed_mps,
        min_vertical_speed_mps: -policy.maximum_nonterminal_gate_vertical_speed_mps,
        max_vertical_speed_mps: policy.maximum_nonterminal_gate_vertical_speed_mps,
    };
    gate.validate().expect("authored CB1 gate validates");
    let layout = one_gate_layout(policy, case, &source, &target, gate_position);
    let source_leg = ballistic_leg(
        policy,
        case,
        layout.source_exit_m,
        layout.redirect_path.anchors_m[0],
        source_steps,
    )
    .expect("authored source leg validates");
    let target_leg = ballistic_leg(
        policy,
        case,
        *layout
            .redirect_path
            .anchors_m
            .last()
            .expect("redirect exit"),
        layout.terminal_path.anchors_m[0],
        target_steps,
    )
    .expect("authored target leg validates");
    let source_acquisition = source_acquisition(
        policy,
        vehicle,
        case,
        &source,
        layout.source_exit_m,
        source_leg.departure_velocity_mps,
    );
    let source_elapsed_after_s = source_acquisition.elapsed_after_s + source_leg.duration_s;
    let source_fuel_after_kg = source_acquisition.fuel_used_after_kg;
    let transition = transition(
        policy,
        vehicle,
        TransitionInput {
            incoming: source_leg.arrival_velocity_mps,
            outgoing: target_leg.departure_velocity_mps,
            powered_path: layout.redirect_path,
            elapsed_before_s: source_elapsed_after_s,
            fuel_used_before_kg: source_fuel_after_kg,
        },
    );
    let source_gate_margins = gate_outbound_assessment(
        &source,
        source_leg.departure_velocity_mps,
        source_leg.clearance_margin,
    );
    let intermediate_margins = gate_outbound_assessment(
        &gate,
        target_leg.departure_velocity_mps,
        target_leg.clearance_margin,
    );
    let terminal = terminal_capture(
        policy,
        vehicle,
        case,
        &target,
        TerminalInput {
            incoming: target_leg.arrival_velocity_mps,
            ballistic_clearance_margin: target_leg.clearance_margin,
            powered_path: layout.terminal_path,
            elapsed_time_s: transition.elapsed_after_s + target_leg.duration_s,
            fuel_used_kg: transition.fuel_used_after_kg,
        },
    );
    let mut margins = source_gate_margins;
    for candidate in [
        source_acquisition.margins,
        intermediate_margins,
        transition.margins,
        terminal.margins,
    ] {
        margins.clearance = margins.clearance.min_by(candidate.clearance);
        margins.speed = margins.speed.min_by(candidate.speed);
        margins.vertical_speed = margins.vertical_speed.min_by(candidate.vertical_speed);
        margins.cross_speed = margins.cross_speed.min_by(candidate.cross_speed);
        margins.transition_delta_v = margins
            .transition_delta_v
            .min_by(candidate.transition_delta_v);
        margins.transition_burn_time = margins
            .transition_burn_time
            .min_by(candidate.transition_burn_time);
        margins.transition_slew_time = margins
            .transition_slew_time
            .min_by(candidate.transition_slew_time);
        margins.transition_distance = margins
            .transition_distance
            .min_by(candidate.transition_distance);
        margins.fuel = margins.fuel.min_by(candidate.fuel);
        margins.time = margins.time.min_by(candidate.time);
    }
    let accepted = source_leg.clearance_margin.robust(policy)
        && target_leg.clearance_margin.robust(policy)
        && source_acquisition.accepted
        && source_gate_margins.speed.robust(policy)
        && source_gate_margins.vertical_speed.robust(policy)
        && source_gate_margins.cross_speed.robust(policy)
        && source_gate_margins.transition_delta_v.robust(policy)
        && transition.accepted
        && intermediate_margins.speed.robust(policy)
        && intermediate_margins.vertical_speed.robust(policy)
        && intermediate_margins.cross_speed.robust(policy)
        && intermediate_margins.transition_delta_v.robust(policy)
        && terminal.accepted;
    let decisive_reason = if !source_leg.clearance_margin.robust(policy)
        || !target_leg.clearance_margin.robust(policy)
    {
        DecisiveReason::TerrainClearance
    } else if !source_acquisition.accepted
        || !source_gate_margins.speed.robust(policy)
        || !source_gate_margins.vertical_speed.robust(policy)
        || !source_gate_margins.cross_speed.robust(policy)
        || !source_gate_margins.transition_delta_v.robust(policy)
    {
        DecisiveReason::SourceAcquisition
    } else if !transition.accepted
        || !intermediate_margins.speed.robust(policy)
        || !intermediate_margins.vertical_speed.robust(policy)
        || !intermediate_margins.cross_speed.robust(policy)
        || !intermediate_margins.transition_delta_v.robust(policy)
    {
        DecisiveReason::TransitionAuthority
    } else if !terminal.accepted {
        DecisiveReason::TerminalCapture
    } else {
        DecisiveReason::Accepted
    };
    let mut result = OneGateEvidence {
        gate,
        source_steps,
        target_steps,
        source_acquisition,
        source_leg,
        transition,
        target_leg,
        terminal_capture: terminal,
        accepted,
        decisive_reason,
        margins,
        identity: String::new(),
    };
    result.identity = digest(&OneGateIdentity {
        gate: &result.gate,
        source_steps: result.source_steps,
        target_steps: result.target_steps,
        source_acquisition: &result.source_acquisition,
        source_leg: &result.source_leg,
        transition: &result.transition,
        target_leg: &result.target_leg,
        terminal_capture: &result.terminal_capture,
        accepted: result.accepted,
        decisive_reason: result.decisive_reason,
        margins: result.margins,
    });
    result
}

#[derive(Serialize)]
struct OneGateIdentity<'a> {
    gate: &'a StateGate,
    source_steps: u64,
    target_steps: u64,
    source_acquisition: &'a SourceAssessment,
    source_leg: &'a BallisticLeg,
    transition: &'a TransitionAssessment,
    target_leg: &'a BallisticLeg,
    terminal_capture: &'a TerminalAssessment,
    accepted: bool,
    decisive_reason: DecisiveReason,
    margins: ComponentMargins,
}

fn evaluate_case(policy: &FrozenPolicy, vehicle: &VehicleInput, case: &CanaryCase) -> CanaryResult {
    let (source, target) = gate_policy(policy, case, vehicle);
    source.validate().expect("CB1 source gate validates");
    target.validate().expect("CB1 target gate validates");
    let direct_layout = direct_layout(policy, case, &source, &target);
    let span = direct_layout.terminal_path.anchors_m[0].x - direct_layout.source_exit_m.x;
    let direct_candidates: Vec<DirectCandidate> = candidate_steps(policy, span)
        .into_iter()
        .map(|steps| direct_candidate(policy, vehicle, case, &source, &target, steps))
        .collect();
    let accepted_direct_count = direct_candidates
        .iter()
        .filter(|candidate| candidate.accepted)
        .count();
    let one_gate_candidates: Vec<OneGateEvidence> = case
        .authored_one_gate
        .map(|position| {
            let layout = one_gate_layout(policy, case, &source, &target, position);
            let source_steps = candidate_steps(
                policy,
                layout.redirect_path.anchors_m[0].x - layout.source_exit_m.x,
            );
            let target_steps = candidate_steps(
                policy,
                layout.terminal_path.anchors_m[0].x
                    - layout
                        .redirect_path
                        .anchors_m
                        .last()
                        .expect("redirect exit")
                        .x,
            );
            let mut candidates = Vec::new();
            for source_steps in source_steps {
                for target_steps in &target_steps {
                    candidates.push(one_gate_evidence(
                        policy,
                        vehicle,
                        case,
                        position,
                        source_steps,
                        *target_steps,
                    ));
                }
            }
            candidates
        })
        .unwrap_or_default();
    let accepted_one_gate_count = one_gate_candidates
        .iter()
        .filter(|candidate| candidate.accepted)
        .count();
    let mut result = CanaryResult {
        id: case.id.clone(),
        direct_candidates,
        accepted_direct_count,
        one_gate_candidates,
        accepted_one_gate_count,
        result_identity: String::new(),
    };
    result.result_identity = digest(&ResultIdentity {
        policy,
        vehicle,
        case,
        id: &result.id,
        direct_candidates: &result.direct_candidates,
        accepted_direct_count: result.accepted_direct_count,
        one_gate_candidates: &result.one_gate_candidates,
        accepted_one_gate_count: result.accepted_one_gate_count,
    });
    result
}

#[derive(Serialize)]
struct ResultIdentity<'a> {
    policy: &'a FrozenPolicy,
    vehicle: &'a VehicleInput,
    case: &'a CanaryCase,
    id: &'a str,
    direct_candidates: &'a [DirectCandidate],
    accepted_direct_count: usize,
    one_gate_candidates: &'a [OneGateEvidence],
    accepted_one_gate_count: usize,
}

fn evaluate_fixture(fixture: &CanaryFixture) -> FixtureEvaluation {
    let mut cases: Vec<&CanaryCase> = fixture.cases.iter().collect();
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    let results: Vec<CanaryResult> = cases
        .into_iter()
        .map(|case| evaluate_case(&fixture.policy, &fixture.vehicle, case))
        .collect();
    let mut evaluation = FixtureEvaluation {
        policy_identity: digest(&PolicyVehicleIdentity {
            policy: &fixture.policy,
            vehicle: &fixture.vehicle,
        }),
        results,
        evaluation_identity: String::new(),
    };
    evaluation.evaluation_identity = digest(&EvaluationIdentity {
        policy_identity: &evaluation.policy_identity,
        results: &evaluation.results,
    });
    evaluation
}

#[derive(Serialize)]
struct PolicyVehicleIdentity<'a> {
    policy: &'a FrozenPolicy,
    vehicle: &'a VehicleInput,
}

#[derive(Serialize)]
struct EvaluationIdentity<'a> {
    policy_identity: &'a str,
    results: &'a [CanaryResult],
}

fn validate_result(
    fixture: &CanaryFixture,
    case: &CanaryCase,
    result: &CanaryResult,
) -> Result<(), String> {
    let recomputed = evaluate_case(&fixture.policy, &fixture.vehicle, case);
    if &recomputed != result {
        return Err(format!("{} result does not recompute from inputs", case.id));
    }
    let result_identity = digest(&ResultIdentity {
        policy: &fixture.policy,
        vehicle: &fixture.vehicle,
        case,
        id: &result.id,
        direct_candidates: &result.direct_candidates,
        accepted_direct_count: result.accepted_direct_count,
        one_gate_candidates: &result.one_gate_candidates,
        accepted_one_gate_count: result.accepted_one_gate_count,
    });
    if result.result_identity != result_identity {
        return Err(format!("{} has a result identity mismatch", case.id));
    }
    if result.direct_candidates.iter().any(|candidate| {
        !valid_powered_path(&candidate.source_acquisition.lift_path)
            || !valid_powered_path(&candidate.source_acquisition.redirect.powered_path)
            || !valid_powered_path(&candidate.terminal_capture.transition.powered_path)
            || candidate.identity
                != digest(&DirectIdentity {
                    steps: candidate.steps,
                    leg: &candidate.leg,
                    source_acquisition: &candidate.source_acquisition,
                    terminal_capture: &candidate.terminal_capture,
                    accepted: candidate.accepted,
                    decisive_reason: candidate.decisive_reason,
                    margins: candidate.margins,
                })
    }) {
        return Err(format!("{} has a direct identity mismatch", case.id));
    }
    if result.one_gate_candidates.iter().any(|candidate| {
        if !valid_powered_path(&candidate.source_acquisition.lift_path)
            || !valid_powered_path(&candidate.source_acquisition.redirect.powered_path)
            || !valid_powered_path(&candidate.transition.powered_path)
            || !valid_powered_path(&candidate.terminal_capture.transition.powered_path)
        {
            return true;
        }
        let identity = digest(&OneGateIdentity {
            gate: &candidate.gate,
            source_steps: candidate.source_steps,
            target_steps: candidate.target_steps,
            source_acquisition: &candidate.source_acquisition,
            source_leg: &candidate.source_leg,
            transition: &candidate.transition,
            target_leg: &candidate.target_leg,
            terminal_capture: &candidate.terminal_capture,
            accepted: candidate.accepted,
            decisive_reason: candidate.decisive_reason,
            margins: candidate.margins,
        });
        candidate.identity != identity
    }) {
        return Err(format!("{} has a one-gate identity mismatch", case.id));
    }
    Ok(())
}

fn validate_evaluation(
    fixture: &CanaryFixture,
    evaluation: &FixtureEvaluation,
) -> Result<(), String> {
    let expected_policy_identity = digest(&PolicyVehicleIdentity {
        policy: &fixture.policy,
        vehicle: &fixture.vehicle,
    });
    if evaluation.policy_identity != expected_policy_identity {
        return Err("CB1 evaluation has a policy or vehicle identity mismatch".to_owned());
    }
    let expected_evaluation_identity = digest(&EvaluationIdentity {
        policy_identity: &evaluation.policy_identity,
        results: &evaluation.results,
    });
    if evaluation.evaluation_identity != expected_evaluation_identity {
        return Err("CB1 evaluation identity is not canonical".to_owned());
    }
    if evaluation.results.len() != fixture.cases.len() {
        return Err("CB1 evaluation has the wrong number of cases".to_owned());
    }
    for case in &fixture.cases {
        let result = evaluation
            .results
            .iter()
            .find(|result| result.id == case.id)
            .ok_or_else(|| format!("CB1 evaluation lacks {}", case.id))?;
        validate_result(fixture, case, result)?;
    }
    Ok(())
}

fn min_by(left: Margin, right: Margin) -> Margin {
    if left.normalized.total_cmp(&right.normalized) == Ordering::Greater {
        right
    } else {
        left
    }
}

trait MarginMin {
    fn min_by(self, other: Self) -> Self;
}

impl MarginMin for Margin {
    fn min_by(self, other: Self) -> Self {
        min_by(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_result<'a>(evaluation: &'a FixtureEvaluation, id: &str) -> &'a CanaryResult {
        evaluation
            .results
            .iter()
            .find(|result| result.id == id)
            .unwrap_or_else(|| panic!("missing CB1 result {id}"))
    }

    fn assert_robust(policy: &FrozenPolicy, margin: f64, label: &str) {
        assert!(
            margin > policy.declared_robustness_margin,
            "{label}: normalized margin {margin} is not strictly above declared robustness {}",
            policy.declared_robustness_margin
        );
    }

    fn assert_extra_accepted_slack(policy: &FrozenPolicy, margin: f64, label: &str) {
        let minimum = policy.declared_robustness_margin + 0.025;
        assert!(
            margin >= minimum,
            "{label}: normalized margin {margin} below anti-threshold floor {minimum}",
        );
    }

    fn assert_robust_rejection(policy: &FrozenPolicy, margin: f64, label: &str) {
        assert!(
            margin <= -policy.declared_robustness_margin,
            "{label}: normalized rejection margin {margin} is not beyond -{}",
            policy.declared_robustness_margin
        );
    }

    #[test]
    fn fixture_is_frozen_and_has_exact_four_cases() {
        let fixture = parse_fixture();
        assert_eq!(fixture.cases.len(), 4);
        assert_eq!(fixture.policy.physics_hz, 120);
        assert_eq!(
            fixture.policy.duration_multipliers,
            vec![0.75, 1.0, 1.25, 1.5]
        );
    }

    #[test]
    fn frozen_canaries_stay_within_game_proximate_zone_and_span_bounds() {
        let fixture = parse_fixture();
        let policy = &fixture.policy;
        assert!(policy.source_gate_height_m <= 800.0);
        assert!(policy.source_exit_distance_m <= 800.0);
        assert!(policy.terminal_entry_distance_m <= 800.0);
        assert!(policy.intermediate_entry_distance_m <= 400.0);
        assert!(policy.intermediate_exit_distance_m <= 400.0);
        for case in &fixture.cases {
            let span_m = case.target.center_x_m - case.source.center_x_m;
            let maximum_span_m = match case.id.as_str() {
                "clear_direct_control" => 2500.0,
                "long_span_capture_split" | "late_ridge_capture_split" => 5000.0,
                "insufficient_authority_control" => 9000.0,
                unexpected => panic!("unexpected frozen canary {unexpected}"),
            };
            assert!(span_m <= maximum_span_m, "{}", case.id);
            if case.id == "late_ridge_capture_split" {
                assert!(
                    case.terrain_points_m.iter().all(|point| point.y <= 1500.0),
                    "{} ridge exceeds the frozen game-proximate bound",
                    case.id
                );
            }
        }
    }

    #[test]
    fn cb1_truth_table_has_direct_split_and_honest_rejection() {
        let fixture = parse_fixture();
        let evaluation = evaluate_fixture(&fixture);
        let direct = expected_result(&evaluation, "clear_direct_control");
        assert_eq!(direct.accepted_direct_count, 4);
        assert_eq!(direct.accepted_one_gate_count, 0);
        for candidate in direct
            .direct_candidates
            .iter()
            .filter(|candidate| candidate.accepted)
        {
            assert_robust(
                &fixture.policy,
                candidate.margins.minimum_normalized(),
                "clear direct candidate",
            );
            assert_extra_accepted_slack(
                &fixture.policy,
                candidate.margins.minimum_normalized(),
                "clear direct candidate",
            );
        }

        for id in ["long_span_capture_split", "late_ridge_capture_split"] {
            let result = expected_result(&evaluation, id);
            assert_eq!(result.accepted_direct_count, 0, "{id}");
            let expected_one_gate_count = match id {
                "long_span_capture_split" => 8,
                "late_ridge_capture_split" => 2,
                _ => unreachable!("fixed split canary set"),
            };
            assert_eq!(
                result.accepted_one_gate_count, expected_one_gate_count,
                "{id}"
            );
            for evidence in result
                .one_gate_candidates
                .iter()
                .filter(|candidate| candidate.accepted)
            {
                assert_robust(&fixture.policy, evidence.margins.minimum_normalized(), id);
                assert_extra_accepted_slack(
                    &fixture.policy,
                    evidence.margins.minimum_normalized(),
                    id,
                );
                assert_robust(
                    &fixture.policy,
                    evidence.source_acquisition.margins.minimum_normalized(),
                    &format!("{id} source acquisition"),
                );
                assert_robust(
                    &fixture.policy,
                    evidence.source_leg.clearance_margin.normalized,
                    &format!("{id} source leg"),
                );
                assert_robust(
                    &fixture.policy,
                    evidence.transition.margins.minimum_normalized(),
                    &format!("{id} intermediate transition"),
                );
                assert_robust(
                    &fixture.policy,
                    evidence.target_leg.clearance_margin.normalized,
                    &format!("{id} target leg"),
                );
                assert_robust(
                    &fixture.policy,
                    evidence.terminal_capture.margins.minimum_normalized(),
                    &format!("{id} terminal capture"),
                );
            }
            for candidate in &result.direct_candidates {
                assert_robust_rejection(
                    &fixture.policy,
                    candidate.margins.minimum_normalized(),
                    &format!("{id} direct duration {}", candidate.steps),
                );
            }
        }

        let ridge = expected_result(&evaluation, "late_ridge_capture_split");
        assert!(ridge.direct_candidates.iter().all(|candidate| {
            matches!(
                candidate.decisive_reason,
                DecisiveReason::TerrainClearance | DecisiveReason::TerminalCapture
            )
        }));

        let reject = expected_result(&evaluation, "insufficient_authority_control");
        assert_eq!(reject.accepted_direct_count, 0);
        assert_eq!(reject.accepted_one_gate_count, 0);
        assert!(
            reject
                .direct_candidates
                .iter()
                .all(|candidate| candidate.decisive_reason != DecisiveReason::Accepted)
        );
        assert!(
            reject
                .one_gate_candidates
                .iter()
                .all(|candidate| !candidate.accepted)
        );
        for candidate in &reject.direct_candidates {
            assert_robust_rejection(
                &fixture.policy,
                candidate.margins.minimum_normalized(),
                &format!("insufficient direct duration {}", candidate.steps),
            );
        }
        for candidate in &reject.one_gate_candidates {
            assert_robust_rejection(
                &fixture.policy,
                candidate.margins.minimum_normalized(),
                "insufficient one-gate",
            );
        }
    }

    #[test]
    fn every_direct_duration_retains_a_stable_decisive_reason() {
        let fixture = parse_fixture();
        let evaluation = evaluate_fixture(&fixture);
        for result in &evaluation.results {
            assert!(!result.direct_candidates.is_empty(), "{}", result.id);
            for candidate in &result.direct_candidates {
                assert!(!candidate.identity.is_empty());
                if result.id != "clear_direct_control" {
                    assert_ne!(candidate.decisive_reason, DecisiveReason::Accepted);
                }
            }
        }
    }

    #[test]
    fn authored_gate_uses_the_complete_shared_duration_cross_product() {
        let fixture = parse_fixture();
        let evaluation = evaluate_fixture(&fixture);
        for case in fixture
            .cases
            .iter()
            .filter(|case| case.authored_one_gate.is_some())
        {
            let result = expected_result(&evaluation, &case.id);
            let position = case.authored_one_gate.expect("filtered authored gate");
            let (source, target) = gate_policy(&fixture.policy, case, &fixture.vehicle);
            let layout = one_gate_layout(&fixture.policy, case, &source, &target, position);
            let expected_source_steps = candidate_steps(
                &fixture.policy,
                layout.redirect_path.anchors_m[0].x - layout.source_exit_m.x,
            );
            let expected_target_steps = candidate_steps(
                &fixture.policy,
                layout.terminal_path.anchors_m[0].x
                    - layout
                        .redirect_path
                        .anchors_m
                        .last()
                        .expect("redirect exit")
                        .x,
            );
            let expected_pairs: Vec<(u64, u64)> = expected_source_steps
                .iter()
                .flat_map(|source_steps| {
                    expected_target_steps
                        .iter()
                        .map(move |target_steps| (*source_steps, *target_steps))
                })
                .collect();
            let actual_pairs: Vec<(u64, u64)> = result
                .one_gate_candidates
                .iter()
                .map(|candidate| (candidate.source_steps, candidate.target_steps))
                .collect();
            assert_eq!(actual_pairs, expected_pairs, "{}", case.id);
        }
    }

    #[test]
    fn accepted_routes_have_explicit_disjoint_powered_and_coast_geometry() {
        let fixture = parse_fixture();
        let evaluation = evaluate_fixture(&fixture);
        for result in &evaluation.results {
            for candidate in result
                .direct_candidates
                .iter()
                .filter(|candidate| candidate.accepted)
            {
                let source = &candidate.source_acquisition;
                let terminal = &candidate.terminal_capture.transition;
                assert!(valid_powered_path(&source.lift_path));
                assert!(valid_powered_path(&source.redirect.powered_path));
                assert!(valid_powered_path(&terminal.powered_path));
                assert_eq!(source.lift_path.clearance_inflation_m, 0.0);
                assert_eq!(
                    source.redirect.powered_path.clearance_inflation_m,
                    fixture.policy.gate_capture_radius_m
                );
                assert_eq!(
                    terminal.powered_path.clearance_inflation_m,
                    fixture.policy.gate_capture_radius_m
                );
                assert_eq!(
                    source.redirect.powered_path.anchors_m.last(),
                    Some(&candidate.leg.start_m)
                );
                assert_eq!(
                    terminal.powered_path.anchors_m.first(),
                    Some(&candidate.leg.end_m)
                );
                assert!(candidate.leg.start_m.x < candidate.leg.end_m.x);
                assert!(
                    source.redirect.powered_path.anchors_m[0].x
                        < source.redirect.powered_path.anchors_m[1].x
                        && candidate.leg.start_m.x < candidate.leg.end_m.x
                        && terminal.powered_path.anchors_m[0].x
                            < terminal.powered_path.anchors_m[1].x
                );
            }
            for candidate in result
                .one_gate_candidates
                .iter()
                .filter(|candidate| candidate.accepted)
            {
                let source = &candidate.source_acquisition;
                let redirect = &candidate.transition;
                let terminal = &candidate.terminal_capture.transition;
                assert!(valid_powered_path(&source.lift_path));
                assert!(valid_powered_path(&source.redirect.powered_path));
                assert!(valid_powered_path(&redirect.powered_path));
                assert!(valid_powered_path(&terminal.powered_path));
                assert_eq!(source.lift_path.clearance_inflation_m, 0.0);
                for maneuver in [
                    &source.redirect.powered_path,
                    &redirect.powered_path,
                    &terminal.powered_path,
                ] {
                    assert_eq!(
                        maneuver.clearance_inflation_m,
                        fixture.policy.gate_capture_radius_m
                    );
                }
                assert_eq!(
                    source.redirect.powered_path.anchors_m.last(),
                    Some(&candidate.source_leg.start_m)
                );
                assert_eq!(
                    redirect.powered_path.anchors_m.first(),
                    Some(&candidate.source_leg.end_m)
                );
                assert_eq!(
                    redirect.powered_path.anchors_m.last(),
                    Some(&candidate.target_leg.start_m)
                );
                assert_eq!(
                    terminal.powered_path.anchors_m.first(),
                    Some(&candidate.target_leg.end_m)
                );
                assert_eq!(
                    redirect.powered_path.anchors_m[1],
                    candidate.gate.position_m
                );
                assert!(
                    source.redirect.powered_path.anchors_m[0].x < candidate.source_leg.start_m.x
                        && candidate.source_leg.start_m.x < candidate.source_leg.end_m.x
                        && candidate.source_leg.end_m.x < candidate.target_leg.start_m.x
                        && candidate.target_leg.start_m.x < candidate.target_leg.end_m.x
                        && candidate.target_leg.end_m.x < terminal.powered_path.anchors_m[1].x
                );
            }
        }
    }

    #[test]
    fn result_identities_recompute_and_tamper_is_rejected() {
        let fixture = parse_fixture();
        let evaluation = evaluate_fixture(&fixture);
        validate_evaluation(&fixture, &evaluation).unwrap();
        for case in &fixture.cases {
            let result = expected_result(&evaluation, &case.id);
            validate_result(&fixture, case, result).unwrap();
        }
        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == "clear_direct_control")
            .unwrap();
        let mut tampered = expected_result(&evaluation, &case.id).clone();
        tampered.direct_candidates[0].margins.clearance.raw += 1.0;
        assert!(validate_result(&fixture, case, &tampered).is_err());

        let mut tampered_leg = expected_result(&evaluation, &case.id).clone();
        tampered_leg.direct_candidates[0].leg.positions_m[1].y += 1.0;
        assert!(validate_result(&fixture, case, &tampered_leg).is_err());

        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == "long_span_capture_split")
            .unwrap();
        let mut tampered_transition = expected_result(&evaluation, &case.id).clone();
        tampered_transition
            .one_gate_candidates
            .first_mut()
            .expect("long-span oracle")
            .transition
            .fuel_burn_kg += 1.0;
        assert!(validate_result(&fixture, case, &tampered_transition).is_err());

        let mut tampered_anchor = expected_result(&evaluation, &case.id).clone();
        tampered_anchor
            .one_gate_candidates
            .first_mut()
            .expect("long-span oracle")
            .transition
            .powered_path
            .anchors_m[1]
            .y += 1.0;
        assert!(validate_result(&fixture, case, &tampered_anchor).is_err());

        let mut tampered_inflation = expected_result(&evaluation, &case.id).clone();
        tampered_inflation
            .one_gate_candidates
            .first_mut()
            .expect("long-span oracle")
            .transition
            .powered_path
            .clearance_inflation_m += 1.0;
        assert!(validate_result(&fixture, case, &tampered_inflation).is_err());

        let mut tampered_evaluation = evaluation.clone();
        tampered_evaluation.evaluation_identity.push('0');
        assert!(validate_evaluation(&fixture, &tampered_evaluation).is_err());

        let mut changed_vehicle = fixture.clone();
        changed_vehicle.vehicle.max_thrust_n += 1.0;
        assert_ne!(
            evaluation.policy_identity,
            evaluate_fixture(&changed_vehicle).policy_identity
        );
    }

    #[test]
    fn fixture_case_order_does_not_change_evaluation_identity() {
        let fixture = parse_fixture();
        let first = evaluate_fixture(&fixture);
        assert_eq!(first, evaluate_fixture(&fixture));
        let mut reordered = fixture.clone();
        reordered.cases.reverse();
        let second = evaluate_fixture(&reordered);
        assert_eq!(first, second);
    }

    #[test]
    fn exact_discrete_leg_matches_semi_implicit_equations() {
        let fixture = parse_fixture();
        let case = fixture
            .cases
            .iter()
            .find(|case| case.id == "clear_direct_control")
            .unwrap();
        let (source, target) = gate_policy(&fixture.policy, case, &fixture.vehicle);
        let layout = direct_layout(&fixture.policy, case, &source, &target);
        let leg = ballistic_leg(
            &fixture.policy,
            case,
            layout.source_exit_m,
            layout.terminal_path.anchors_m[0],
            candidate_steps(
                &fixture.policy,
                layout.terminal_path.anchors_m[0].x - layout.source_exit_m.x,
            )[1],
        )
        .unwrap();
        let dt = 1.0 / f64::from(fixture.policy.physics_hz);
        let n = leg.steps as f64;
        let expected_y = leg.start_m.y + n * dt * leg.departure_velocity_mps.y
            - 0.5 * fixture.policy.gravity_mps2 * dt * dt * n * (n + 1.0);
        assert!((expected_y - leg.end_m.y).abs() < 1.0e-9);
        assert!((leg.positions_m.last().unwrap().y - leg.end_m.y).abs() < 1.0e-9);
    }
}
