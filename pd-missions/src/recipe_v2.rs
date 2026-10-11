//! Explicit configurable physical generation contracts. V1 stays frozen.
use pd_core::{SimConfig, VehicleGeometry, VehicleSpec};
use serde::{Deserialize, Serialize};

use crate::{RangeV1, sha256};

pub const GENERATOR_VERSION_V2: &str = "pd-missions/configurable-rest-v2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainBackendV2 {
    LayeredPerlin,
    ValueNoise,
}

impl TerrainBackendV2 {
    pub fn key(self) -> &'static str {
        match self {
            Self::LayeredPerlin => "layered_perlin",
            Self::ValueNoise => "value_noise",
        }
    }
}

/// The same named controls carry ranges in a recipe and realized values in a
/// proposal. A shared map/order prevents draw and validation field drift.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainControlsV2<T> {
    pub macro_amplitude_m: T,
    pub macro_wavelength_m: T,
    pub structure_amplitude_m: T,
    pub structure_wavelength_m: T,
    pub ridge_mix: T,
    pub octave_gain: T,
    pub lacunarity: T,
    pub weighted_strength: T,
    pub warp_amplitude_m: T,
    pub warp_wavelength_m: T,
    pub feature_amplitude_m: T,
    pub feature_cell_m: T,
    pub feature_density: T,
}

impl<T: Copy> TerrainControlsV2<T> {
    pub fn map<U>(&self, mut f: impl FnMut(T) -> U) -> TerrainControlsV2<U> {
        TerrainControlsV2 {
            macro_amplitude_m: f(self.macro_amplitude_m),
            macro_wavelength_m: f(self.macro_wavelength_m),
            structure_amplitude_m: f(self.structure_amplitude_m),
            structure_wavelength_m: f(self.structure_wavelength_m),
            ridge_mix: f(self.ridge_mix),
            octave_gain: f(self.octave_gain),
            lacunarity: f(self.lacunarity),
            weighted_strength: f(self.weighted_strength),
            warp_amplitude_m: f(self.warp_amplitude_m),
            warp_wavelength_m: f(self.warp_wavelength_m),
            feature_amplitude_m: f(self.feature_amplitude_m),
            feature_cell_m: f(self.feature_cell_m),
            feature_density: f(self.feature_density),
        }
    }

