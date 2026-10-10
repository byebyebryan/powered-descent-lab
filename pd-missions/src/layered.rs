//! Versioned, self-contained layered terrain. The pinned Perlin primitive is
//! FastNoiseLite, not Pylander's primitive; the composition follows the earlier
//! input-only study. All constants belong to layered-perlin-rest-v1.
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

fn primitive(seed: i32) -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(seed);
    noise.set_noise_type(Some(NoiseType::Perlin));
    noise.set_frequency(Some(1.0));
    noise.set_fractal_type(Some(FractalType::None));
    noise
}

fn rand01(seed: i32, cell: i64, salt: u64) -> f64 {
    let mut value = (cell as u64).wrapping_mul(1619)
        ^ (seed as u64).wrapping_mul(31337)
        ^ salt.wrapping_mul(6971);
    value ^= value.wrapping_shl(13);
    let hash = value
        .wrapping_mul(
            value
                .wrapping_mul(value)
                .wrapping_mul(15731)
                .wrapping_add(789221),
        )
        .wrapping_add(1376312589);
    (hash & 0x7fffffff) as f64 / 2147483647.0
}

fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn feature(x: f64, cell: i64, seed: i32, cell_size: f64, amp: f64) -> f64 {
    let rand = |salt| rand01(seed, cell, salt);
    if rand(0) >= 0.35 {
        return 0.0;
    }
    let center = (cell as f64 + 0.5) * cell_size + (rand(1) - 0.5) * cell_size * 0.7;
    let t = (x - center).abs() / (cell_size * (0.18 + 0.30 * rand(2)));
    if t >= 1.0 {
        return 0.0;
    }
    match (rand(3) * 3.0) as i32 {
        0 => -amp * (0.08 + 0.10 * rand(4)) * (1.0 - t * t).powi(2),
        1 => {
            let height = amp * (0.06 + 0.10 * rand(5));
            if t <= 0.45 {
                height
            } else {
                height * (1.0 - smoothstep((t - 0.45) / 0.55))
            }
        }
        _ => -amp * (0.05 + 0.08 * rand(6)) * (1.0 - smoothstep(t)),
    }
}

pub(crate) struct LayeredTerrain {
    seed: i32,
    amplitude: f64,
    wavelength: f64,
    macro_noise: FastNoiseLite,
    structure_noise: FastNoiseLite,
    ridge_noise: FastNoiseLite,
    warp_noise: FastNoiseLite,
}

impl LayeredTerrain {
    pub(crate) fn new(seed: u64, amplitude: f64, wavelength: f64) -> Self {
        let seed = (seed & 0x7fffffff) as i32;
        Self {
            seed,
            amplitude,
            wavelength,
            macro_noise: primitive(seed.wrapping_add(101)),
            structure_noise: primitive(seed.wrapping_add(211)),
            ridge_noise: primitive(seed.wrapping_add(307)),
            warp_noise: primitive(seed.wrapping_add(401)),
        }
    }

    pub(crate) fn height(&self, route_x: f64) -> f64 {
        let noise = |generator: &FastNoiseLite, x, y| generator.get_noise_2d(x, y) as f64;
        let x = route_x + 137.0;
        let xx = x + noise(&self.warp_noise, x / (self.wavelength * (5.0 / 3.0)), 91.0)
            * (self.wavelength / 24.0);
        let (mut regular, mut ridged, mut norm) = (0.0, 0.0, 0.0);
        let (mut amp, mut regular_amp, mut ridge_amp) = (1.0, 1.0, 1.0);
        let mut freq = 1.0 / self.wavelength;
        for _ in 0..3 {
            let n = noise(&self.structure_noise, xx * freq, 23.0);
            let r = (1.0 - noise(&self.ridge_noise, xx * freq, 67.37).abs()).powi(2);
            regular += n * regular_amp;
            ridged += (2.0 * r - 1.0) * ridge_amp;
            norm += amp;
            regular_amp *= 0.35 * (1.0 - 0.65 + 0.65 * ((n + 1.0) * 0.5).clamp(0.0, 1.0));
            ridge_amp *= 0.35 * (1.0 - 0.65 + 0.65 * r.clamp(0.0, 1.0));
            amp *= 0.35;
            freq *= 2.0;
        }
        let structure = (regular * (1.0 - 0.45) + ridged * 0.45) / norm * self.amplitude;
        let cell_size = self.wavelength * 0.6;
        let cell = (x / cell_size).floor() as i64;
        let features: f64 = (cell - 1..=cell + 1)
            .map(|c| feature(x, c, self.seed, cell_size, self.amplitude))
            .sum();
        600.0
            + noise(&self.macro_noise, x * (1.0 / (self.wavelength * 3.0)), 0.0)
                * (self.amplitude * (4.0 / 7.0))
            + structure
            + features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_independent_study_samples_on_both_sides_of_zero() {
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../tests/layered-reference.json")).unwrap();
        for profile in golden["profiles"].as_array().unwrap() {
            let sampler = LayeredTerrain::new(profile["seed"].as_u64().unwrap(), 140.0, 600.0);
            for point in profile["points_m"].as_array().unwrap() {
                let actual = sampler.height(point[0].as_f64().unwrap());
                let expected = 600.0 + point[1].as_f64().unwrap();
                assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
            }
        }
    }
}
