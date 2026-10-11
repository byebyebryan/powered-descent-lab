use std::collections::BTreeMap;

use pd_core::{
    EvaluationGoal, LandingPadSpec, MissionSpec, ScenarioSpec, TerrainDefinition, Vec2,
    VehicleInitialState, WorldSpec,
};
use serde::{Deserialize, Serialize};

use crate::recipe_v2::in_range_v2;
use crate::terrain_v2::{TerrainSamplerV2, mix_v2};
use crate::{
    GeometrySpecV1, MissionGeometryReceiptV1, MissionRecipeV2, ProposalKeyV1, RangeV1,
    TerrainControlsV2, TransferGeometryV1, measure_mission_geometry, measure_transfer_geometry,
    physical_identity, prepare_pads_v2, sha256,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedMissionV2 {
    pub schema_version: u32,
    pub recipe: MissionRecipeV2,
    pub recipe_sha256: String,
    pub proposal: ProposalKeyV1,
    pub direction: i32,
    pub terrain_seed: u64,
    pub terrain_parameters: TerrainControlsV2<f64>,
    pub raw_terrain: TerrainDefinition,
    pub raw_terrain_sha256: String,
    pub raw_geometry: TransferGeometryV1,
    pub geometry: MissionGeometryReceiptV1,
    pub physical_sha256: String,
    pub scenario: ScenarioSpec,
}

impl ResolvedMissionV2 {
    /// Validate saved physical inputs and derived records. Does not regenerate
    /// noise, choose a route, evaluate a controller or certify solvability.
    pub fn validate(&self) -> Result<(), String> {
        if self.raw_terrain.points().len() > 100001
            || self.scenario.world.terrain.points().len() > 100001
        {
            return Err("V2 resolved terrain exceeds bounded point count".into());
        }
        self.scenario.validate()?;
        self.raw_terrain.validate()?;
        if self.schema_version != 2
            || self.recipe_sha256 != self.recipe.identity()?
            || self.proposal.attempt >= self.recipe.max_attempts
            || self.direction != direction_v2(self.proposal.index)
            || self.physical_sha256
                != physical_identity(&self.scenario).map_err(|e| e.to_string())?
            || self.raw_terrain_sha256
                != sha256(&serde_json::to_vec(&self.raw_terrain).map_err(|e| e.to_string())?)
            || !self
                .terrain_parameters
                .values()
                .into_iter()
                .zip(self.recipe.terrain.values())
                .all(|(v, r)| in_range_v2(v, r))
        {
            return Err("V2 resolved identity or parameter receipt mismatch".into());
        }
        let scenario = &self.scenario;
        let pads = &scenario.world.landing_pads;
        if pads.len() != 2 || pads[0].id != "source" || pads[1].id != "target" {
            return Err("V2 resolved mission requires its two declared pads".into());
        }
        let left = ((pads[0].center_x_m.min(pads[1].center_x_m) - self.recipe.domain_margin_m)
            / self.recipe.terrain_spacing_m)
            .floor()
            * self.recipe.terrain_spacing_m;
        let right = ((pads[0].center_x_m.max(pads[1].center_x_m) + self.recipe.domain_margin_m)
            / self.recipe.terrain_spacing_m)
            .ceil()
            * self.recipe.terrain_spacing_m;
        let raw_points = self.raw_terrain.points();
        let count = ((right - left) / self.recipe.terrain_spacing_m).ceil() as usize;
        if raw_points.len() != count + 1
            || raw_points.iter().enumerate().any(|(i, p)| {
                p.x != if i == count {
                    right
                } else {
                    left + i as f64 * self.recipe.terrain_spacing_m
                }
            })
        {
            return Err("V2 raw terrain domain or sampling differs from recipe".into());
        }
        if scenario.sim != self.recipe.sim
            || scenario.vehicle != self.recipe.vehicle
            || scenario.world.gravity_mps2 != self.recipe.gravity_mps2
            || scenario.seed != self.proposal.seed
            || !in_range_v2(pads[0].center_x_m, self.recipe.source_x_m)
            || !in_range_v2(pads[0].width_m, self.recipe.source_width_m)
            || !in_range_v2(pads[1].width_m, self.recipe.target_width_m)
            || !in_range_v2(
                (pads[1].center_x_m - pads[0].center_x_m) * f64::from(self.direction),
                self.recipe.travel_distance_m,
            )
            || pads.iter().any(|pad| {
                pad.surface_y_m != millimetres(self.raw_terrain.sample_height(pad.center_x_m))
            })
            || scenario.mission.transfer_route.is_some()
            || scenario.mission.goal
                != (EvaluationGoal::LandingOnPad {
                    target_pad_id: "target".into(),
                })
            || scenario.initial_state != source_rest(&self.recipe, &pads[0])
        {
            return Err("V2 resolved mission differs from declared physical tier".into());
        }
        let (prepared, edits) =
            prepare_pads_v2(&self.raw_terrain, pads, self.recipe.pad_transition_m)?;
        if prepared != scenario.world.terrain || edits != self.geometry.pad_edit_intervals_x_m {
            return Err("V2 prepared terrain differs from its bounded raw pad edits".into());
        }
        self.geometry.verify(scenario)?;
        if self.raw_geometry
            != measure_transfer_geometry(
                &self.raw_terrain,
                &pads[0],
                &pads[1],
                &self.geometry.spec,
                &[],
            )?
        {
            return Err("V2 raw geometry receipt mismatch".into());
        }
        Ok(())
    }

    /// Explicit input-QA regeneration at equal/finer resolution. This is not the
    /// collection reader and does not replace the authoritative physical polyline.
    pub fn raw_reference(&self, spacing_m: f64) -> Result<TerrainDefinition, String> {
        self.validate()?;
        if !spacing_m.is_finite() || spacing_m < 1.0 || spacing_m > self.recipe.terrain_spacing_m {
            return Err("reference spacing must be 1 m through the resolved spacing".into());
        }
        let points = self.raw_terrain.points();
        sample_raw(
            &self.recipe,
            self.terrain_parameters,
            self.terrain_seed,
            points[0].x,
            points.last().unwrap().x,
            spacing_m,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralRejectionV2 {
    OverlappingPadEdits,
    InvalidScenario,
    ObstructedSourceRest,
    GeometryUnmeasurable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectedProposalV2 {
    pub proposal: ProposalKeyV1,
    pub reason: StructuralRejectionV2,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationResultV2 {
    pub mission: Option<ResolvedMissionV2>,
    pub rejections: Vec<RejectedProposalV2>,
}

pub fn generate_mission_v2(
    recipe: &MissionRecipeV2,
    spec: &GeometrySpecV1,
    seed: u64,
    index: u64,
) -> Result<GenerationResultV2, String> {
    recipe.validate()?;
    spec.validate()?;
    let mut rejections = Vec::new();
    for attempt in 0..recipe.max_attempts {
        let proposal = ProposalKeyV1 {
            seed,
            index,
            attempt,
        };
        match propose_v2(recipe, spec, proposal) {
            Ok(mission) => {
                return Ok(GenerationResultV2 {
                    mission: Some(mission),
                    rejections,
                });
            }
            Err((reason, detail)) => rejections.push(RejectedProposalV2 {
                proposal,
                reason,
                detail,
            }),
        }
    }
    Ok(GenerationResultV2 {
        mission: None,
        rejections,
    })
}

fn direction_v2(index: u64) -> i32 {
    if index.is_multiple_of(2) { 1 } else { -1 }
}
fn millimetres(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

struct RngV2(u64);
impl RngV2 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix_v2(self.0)
    }
    fn draw(&mut self, range: RangeV1) -> f64 {
        range.min + (range.max - range.min) * ((self.next() >> 11) as f64 / ((1_u64 << 53) as f64))
    }
}

fn source_rest(recipe: &MissionRecipeV2, source: &LandingPadSpec) -> VehicleInitialState {
    VehicleInitialState {
        position_m: Vec2::new(
            source.center_x_m,
            source.surface_y_m + recipe.vehicle.geometry.touchdown_base_offset_m,
        ),
        velocity_mps: Vec2::new(0.0, 0.0),
        attitude_rad: 0.0,
        angular_rate_radps: 0.0,
    }
}

fn sample_raw(
    recipe: &MissionRecipeV2,
    controls: TerrainControlsV2<f64>,
    seed: u64,
    left: f64,
    right: f64,
    spacing: f64,
) -> Result<TerrainDefinition, String> {
    let count = ((right - left) / spacing).ceil() as usize;
    if count == 0 || count > 100000 {
        return Err("raw/reference sampling exceeds bounded work".into());
    }
    let sampler = TerrainSamplerV2::new(recipe.backend, seed, controls, recipe.octaves);
    let mut points: Vec<_> = (0..count)
        .map(|i| {
            let x = left + i as f64 * spacing;
            Vec2::new(x, sampler.height(x))
        })
        .collect();
    points.push(Vec2::new(right, sampler.height(right)));
    let terrain = TerrainDefinition::Heightfield { points_m: points };
    terrain.validate()?;
    Ok(terrain)
}

fn propose_v2(
    recipe: &MissionRecipeV2,
    spec: &GeometrySpecV1,
    proposal: ProposalKeyV1,
) -> Result<ResolvedMissionV2, (StructuralRejectionV2, String)> {
    let invalid = |detail| (StructuralRejectionV2::InvalidScenario, detail);
    let mut rng = RngV2(mix_v2(
        proposal.seed
            ^ mix_v2(proposal.index)
            ^ mix_v2(u64::from(proposal.attempt).wrapping_add(0xf292_3117)),
    ));
    let source_x = rng.draw(recipe.source_x_m);
    let distance = rng.draw(recipe.travel_distance_m);
    let direction = direction_v2(proposal.index);
    let target_x = source_x + f64::from(direction) * distance;
    let source_width = rng.draw(recipe.source_width_m);
    let target_width = rng.draw(recipe.target_width_m);
    if distance <= (source_width + target_width) * 0.5 + 2.0 * recipe.pad_transition_m + 1e-6 {
        return Err((
            StructuralRejectionV2::OverlappingPadEdits,
            "source and target local edit intervals overlap".into(),
        ));
    }
    let parameters = recipe.terrain.map(|r| rng.draw(r));
    let terrain_seed = rng.next();
    let left = ((source_x.min(target_x) - recipe.domain_margin_m) / recipe.terrain_spacing_m)
        .floor()
        * recipe.terrain_spacing_m;
    let right = ((source_x.max(target_x) + recipe.domain_margin_m) / recipe.terrain_spacing_m)
        .ceil()
        * recipe.terrain_spacing_m;
    let raw = sample_raw(
        recipe,
        parameters,
        terrain_seed,
        left,
        right,
        recipe.terrain_spacing_m,
    )
    .map_err(invalid)?;
    let pads = vec![
        LandingPadSpec {
            id: "source".into(),
            center_x_m: source_x,
            surface_y_m: millimetres(raw.sample_height(source_x)),
            width_m: source_width,
        },
        LandingPadSpec {
            id: "target".into(),
            center_x_m: target_x,
            surface_y_m: millimetres(raw.sample_height(target_x)),
            width_m: target_width,
        },
    ];
    let (prepared, edits) =
        prepare_pads_v2(&raw, &pads, recipe.pad_transition_m).map_err(invalid)?;
    let initial_state = source_rest(recipe, &pads[0]);
    let scenario = ScenarioSpec {
        id: format!(
            "procedural-v2-{:016x}-{:08}-{}",
            proposal.seed, proposal.index, proposal.attempt
        ),
        name: format!("Procedural V2 transfer {}", proposal.index),
        description: "Complete procedural source-rest transfer; structural validity only".into(),
        seed: proposal.seed,
        tags: vec!["procedural".into(), "source_rest".into()],
        metadata: BTreeMap::from([
            ("generator".into(), recipe.generator_version.clone()),
            ("backend".into(), recipe.backend.key().into()),
        ]),
        sim: recipe.sim.clone(),
        world: WorldSpec {
            gravity_mps2: recipe.gravity_mps2,
            terrain: prepared,
            landing_pads: pads,
        },
        vehicle: recipe.vehicle.clone(),
        initial_state,
        mission: MissionSpec {
            transfer_route: None,
            goal: EvaluationGoal::LandingOnPad {
                target_pad_id: "target".into(),
            },
        },
    };
    scenario.validate().map_err(invalid)?;
    let bottom =
        scenario.initial_state.position_m.y - scenario.vehicle.geometry.hull_height_m * 0.5;
    if [-1.0, 0.0, 1.0].into_iter().any(|sign| {
        scenario
            .world
            .terrain
            .sample_height(source_x + sign * scenario.vehicle.geometry.hull_width_m * 0.5)
            > bottom + 1e-9
    }) {
        return Err((
            StructuralRejectionV2::ObstructedSourceRest,
            "source-rest hull intersects terrain".into(),
        ));
    }
    let geometry = measure_mission_geometry(&scenario, spec, &edits)
        .map_err(|e| (StructuralRejectionV2::GeometryUnmeasurable, e))?;
    let raw_geometry = measure_transfer_geometry(
        &raw,
        &scenario.world.landing_pads[0],
        &scenario.world.landing_pads[1],
        spec,
        &[],
    )
    .map_err(|e| (StructuralRejectionV2::GeometryUnmeasurable, e))?;
    Ok(ResolvedMissionV2 {
        schema_version: 2,
        recipe: recipe.clone(),
        recipe_sha256: recipe.identity().map_err(invalid)?,
        proposal,
        direction,
        terrain_seed,
        terrain_parameters: parameters,
        raw_terrain_sha256: sha256(&serde_json::to_vec(&raw).map_err(|e| invalid(e.to_string()))?),
        raw_terrain: raw,
        raw_geometry,
        physical_sha256: geometry.physical_sha256.clone(),
        geometry,
        scenario,
    })
}
