//! Shared discrete ballistic and powered-bridge math used by current nominal
//! waypoint execution. This module has no fixture, report, terrain, or
//! experimental-ridge dependencies.

use std::collections::BTreeSet;

use pd_core::Vec2;
use serde::{Deserialize, Serialize};

pub mod aim;
pub mod canonical_initial;
pub use canonical_initial::*;

pub(crate) const ENDPOINT_TOLERANCE: f64 = 1.0e-8;

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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarginV2 {
    pub raw: f64,
    pub normalized: f64,
}

impl MarginV2 {
    pub(crate) fn unbounded() -> Self {
        Self {
            raw: f64::MAX,
            normalized: f64::MAX,
        }
    }

    pub(crate) fn failed() -> Self {
        Self {
            raw: -f64::MAX,
            normalized: -f64::MAX,
        }
    }

    pub(crate) fn upper(limit: f64, value: f64) -> Self {
        let raw = limit - value;
        Self {
            raw,
            normalized: raw / limit.abs().max(f64::EPSILON),
        }
    }

    /// Tight numerical equality uses its own scale. Normalizing against one
    /// metre/second would otherwise turn a correct `1e-8` endpoint into an
    /// artificial robustness failure.
    pub(crate) fn upper_relative(limit: f64, value: f64) -> Self {
        let raw = limit - value;
        Self {
            raw,
            normalized: raw / limit,
        }
    }

    pub(crate) fn lower(value: f64, limit: f64) -> Self {
        let raw = value - limit;
        Self {
            raw,
            normalized: raw / limit.abs().max(f64::EPSILON),
        }
    }

