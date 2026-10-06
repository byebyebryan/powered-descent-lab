use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use serde_json::{json, Value};
use std::io::{self, Read};

fn primitive(seed: i32) -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(seed);
    noise.set_noise_type(Some(NoiseType::Perlin));
    noise.set_frequency(Some(1.0));
    noise.set_fractal_type(Some(FractalType::None));
    noise
}

fn rand01(seed: i32, cell: i64, salt: u64) -> f64 {
    // The low 31 bits match Pylander's unbounded-integer hash, including x < 0.
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

fn feature(x: f64, cell: i64, seed: i32, cell_size: f64, density: f64, amp: f64) -> f64 {
    let rand = |salt| rand01(seed, cell, salt);
    if rand(0) >= density {
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

fn sample(
    seed: i32,
    xs: &[Value],
    recipe: &Value,
    strength: f64,
    offset: f64,
    ridge_slice_offset: f64,
) -> Result<Value, String> {
    if !(0.0..1.0).contains(&ridge_slice_offset) {
        return Err("ridge slice offset must be finite and in [0,1)".into());
    }
    let p = |key: &str| -> f64 { recipe[key].as_f64().expect("validated recipe number") };
    let macro_noise = primitive(seed + 101);
    let structure_noise = primitive(seed + 211);
    let ridge_noise = primitive(seed + 307);
    let warp_noise = primitive(seed + 401);
    let noise = |generator: &FastNoiseLite, x, y| generator.get_noise_2d(x, y) as f64;
    let mut points = Vec::with_capacity(xs.len());
    let mut feature_values = Vec::with_capacity(xs.len());
    for x in xs {
        let route_x = x.as_f64().ok_or("non-numeric x")?;
        let x = route_x + offset;
        let xx = x + noise(&warp_noise, x * p("warp_frequency"), 91.0) * p("warp_amplitude");
        let (mut regular, mut ridged, mut norm) = (0.0, 0.0, 0.0);
        let (mut amp, mut regular_amp, mut ridge_amp) = (1.0, 1.0, 1.0);
        let mut freq = p("structure_frequency");
        for _ in 0..p("structure_octaves") as usize {
            let n = noise(&structure_noise, xx * freq, 23.0);
            let r = (1.0 - noise(&ridge_noise, xx * freq, 67.0 + ridge_slice_offset).abs()).powi(2);
            regular += n * regular_amp;
            ridged += (2.0 * r - 1.0) * ridge_amp;
            norm += amp;
            regular_amp *= p("structure_persistence")
                * (1.0 - strength + strength * ((n + 1.0) * 0.5).clamp(0.0, 1.0));
            ridge_amp *=
                p("structure_persistence") * (1.0 - strength + strength * r.clamp(0.0, 1.0));
            amp *= p("structure_persistence");
            freq *= p("structure_lacunarity");
        }
        let structure = (regular * (1.0 - p("ridge_mix")) + ridged * p("ridge_mix")) / norm
            * p("structure_amplitude");
        let cell = (x / p("feature_cell_size")).floor() as i64;
        let features: f64 = (cell - 1..=cell + 1)
            .map(|c| {
                feature(
                    x,
                    c,
                    seed,
                    p("feature_cell_size"),
                    p("feature_density"),
                    p("structure_amplitude"),
                )
            })
            .sum();
        let y = p("base_height")
            + noise(&macro_noise, x * p("macro_frequency"), 0.0) * p("macro_amplitude")
            + structure
            + features;
        if !x.is_finite() || !y.is_finite() {
            return Err("non-finite profile".into());
        }
        points.push(json!([route_x, y]));
        feature_values.push(features);
    }
    Ok(
        json!({"variant": "fastnoise", "seed": seed, "points_m": points, "features_m": feature_values}),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let request: Value = serde_json::from_str(&input)?;
    let recipe = &request["refined"];
    if !recipe.is_object() || recipe.as_object().unwrap().values().any(|v| !v.is_number()) {
        return Err("recipe must contain numeric parameters".into());
    }
    let strength = request["weighted_strength"]
        .as_f64()
        .ok_or("missing strength")?;
    if !(0.0..=1.0).contains(&strength) {
        return Err("strength out of range".into());
    }
    let xs = request["xs"].as_array().ok_or("missing coordinates")?;
    let offset = request["source_offset_m"]
        .as_f64()
        .ok_or("missing offset")?;
    // An omitted offset preserves the frozen first-study recipe exactly.
    let ridge_slice_offset = match request.get("ridge_slice_offset") {
        None => 0.0,
        Some(value) => value.as_f64().ok_or("non-numeric ridge slice offset")?,
    };
    let mut profiles = Vec::new();
    for seed in request["seeds"].as_array().ok_or("missing seeds")? {
        let seed = i32::try_from(seed.as_i64().ok_or("invalid seed")?)?;
        profiles.push(sample(
            seed,
            xs,
            recipe,
            strength,
            offset,
            ridge_slice_offset,
        )?);
    }
    println!("{}", serde_json::to_string(&profiles)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_cells_and_seed_hashes_are_bounded() {
        for seed in [0, 1, 2, 7, 42, 12345] {
            for cell in -5..5 {
                for salt in 0..7 {
                    assert!((0.0..=1.0).contains(&rand01(seed, cell, salt)));
                }
            }
        }
    }

    #[test]
    fn hashes_match_the_frozen_pylander_function() {
        for (seed, cell, salt, expected) in [
            (0, -1, 0, 305313885_u64),
            (0, 0, 0, 1376312589),
            (1, 2, 3, 184901419),
            (42, -2, 6, 159730543),
            (12345, 1, 5, 1310508301),
        ] {
            assert_eq!(rand01(seed, cell, salt), expected as f64 / 2147483647.0);
        }
    }

    #[test]
    fn feature_support_and_flat_offset_are_explicit() {
        assert_eq!(feature(1e6, 0, 1, 360.0, 1.0, 140.0), 0.0);
        assert_eq!(feature(180.0, 0, 1, 360.0, 0.0, 140.0), 0.0);
    }

    #[test]
    fn integer_slice_forces_zeros_but_fractional_slice_does_not() {
        let noise = primitive(307);
        for x in [0.0, 1.0, 2.0] {
            assert_eq!(noise.get_noise_2d(x, 67.0), 0.0);
        }
        assert!([0.0, 1.0, 2.0]
            .iter()
            .any(|&x| noise.get_noise_2d(x, 67.37).abs() > 0.01));
    }
}
