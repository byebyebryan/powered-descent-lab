use std::collections::BTreeMap;

use pd_core::{
    EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, SimConfig, TerrainDefinition, Vec2,
    VehicleGeometry, VehicleInitialState, VehicleSpec, WorldSpec,
};
use serde::{Deserialize, Serialize};

use crate::{physical_identity, sha256};

pub const GENERATOR_VERSION: &str = "pd-missions/value-noise-rest-v1";
pub const LAYERED_GENERATOR_VERSION: &str = "pd-missions/layered-perlin-rest-v1";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeV1 {
    pub min: f64,
    pub max: f64,
}

impl RangeV1 {
    fn validate(self) -> Result<(), String> {
        if !self.min.is_finite()
            || !self.max.is_finite()
            || self.min > self.max
            || self.min.abs().max(self.max.abs()) > 1_000_000.0
        {
            return Err("recipe range must be finite, ordered and within +/- 1,000,000".to_owned());
        }
        Ok(())
    }
    fn draw(self, rng: &mut Rng) -> f64 {
        quantize(self.min + (self.max - self.min) * rng.unit())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionRecipeV1 {
    pub schema_version: u32,
    pub generator_version: String,
    pub sim: SimConfig,
    pub gravity_mps2: f64,
    pub vehicle: VehicleSpec,
    pub travel_distance_m: RangeV1,
    pub source_x_m: RangeV1,
    pub pad_width_m: RangeV1,
    pub terrain_amplitude_strata_m: [RangeV1; 4],
    pub terrain_wavelength_m: RangeV1,
    pub terrain_spacing_m: f64,
    pub domain_margin_m: f64,
    pub max_attempts: u32,
}

impl Default for MissionRecipeV1 {
    fn default() -> Self {
        Self {
            schema_version: 1,
            generator_version: GENERATOR_VERSION.to_owned(),
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
                dry_mass_kg: 7_200.0,
                initial_fuel_kg: 6_300.0,
                max_fuel_kg: 6_300.0,
                max_thrust_n: 240_000.0,
                max_fuel_burn_kgps: 49.5,
                min_throttle_frac: 0.25,
                max_rotation_rate_radps: std::f64::consts::FRAC_PI_2,
                safe_touchdown_normal_speed_mps: 3.0,
                safe_touchdown_tangential_speed_mps: 2.0,
                safe_touchdown_attitude_error_rad: 0.15,
                safe_touchdown_angular_rate_radps: 0.35,
            },
            travel_distance_m: RangeV1 {
                min: 300.0,
                max: 2_400.0,
            },
            source_x_m: RangeV1 {
                min: -500.0,
                max: 500.0,
            },
            pad_width_m: RangeV1 {
                min: 24.0,
                max: 80.0,
            },
            terrain_amplitude_strata_m: [
                RangeV1 {
                    min: 20.0,
                    max: 60.0,
                },
                RangeV1 {
                    min: 60.0,
                    max: 140.0,
                },
                RangeV1 {
                    min: 140.0,
                    max: 280.0,
                },
                RangeV1 {
                    min: 280.0,
                    max: 480.0,
                },
            ],
            terrain_wavelength_m: RangeV1 {
                min: 320.0,
                max: 960.0,
            },
            terrain_spacing_m: 8.0,
            domain_margin_m: 800.0,
            max_attempts: 4,
        }
    }
}

impl MissionRecipeV1 {
    /// The layered family shares the physical distribution, not the noise primitive.
    pub fn layered() -> Self {
        Self {
            generator_version: LAYERED_GENERATOR_VERSION.to_owned(),
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || ![GENERATOR_VERSION, LAYERED_GENERATOR_VERSION]
                .contains(&self.generator_version.as_str())
        {
            return Err("unsupported mission recipe/generator version".to_owned());
        }
        self.sim.validate()?;
        self.vehicle.validate()?;
        for range in [
            self.travel_distance_m,
            self.source_x_m,
            self.pad_width_m,
            self.terrain_wavelength_m,
        ]
        .into_iter()
        .chain(self.terrain_amplitude_strata_m)
        {
            range.validate()?;
        }
        let minimum_width = self
            .vehicle
            .geometry
            .hull_width_m
            .max(self.vehicle.geometry.touchdown_half_span_m * 2.0)
            + 2.0;
        if !self.gravity_mps2.is_finite()
            || self.gravity_mps2 <= 0.0
            || self.travel_distance_m.min <= 0.0
            || self.pad_width_m.min < minimum_width
            || self.terrain_wavelength_m.min < 8.0
            || self.terrain_amplitude_strata_m.iter().any(|r| r.min < 0.0)
            || !self.terrain_spacing_m.is_finite()
            || !(1.0..=128.0).contains(&self.terrain_spacing_m)
            || !self.domain_margin_m.is_finite()
            || !(128.0..=10_000.0).contains(&self.domain_margin_m)
            || self.max_attempts == 0
            || self.max_attempts > 64
            || (self.travel_distance_m.max + 2.0 * self.domain_margin_m) / self.terrain_spacing_m
                > 100_000.0
        {
            return Err("mission recipe exceeds structural or bounded-work limits".to_owned());
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalKeyV1 {
    pub seed: u64,
    pub index: u64,
    pub attempt: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedMissionV1 {
    pub schema_version: u32,
    pub recipe: MissionRecipeV1,
    pub recipe_sha256: String,
    pub proposal: ProposalKeyV1,
    pub terrain_stratum: u32,
    pub direction: i32,
    pub terrain_amplitude_m: f64,
    pub terrain_wavelength_m: f64,
    pub physical_sha256: String,
    pub scenario: ScenarioSpec,
}

impl ResolvedMissionV1 {
    /// Input validation and identity checking only; never invokes a strategy.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.recipe_sha256 != self.recipe.identity()?
            || self.proposal.attempt >= self.recipe.max_attempts
            || self.terrain_stratum != (self.proposal.index % 4) as u32
            || self.direction != direction(self.proposal.index)
            || self.physical_sha256
                != physical_identity(&self.scenario).map_err(|e| e.to_string())?
        {
            return Err("resolved mission receipt identity mismatch".to_owned());
        }
        self.scenario.validate()?;
        let amplitude = self.recipe.terrain_amplitude_strata_m[self.terrain_stratum as usize];
        let pads = &self.scenario.world.landing_pads;
        if self.scenario.sim != self.recipe.sim
            || self.scenario.vehicle != self.recipe.vehicle
            || self.scenario.world.gravity_mps2 != self.recipe.gravity_mps2
            || self.scenario.seed != self.proposal.seed
            || !in_range(self.terrain_amplitude_m, amplitude)
            || !in_range(self.terrain_wavelength_m, self.recipe.terrain_wavelength_m)
            || pads.len() != 2
            || pads[0].id != "source"
            || pads[1].id != "target"
            || !in_range(pads[0].center_x_m, self.recipe.source_x_m)
            || pads
                .iter()
                .any(|pad| !in_range(pad.width_m, self.recipe.pad_width_m))
            || !in_range(
                (pads[1].center_x_m - pads[0].center_x_m) * f64::from(self.direction),
                self.recipe.travel_distance_m,
            )
            || self.scenario.mission.transfer_route.is_some()
            || self.scenario.mission.goal
                != (EvaluationGoal::LandingOnPad {
                    target_pad_id: "target".to_owned(),
                })
            || self.scenario.initial_state
                != (VehicleInitialState {
                    position_m: Vec2::new(
                        pads[0].center_x_m,
                        pads[0].surface_y_m + self.recipe.vehicle.geometry.touchdown_base_offset_m,
                    ),
                    velocity_mps: Vec2::new(0.0, 0.0),
                    attitude_rad: 0.0,
                    angular_rate_radps: 0.0,
                })
        {
            return Err("resolved mission differs from declared recipe tier".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralRejectionV1 {
    OverlappingPads,
    InvalidScenario,
    ObstructedSourceRest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectedProposalV1 {
    pub proposal: ProposalKeyV1,
    pub reason: StructuralRejectionV1,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationResultV1 {
    pub mission: Option<ResolvedMissionV1>,
    pub rejections: Vec<RejectedProposalV1>,
}

fn direction(index: u64) -> i32 {
    if (index / 4).is_multiple_of(2) { 1 } else { -1 }
}

/// Each request has a finite retry budget. Index-local randomness makes parallel
/// generation and reversed iteration identical to serial generation.
pub fn generate_mission(
    recipe: &MissionRecipeV1,
    seed: u64,
    index: u64,
) -> Result<GenerationResultV1, String> {
    recipe.validate()?;
    let mut rejections = Vec::new();
    for attempt in 0..recipe.max_attempts {
        let proposal = ProposalKeyV1 {
            seed,
            index,
            attempt,
        };
        match propose(recipe, proposal) {
            Ok(mission) => {
                return Ok(GenerationResultV1 {
                    mission: Some(mission),
                    rejections,
                });
            }
            Err((reason, detail)) => rejections.push(RejectedProposalV1 {
                proposal,
                reason,
                detail,
            }),
        }
    }
    Ok(GenerationResultV1 {
        mission: None,
        rejections,
    })
}

fn propose(
    recipe: &MissionRecipeV1,
    proposal: ProposalKeyV1,
) -> Result<ResolvedMissionV1, (StructuralRejectionV1, String)> {
    let mut rng = Rng(mix(proposal.seed
        ^ mix(proposal.index)
        ^ mix(u64::from(proposal.attempt).wrapping_add(0x76ab_5e21))));
    let source_x = recipe.source_x_m.draw(&mut rng);
    let distance = recipe.travel_distance_m.draw(&mut rng);
    let direction = direction(proposal.index);
    let target_x = quantize(source_x + f64::from(direction) * distance);
    let source_width = recipe.pad_width_m.draw(&mut rng);
    let target_width = recipe.pad_width_m.draw(&mut rng);
    if distance <= (source_width + target_width) / 2.0 + recipe.terrain_spacing_m {
        return Err((
            StructuralRejectionV1::OverlappingPads,
            "source and target pad preparation intervals overlap".to_owned(),
        ));
    }
    let stratum = (proposal.index % 4) as usize;
    let amplitude = recipe.terrain_amplitude_strata_m[stratum].draw(&mut rng);
    let wavelength = recipe.terrain_wavelength_m.draw(&mut rng);
    let terrain_seed = rng.next();
    let layered = (recipe.generator_version == LAYERED_GENERATOR_VERSION)
        .then(|| crate::layered::LayeredTerrain::new(terrain_seed, amplitude, wavelength));
    let spacing = recipe.terrain_spacing_m;
    let left = ((source_x.min(target_x) - recipe.domain_margin_m) / spacing).floor() as i64;
    let right = ((source_x.max(target_x) + recipe.domain_margin_m) / spacing).ceil() as i64;
    let raw = TerrainDefinition::Heightfield {
        points_m: (left..=right)
            .map(|cell| {
                let x = cell as f64 * spacing;
                let height = match &layered {
                    Some(sampler) => quantize(sampler.height(x)),
                    None => terrain_height(terrain_seed, x, amplitude, wavelength),
                };
                Vec2::new(x, height)
            })
            .collect(),
    };
    let source_y = quantize(raw.sample_height(source_x));
    let target_y = quantize(raw.sample_height(target_x));
    let pads = vec![
        LandingPadSpec {
            id: "source".to_owned(),
            center_x_m: source_x,
            surface_y_m: source_y,
            width_m: source_width,
        },
        LandingPadSpec {
            id: "target".to_owned(),
            center_x_m: target_x,
            surface_y_m: target_y,
            width_m: target_width,
        },
    ];
    let terrain = prepare_pads(&raw, &pads);
    let scenario = ScenarioSpec {
        id: format!(
            "procedural-{:016x}-{:08}-{}",
            proposal.seed, proposal.index, proposal.attempt
        ),
        name: format!("Procedural transfer {}", proposal.index),
        description: "Complete procedural source-rest transfer; structural validity only"
            .to_owned(),
        seed: proposal.seed,
        tags: vec!["procedural".to_owned(), "source_rest".to_owned()],
        metadata: BTreeMap::from([
            ("generator".to_owned(), recipe.generator_version.clone()),
            ("stratum".to_owned(), stratum.to_string()),
        ]),
        sim: recipe.sim.clone(),
        world: WorldSpec {
            gravity_mps2: recipe.gravity_mps2,
            terrain,
            landing_pads: pads,
        },
        vehicle: recipe.vehicle.clone(),
        initial_state: VehicleInitialState {
            position_m: Vec2::new(
                source_x,
                source_y + recipe.vehicle.geometry.touchdown_base_offset_m,
            ),
            velocity_mps: Vec2::new(0.0, 0.0),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
        },
        mission: MissionSpec {
            transfer_route: None,
            goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".to_owned(),
            },
        },
    };
    scenario
        .validate()
        .map_err(|e| (StructuralRejectionV1::InvalidScenario, e))?;
    let hull_half = scenario.vehicle.geometry.hull_width_m / 2.0;
    let bottom =
        scenario.initial_state.position_m.y - scenario.vehicle.geometry.hull_height_m / 2.0;
    if [source_x - hull_half, source_x, source_x + hull_half]
        .iter()
        .any(|x| scenario.world.terrain.sample_height(*x) > bottom + 1e-9)
    {
        return Err((
            StructuralRejectionV1::ObstructedSourceRest,
            "source-rest hull intersects terrain".to_owned(),
        ));
    }
    Ok(ResolvedMissionV1 {
        schema_version: 1,
        recipe: recipe.clone(),
        recipe_sha256: recipe
            .identity()
            .map_err(|e| (StructuralRejectionV1::InvalidScenario, e))?,
        proposal,
        terrain_stratum: stratum as u32,
        direction,
        terrain_amplitude_m: amplitude,
        terrain_wavelength_m: wavelength,
        physical_sha256: physical_identity(&scenario)
            .map_err(|e| (StructuralRejectionV1::InvalidScenario, e.to_string()))?,
        scenario,
    })
}

fn prepare_pads(raw: &TerrainDefinition, pads: &[LandingPadSpec]) -> TerrainDefinition {
    let mut points: Vec<Vec2> = raw
        .points()
        .iter()
        .copied()
        .filter(|p| {
            !pads
                .iter()
                .any(|pad| (p.x - pad.center_x_m).abs() <= pad.width_m / 2.0)
        })
        .collect();
    for pad in pads {
        points.push(Vec2::new(
            pad.center_x_m - pad.width_m / 2.0,
            pad.surface_y_m,
        ));
        points.push(Vec2::new(
            pad.center_x_m + pad.width_m / 2.0,
            pad.surface_y_m,
        ));
    }
    points.sort_by(|a, b| a.x.total_cmp(&b.x));
    TerrainDefinition::Heightfield { points_m: points }
}

fn quantize(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

fn in_range(value: f64, range: RangeV1) -> bool {
    value.is_finite() && value >= range.min - 1e-8 && value <= range.max + 1e-8
}
fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix(self.0)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}
fn terrain_height(seed: u64, x: f64, amplitude: f64, wavelength: f64) -> f64 {
    let mut value = 0.0;
    for (octave, weight) in [8.0, 4.0, 2.0, 1.0].iter().enumerate() {
        let u = x / (wavelength / (1_u64 << octave) as f64);
        let cell = u.floor() as i64;
        let noise = |c: i64| {
            let bits = mix(seed ^ mix(c as u64) ^ mix(octave as u64 + 0x1521));
            (bits >> 11) as f64 / ((1_u64 << 53) as f64) * 2.0 - 1.0
        };
        let a = noise(cell);
        let b = noise(cell + 1);
        value += weight * (a + (b - a) * (u - cell as f64));
    }
    quantize(600.0 + amplitude * value / 15.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_generator_retains_its_pinned_input_identity() {
        let result = generate_mission(&MissionRecipeV1::default(), 987, 7).unwrap();
        let mission = result.mission.unwrap();
        assert_eq!(
            mission.physical_sha256,
            "39bad3e709064eca98e09e4357afb74843b8ea865bcd7061f7865161026abf63"
        );
        assert_eq!(
            crate::sha256(&crate::json_bytes(&mission).unwrap()),
            "05220c9c1958b2a51bd4401d3edcd595f13e47038bc50b04a07acbef52e88f7d"
        );
    }

    #[test]
    fn layered_family_is_deterministic_and_physically_distinct() {
        let recipe = MissionRecipeV1::layered();
        for index in 0..16 {
            let first = generate_mission(&recipe, 991, index).unwrap();
            let second = generate_mission(&recipe, 991, index).unwrap();
            assert_eq!(
                crate::json_bytes(&first).unwrap(),
                crate::json_bytes(&second).unwrap()
            );
            let mission = first.mission.unwrap();
            mission.validate().unwrap();
            let old = generate_mission(&MissionRecipeV1::default(), 991, index)
                .unwrap()
                .mission
                .unwrap();
            assert_ne!(mission.physical_sha256, old.physical_sha256);
        }
    }

    #[test]
    fn pad_preparation_preserves_every_point_outside_pad_intervals() {
        let raw = TerrainDefinition::Heightfield {
            points_m: (-50..=50)
                .map(|x| Vec2::new(x as f64, (x * x) as f64))
                .collect(),
        };
        let pads = [LandingPadSpec {
            id: "one".to_owned(),
            center_x_m: 4.5,
            surface_y_m: 20.0,
            width_m: 12.0,
        }];
        let prepared = prepare_pads(&raw, &pads);
        prepared.validate().unwrap();
        for point in raw.points() {
            if (point.x - pads[0].center_x_m).abs() > 6.0 {
                assert!(prepared.points().contains(point));
            }
        }
        for x in [-1.5, 4.5, 10.5] {
            assert_eq!(prepared.sample_height(x), 20.0);
        }
    }
}