    pub fn values(&self) -> [T; 13] {
        [
            self.macro_amplitude_m,
            self.macro_wavelength_m,
            self.structure_amplitude_m,
            self.structure_wavelength_m,
            self.ridge_mix,
            self.octave_gain,
            self.lacunarity,
            self.weighted_strength,
            self.warp_amplitude_m,
            self.warp_wavelength_m,
            self.feature_amplitude_m,
            self.feature_cell_m,
            self.feature_density,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionRecipeV2 {
    pub schema_version: u32,
    pub generator_version: String,
    pub backend: TerrainBackendV2,
    pub sim: SimConfig,
    pub gravity_mps2: f64,
    pub vehicle: VehicleSpec,
    pub travel_distance_m: RangeV1,
    pub source_x_m: RangeV1,
    pub source_width_m: RangeV1,
    pub target_width_m: RangeV1,
    pub terrain: TerrainControlsV2<RangeV1>,
    pub octaves: u32,
    pub terrain_spacing_m: f64,
    pub domain_margin_m: f64,
    pub pad_transition_m: f64,
    pub max_attempts: u32,
}

impl Default for MissionRecipeV2 {
    fn default() -> Self {
        let range = |min, max| RangeV1 { min, max };
        Self {
            schema_version: 2,
            generator_version: GENERATOR_VERSION_V2.into(),
            backend: TerrainBackendV2::LayeredPerlin,
            sim: SimConfig {
                physics_hz: 120,
                controller_hz: 60,
                max_time_s: 90.0,
                sample_hz: Some(10),
            },
            gravity_mps2: 9.81,
            vehicle: VehicleSpec {
                geometry: VehicleGeometry {
                    hull_width_m: 8.0,
                    hull_height_m: 10.0,
                    touchdown_half_span_m: 4.0,
                    touchdown_base_offset_m: 5.0,
                },
                dry_mass_kg: 7200.0,
                initial_fuel_kg: 6300.0,
                max_fuel_kg: 6300.0,
                max_thrust_n: 240000.0,
                max_fuel_burn_kgps: 49.5,
                min_throttle_frac: 0.25,
                max_rotation_rate_radps: std::f64::consts::FRAC_PI_2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            travel_distance_m: range(300.0, 2400.0),
            source_x_m: range(-500.0, 500.0),
            source_width_m: range(24.0, 80.0),
            target_width_m: range(24.0, 80.0),
            terrain: TerrainControlsV2 {
                macro_amplitude_m: range(240.0, 960.0),
                macro_wavelength_m: range(1200.0, 3600.0),
                structure_amplitude_m: range(420.0, 1680.0),
                structure_wavelength_m: range(300.0, 1000.0),
                ridge_mix: range(0.1, 0.85),
                octave_gain: range(0.25, 0.55),
                lacunarity: range(1.6, 2.4),
                weighted_strength: range(0.0, 0.8),
                warp_amplitude_m: range(0.0, 120.0),
                warp_wavelength_m: range(500.0, 2000.0),
                feature_amplitude_m: range(40.0, 240.0),
                feature_cell_m: range(180.0, 720.0),
                feature_density: range(0.05, 0.6),
            },
            octaves: 3,
            terrain_spacing_m: 4.0,
            domain_margin_m: 800.0,
            pad_transition_m: 24.0,
            max_attempts: 8,
        }
    }
}

impl MissionRecipeV2 {
    pub fn value_noise() -> Self {
        Self {
            backend: TerrainBackendV2::ValueNoise,
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.sim.validate()?;
        self.vehicle.validate()?;
        for range in [
            self.travel_distance_m,
            self.source_x_m,
            self.source_width_m,
            self.target_width_m,
        ]
        .into_iter()
        .chain(self.terrain.values())
        {
            validate_range_v2(range)?;
        }
        let controls = self.terrain;
        let positive = [
            controls.macro_wavelength_m,
            controls.structure_wavelength_m,
            controls.warp_wavelength_m,
            controls.feature_cell_m,
        ];
        let nonnegative = [
            controls.macro_amplitude_m,
            controls.structure_amplitude_m,
            controls.warp_amplitude_m,
            controls.feature_amplitude_m,
        ];
        let unit = [
            controls.ridge_mix,
            controls.weighted_strength,
            controls.feature_density,
        ];
        let minimum_width = self
            .vehicle
            .geometry
            .hull_width_m
            .max(self.vehicle.geometry.touchdown_half_span_m * 2.0)
            + 2.0;
        if self.schema_version != 2
            || self.generator_version != GENERATOR_VERSION_V2
            || !self.gravity_mps2.is_finite()
            || self.gravity_mps2 <= 0.0
            || self.travel_distance_m.min <= 0.0
            || [self.source_width_m, self.target_width_m]
                .iter()
                .any(|r| r.min < minimum_width || r.max > 256.0)
            || positive.iter().any(|r| r.min < 16.0)
            || nonnegative.iter().any(|r| r.min < 0.0 || r.max > 20000.0)
            || unit.iter().any(|r| r.min < 0.0 || r.max > 1.0)
            || controls.octave_gain.min < 0.1
            || controls.octave_gain.max > 0.8
            || controls.lacunarity.min < 1.1
            || controls.lacunarity.max > 3.0
            || !(1..=4).contains(&self.octaves)
            || !self.terrain_spacing_m.is_finite()
            || !(1.0..=16.0).contains(&self.terrain_spacing_m)
            || !self.domain_margin_m.is_finite()
            || !(128.0..=10000.0).contains(&self.domain_margin_m)
            || self.domain_margin_m
                < self.pad_transition_m
                    + self.source_width_m.max.max(self.target_width_m.max) * 0.5
                    + 1e-6
            || !self.pad_transition_m.is_finite()
            || !(1.0..=128.0).contains(&self.pad_transition_m)
            || self.max_attempts == 0
            || self.max_attempts > 64
            || (self.travel_distance_m.max + 2.0 * self.domain_margin_m) / self.terrain_spacing_m
                > 99980.0
            || controls.structure_wavelength_m.min
                / controls
                    .lacunarity
                    .max
                    .powi(self.octaves.saturating_sub(1) as i32)
                < self.terrain_spacing_m * 4.0
            || controls.feature_cell_m.min * 0.18 < self.terrain_spacing_m * 4.0
        {
            return Err(
                "unsupported or unbounded V2 recipe; inspect physical/sampling limits".into(),
            );
        }
        Ok(())
    }

    pub fn identity(&self) -> Result<String, String> {
        self.validate()?;
        Ok(sha256(
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        ))
    }
}

pub(crate) fn validate_range_v2(range: RangeV1) -> Result<(), String> {
    if !range.min.is_finite()
        || !range.max.is_finite()
        || range.min > range.max
        || range.min.abs().max(range.max.abs()) > 1_000_000.0
    {
        return Err("V2 range must be finite, ordered and bounded".into());
    }
    Ok(())
}

pub(crate) fn in_range_v2(value: f64, range: RangeV1) -> bool {
    value.is_finite() && value >= range.min - 1e-7 && value <= range.max + 1e-7
}
