//! V2 composition over two explicit primitive backends. Both retain provenance;
//! neither morphologies nor a controller influence sampling.
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

use crate::{TerrainBackendV2, TerrainControlsV2};

pub(crate) fn mix_v2(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit(seed: u64, cell: i64, salt: u64) -> f64 {
    (mix_v2(seed ^ mix_v2(cell as u64) ^ mix_v2(salt)) >> 11) as f64 / ((1_u64 << 53) as f64)
}

fn primitive(seed: i32) -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(seed);
    noise.set_noise_type(Some(NoiseType::Perlin));
    noise.set_frequency(Some(1.0));
    noise.set_fractal_type(Some(FractalType::None));
    noise
}

pub(crate) struct TerrainSamplerV2 {
    backend: TerrainBackendV2,
    seed: u64,
    controls: TerrainControlsV2<f64>,
    octaves: u32,
    primitives: [FastNoiseLite; 4],
}

impl TerrainSamplerV2 {
    pub(crate) fn new(
        backend: TerrainBackendV2,
        seed: u64,
        controls: TerrainControlsV2<f64>,
        octaves: u32,
    ) -> Self {
        Self {
            backend,
            seed,
            controls,
            octaves,
            primitives: [101, 211, 307, 401]
                .map(|salt| primitive(((seed & 0x7fffffff) as i32).wrapping_add(salt))),
        }
    }

    fn noise(&self, component: usize, x: f64, y: f64) -> f64 {
        match self.backend {
            TerrainBackendV2::LayeredPerlin => self.primitives[component].get_noise_2d(x, y) as f64,
            TerrainBackendV2::ValueNoise => {
                let cell = x.floor() as i64;
                let seed = mix_v2(self.seed ^ mix_v2(component as u64) ^ mix_v2(y.to_bits()));
                let a = unit(seed, cell, 0) * 2.0 - 1.0;
                let b = unit(seed, cell + 1, 0) * 2.0 - 1.0;
                a + (b - a) * (x - cell as f64)
            }
        }
    }

    pub(crate) fn height(&self, physical_x: f64) -> f64 {
        let p = self.controls;
        let x = physical_x + 137.0;
        let warped = x + self.noise(3, x / p.warp_wavelength_m, 91.0) * p.warp_amplitude_m;
        let (mut regular, mut ridge, mut norm) = (0.0, 0.0, 0.0);
        let (mut weight, mut regular_weight, mut ridge_weight) = (1.0, 1.0, 1.0);
        let mut frequency = 1.0 / p.structure_wavelength_m;
        for _ in 0..self.octaves {
            let n = self.noise(1, warped * frequency, 23.0);
            let r = (1.0 - self.noise(2, warped * frequency, 67.37).abs()).powi(2);
            regular += n * regular_weight;
            ridge += (2.0 * r - 1.0) * ridge_weight;
            norm += weight;
            regular_weight *= p.octave_gain
                * (1.0 - p.weighted_strength
                    + p.weighted_strength * ((n + 1.0) * 0.5).clamp(0.0, 1.0));
            ridge_weight *= p.octave_gain
                * (1.0 - p.weighted_strength + p.weighted_strength * r.clamp(0.0, 1.0));
            weight *= p.octave_gain;
            frequency *= p.lacunarity;
        }
        let cell = (x / p.feature_cell_m).floor() as i64;
        let features: f64 = (cell - 1..=cell + 1).map(|c| self.feature(x, c)).sum();
        let height = 600.0
            + self.noise(0, x / p.macro_wavelength_m, 0.0) * p.macro_amplitude_m
            + (regular * (1.0 - p.ridge_mix) + ridge * p.ridge_mix) / norm
                * p.structure_amplitude_m
            + features;
        (height * 1000.0).round() / 1000.0
    }

    fn feature(&self, x: f64, cell: i64) -> f64 {
        let p = self.controls;
        let random = |salt| unit(self.seed, cell, salt);
        if random(0) >= p.feature_density {
            return 0.0;
        }
        let center =
            (cell as f64 + 0.5) * p.feature_cell_m + (random(1) - 0.5) * p.feature_cell_m * 0.7;
        let t = (x - center).abs() / (p.feature_cell_m * (0.18 + 0.30 * random(2)));
        if t >= 1.0 {
            return 0.0;
        }
        let smooth = |a: f64| {
            let a = a.clamp(0.0, 1.0);
            a * a * (3.0 - 2.0 * a)
        };
        match (random(3) * 3.0) as u32 {
            0 => -p.feature_amplitude_m * (0.08 + 0.10 * random(4)) * (1.0 - t * t).powi(2),
            1 => {
                p.feature_amplitude_m
                    * (0.06 + 0.10 * random(5))
                    * if t <= 0.45 {
                        1.0
                    } else {
                        1.0 - smooth((t - 0.45) / 0.55)
                    }
            }
            _ => -p.feature_amplitude_m * (0.05 + 0.08 * random(6)) * (1.0 - smooth(t)),
        }
    }
}