    pub(crate) fn passes(self, policy: &DirectBridgePolicyV2) -> bool {
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
    pub(crate) fn unbounded() -> Self {
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeSampleV2 {
    /// Zero-based acceleration sample index. `state_m` is the exact state
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
    /// are derived from the compact coefficients and intentionally omitted
    /// from setup-report artifacts.
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
    pub(crate) fn dt_s(&self) -> f64 {
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
    pub(crate) fn worst_case_mass_kg(&self) -> f64 {
        self.dry_mass_kg + self.max_fuel_kg
    }

    pub(crate) fn max_acceleration_mps2(&self) -> f64 {
        self.max_thrust_n / self.worst_case_mass_kg()
    }

    pub(crate) fn derated_max_acceleration_mps2(&self, policy: &DirectBridgePolicyV2) -> f64 {
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

/// Return the immutable supported vehicle contract used by the nominal
/// waypoint lane. Kept here so current preflight does not need the retired
/// experiment fixture loader.
pub fn supported_vehicle_input_v2() -> VehicleInputV2 {
    VehicleInputV2 {
        geometry: VehicleGeometryInputV2 {
            hull_width_m: 8.0,
            hull_height_m: 10.0,
            touchdown_half_span_m: 4.0,
            touchdown_base_offset_m: 5.0,
        },
        dry_mass_kg: 7200.0,
        initial_fuel_kg: 6300.0,
        max_fuel_kg: 6300.0,
        max_fuel_burn_kgps: 49.5,
        max_thrust_n: 240000.0,
        min_throttle_frac: 0.25,
        max_rotation_rate_radps: std::f64::consts::FRAC_PI_2,
        safe_touchdown_normal_speed_mps: 3.0,
        safe_touchdown_tangential_speed_mps: 2.0,
        safe_touchdown_attitude_error_rad: 0.15,
        safe_touchdown_angular_rate_radps: 0.35,
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

#[derive(Serialize)]
pub(crate) struct BridgeIdentity<'a> {
    pub(crate) kind: BridgeKindV2,
    pub(crate) start_state: KinematicStateV2,
    pub(crate) end_state: KinematicStateV2,
    pub(crate) steps: u64,
    pub(crate) initial_net_acceleration_mps2: Vec2,
    pub(crate) net_acceleration_step_mps2: Vec2,
    pub(crate) endpoint_position_error_m: f64,
    pub(crate) endpoint_velocity_error_mps: f64,
    pub(crate) fuel_burn_kg: f64,
    pub(crate) classification: CertificationV2,
    pub(crate) reasons: &'a [DirectBridgeReasonV2],
    pub(crate) margins: ComponentMarginsV2,
}

pub(crate) fn digest<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("v2 direct bridge identity serialization");
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

pub(crate) fn distance(left: Vec2, right: Vec2) -> f64 {
    (left - right).length()
}

pub(crate) fn normalize(vector: Vec2) -> Option<Vec2> {
    let length = vector.length();
    (length > 1.0e-12).then(|| vector * (1.0 / length))
}

pub(crate) fn angle_between(left: Vec2, right: Vec2) -> f64 {
    match (normalize(left), normalize(right)) {
        (Some(left), Some(right)) => {
            let dot = (left.x * right.x) + (left.y * right.y);
            dot.clamp(-1.0, 1.0).acos()
        }
        // A zero-thrust direction is not an attitude command. The caller's
        // minimum-throttle/required-direction screen owns that rejection.
        _ => 0.0,
    }
}

pub(crate) fn candidate_steps(
    policy: &DirectBridgePolicyV2,
    horizontal_span_m: f64,
) -> Vec<(f64, u64)> {
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

pub(crate) fn bridge_duration_steps(policy: &DirectBridgePolicyV2) -> Vec<u64> {
    let interval = policy.bridge_interval_ticks();
    let maximum = (policy.mission_budget_s() * f64::from(policy.physics_hz)).floor() as u64;
    (interval..=maximum).step_by(interval as usize).collect()
}

pub(crate) fn source_handoff_steps(arc: &VirtualBallisticArcV2, interval: u64) -> Vec<u64> {
    let mut steps = BTreeSet::new();
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

pub(crate) fn terminal_handoff_steps(arc: &VirtualBallisticArcV2, interval: u64) -> Vec<u64> {
    let mut steps = BTreeSet::new();
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

pub(crate) fn bridge_coefficients(
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
/// A bridge sample with zero-based tick `j` is the state after `j + 1` updates.
pub(crate) fn exact_affine_bridge_state(
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

pub(crate) fn primitive_precheck_failure(
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

pub(crate) fn local_attitude_precheck_failure(
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

/// Construct the exact semi-implicit bridge with an affine net-acceleration
/// sequence. It certifies dynamics feasibility but does not produce commands.
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

pub(crate) fn first_powered_direction(bridge: &AnalyticalBridgeV2) -> Option<(u64, Vec2)> {
    bridge.samples.iter().find_map(|sample| {
        sample
            .thrust_direction_unit
            .map(|direction| (sample.tick, direction))
    })
}

pub(crate) fn final_powered_direction(bridge: &AnalyticalBridgeV2) -> Option<(u64, Vec2)> {
    bridge.samples.iter().rev().find_map(|sample| {
        sample
            .thrust_direction_unit
            .map(|direction| (sample.tick, direction))
    })
}

pub(crate) fn terminal_final_rotation_rate(bridge: &AnalyticalBridgeV2) -> Option<f64> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy() -> DirectBridgePolicyV2 {
        DirectBridgePolicyV2 {
            physics_hz: 120,
            gravity_mps2: 9.81,
            duration_multipliers: vec![0.75, 1.0, 1.25, 1.5],
            minimum_clearance_m: 5.0,
            maximum_mission_time_s: 180.0,
            mission_time_reserve_s: 10.0,
            thrust_derate: 0.81,
            declared_robustness_margin: 0.075,
            handoff_interval_s: 0.25,
            bridge_duration_interval_s: 0.5,
            terminal_target_downward_speed_fraction: 0.5,
        }
    }

    fn test_vehicle() -> VehicleInputV2 {
        VehicleInputV2 {
            geometry: VehicleGeometryInputV2 {
                hull_width_m: 8.0,
                hull_height_m: 10.0,
                touchdown_half_span_m: 4.0,
                touchdown_base_offset_m: 5.0,
            },
            dry_mass_kg: 7200.0,
            initial_fuel_kg: 6300.0,
            max_fuel_kg: 6300.0,
            max_fuel_burn_kgps: 49.5,
            max_thrust_n: 240000.0,
            min_throttle_frac: 0.25,
            max_rotation_rate_radps: std::f64::consts::FRAC_PI_2,
            safe_touchdown_normal_speed_mps: 3.0,
            safe_touchdown_tangential_speed_mps: 2.0,
            safe_touchdown_attitude_error_rad: 0.15,
            safe_touchdown_angular_rate_radps: 0.35,
        }
    }

    #[test]
    fn exact_bridge_reproduces_both_endpoint_components() {
        let policy = test_policy();
        let vehicle = test_vehicle();
        let bridge = exact_discrete_bridge_v2(
            &policy,
            &vehicle,
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
            &policy,
            &vehicle,
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
        assert_eq!(supported_vehicle_input_v2(), vehicle);
    }

    #[test]
    fn virtual_ballistic_arc_uses_exact_semi_implicit_equations() {
        let policy = test_policy();
        let arc = VirtualBallisticArcV2::new(
            &policy,
            Vec2::new(18.0, 10.0),
            Vec2::new(2500.0, 5.0),
            2_700,
        );
        let end = arc.state_at(arc.steps);
        assert!(distance(end.position_m, arc.end_m) <= ENDPOINT_TOLERANCE);
        assert!(distance(end.velocity_mps, arc.arrival_velocity_mps) <= ENDPOINT_TOLERANCE);
        assert_eq!(arc.gravity_mps2, policy.gravity_mps2);
        let one = arc.state_at(1);
        assert_eq!(
            one.velocity_mps,
            arc.departure_velocity_mps + Vec2::new(0.0, -policy.gravity_mps2 * arc.dt_s)
        );
        let mut altered_policy = policy.clone();
        altered_policy.gravity_mps2 = 8.0;
        let altered = VirtualBallisticArcV2::new(
            &altered_policy,
            Vec2::new(18.0, 10.0),
            Vec2::new(2500.0, 5.0),
            2_700,
        );
        assert_ne!(arc.identity, altered.identity);
    }
}
