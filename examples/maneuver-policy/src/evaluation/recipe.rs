use super::identity;
use crate::{ActivationProgramV1, ActivationV1};
use pd_core::{RunContext, ScenarioSpec, TerrainDefinition, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntervalV1 {
    pub min: f64,
    pub max: f64,
}
impl IntervalV1 {
    pub fn validate(&self) -> Result<(), String> {
        if !self.min.is_finite() || !self.max.is_finite() || self.min > self.max {
            Err("invalid finite interval".into())
        } else {
            Ok(())
        }
    }
    pub fn at(&self, u: f64) -> f64 {
        self.min + (self.max - self.min) * u
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalBoundsV1 {
    pub clearance_m: IntervalV1,
    pub offset_per_clearance: IntervalV1,
    pub lateral_residual_mps: IntervalV1,
    /// Downward speed / sqrt(2 * available net upright acceleration * clearance).
    pub braking_speed_ratio: IntervalV1,
    pub attitude_rad: IntervalV1,
    pub angular_rate_radps: IntervalV1,
    pub dry_mass_kg: Vec<f64>,
    pub fuel_kg: IntervalV1,
    pub pad_width_m: IntervalV1,
    pub pad_surface_y_m: IntervalV1,
    pub deadline_s: IntervalV1,
    pub terrain: Vec<TerrainProfileV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TerrainProfileV1 {
    Flat {},
    ApproachRidge {
        position_fraction: f64,
        height_m: f64,
        half_width_m: f64,
    },
    Backstop {
        distance_m: f64,
        height_m: f64,
        half_width_m: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransformV1 {
    Identity {},
    Reflect {},
    Translate { x_m: f64, y_m: f64 },
    Relabel { seed_delta: u64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticV1 {
    Fuel { kg: f64 },
    Deadline { seconds: f64 },
    Ascending { vertical_mps: f64 },
    NarrowPad { width_m: f64 },
    UnsupportedGravity { mps2: f64 },
    BelowTarget { surface_y_m: f64 },
    NearContact { clearance_m: f64, descent_mps: f64 },
    Attitude { angle_rad: f64, rate_radps: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PopulationV1 {
    Retained {
        seeds: Vec<u64>,
    },
    Local {
        anchor_id: String,
        cohort: String,
        seed: u64,
        indices: Vec<u64>,
        independent: bool,
        bounds: Box<LocalBoundsV1>,
        transforms: Vec<TransformV1>,
        /// Structured inputs have four development cases for each diagnostic.
        diagnostics: Vec<DiagnosticV1>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationBudgetV1 {
    pub max_wall_seconds: u64,
    pub max_attempt_wall_seconds: u64,
    pub max_output_bytes: u64,
    pub max_bundle_bytes: u64,
    pub min_free_bytes: u64,
}
impl EvaluationBudgetV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_wall_seconds == 0
            || self.max_wall_seconds > 12 * 3600
            || self.max_attempt_wall_seconds == 0
            || self.max_attempt_wall_seconds > self.max_wall_seconds
            || self.max_output_bytes == 0
            || self.max_output_bytes > 40 * 1024_u64.pow(3)
            || self.max_bundle_bytes < 1024 * 1024
            || self.max_bundle_bytes > 128 * 1024 * 1024
            || self.max_output_bytes < self.max_bundle_bytes
        {
            return Err("invalid finite evaluation budget".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationRecipeV1 {
    pub schema_version: u32,
    pub task_id: String,
    pub population: PopulationV1,
    pub programs: Vec<ActivationProgramV1>,
    pub direct_parity: bool,
    pub budget: EvaluationBudgetV1,
    pub exposure_inventories: Vec<super::BoundFileV1>,
    pub audit_bins: std::collections::BTreeMap<String, Vec<f64>>,
    pub joint_groups: Vec<Vec<String>>,
}
impl EvaluationRecipeV1 {
    pub fn validate(&self) -> Result<(), String> {
        self.budget.validate()?;
        for (name, edges) in &self.audit_bins {
            if name.is_empty()
                || edges.len() > 32
                || edges.iter().any(|v| !v.is_finite())
                || edges.windows(2).any(|w| w[0] >= w[1])
            {
                return Err("invalid audit bins".into());
            }
        }
        for group in &self.joint_groups {
            if !(2..=4).contains(&group.len())
                || group.iter().any(|name| !self.audit_bins.contains_key(name))
                || group.iter().collect::<BTreeSet<_>>().len() != group.len()
            {
                return Err("invalid joint coverage axes".into());
            }
        }
        if self.schema_version != 1
            || self.task_id.is_empty()
            || self.task_id.len() > 128
            || self.programs.is_empty()
            || self.programs.len() > 7
        {
            return Err("recipe needs version 1, task identity and 1..7 programs".into());
        }
        let mut seen = BTreeSet::new();
        for program in &self.programs {
            program.validate()?;
            if !seen.insert(identity(program)?) {
                return Err("duplicate activation program".into());
            }
            if self.direct_parity && !matches!(program.activation, ActivationV1::Immediate {}) {
                return Err("direct parity requires immediate activation".into());
            }
        }
        match &self.population {
            PopulationV1::Retained { seeds } => {
                if seeds.len() > 12
                    || seeds.iter().any(|v| *v > 11)
                    || seeds.iter().collect::<BTreeSet<_>>().len() != seeds.len()
                {
                    return Err("invalid retained seed selection".into());
                }
            }
            PopulationV1::Local {
                anchor_id,
                cohort,
                indices,
                independent,
                bounds,
                transforms,
                diagnostics,
                ..
            } => {
                bounds.validate()?;
                if anchor_id.is_empty()
                    || cohort.is_empty()
                    || cohort.len() > 64
                    || indices.is_empty()
                    || indices.len() > 2048
                    || indices.iter().any(|v| *v > 1_000_000)
                    || indices.iter().collect::<BTreeSet<_>>().len() != indices.len()
                    || transforms.is_empty()
                    || transforms.len() > 8
                    || diagnostics.len() > 16
                {
                    return Err("invalid local population schedule".into());
                }
                if *independent
                    && (!diagnostics.is_empty() || transforms != &[TransformV1::Identity {}])
                {
                    return Err(
                        "independent cohort forbids derived transforms and diagnostics".into(),
                    );
                }
                for transform in transforms {
                    if let TransformV1::Translate { x_m, y_m } = transform
                        && (!x_m.is_finite() || !y_m.is_finite())
                    {
                        return Err("nonfinite translation".into());
                    }
                }
                // Validate every diagnostic numeric field without permitting arbitrary patches.
                let wire = serde_json::to_value(diagnostics).map_err(|e| e.to_string())?;
                if wire.to_string().contains("null") {
                    return Err("nonfinite diagnostic".into());
                }
            }
        }
        Ok(())
    }
}
impl LocalBoundsV1 {
    pub fn validate(&self) -> Result<(), String> {
        for bound in [
            &self.clearance_m,
            &self.offset_per_clearance,
            &self.lateral_residual_mps,
            &self.braking_speed_ratio,
            &self.attitude_rad,
            &self.angular_rate_radps,
            &self.fuel_kg,
            &self.pad_width_m,
            &self.pad_surface_y_m,
            &self.deadline_s,
        ] {
            bound.validate()?;
        }
        if self.clearance_m.min <= 0.0
            || self.fuel_kg.min <= 0.0
            || self.pad_width_m.min <= 0.0
            || self.deadline_s.min <= 0.0
            || self.deadline_s.max > 600.0
            || self.dry_mass_kg.is_empty()
            || self.dry_mass_kg.len() > 16
            || self.dry_mass_kg.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || self.terrain.is_empty()
            || self.terrain.len() > 8
        {
            return Err("unsupported local bounds".into());
        }
        for profile in &self.terrain {
            let (height, width, location) = match profile {
                TerrainProfileV1::Flat {} => continue,
                TerrainProfileV1::ApproachRidge {
                    position_fraction,
                    height_m,
                    half_width_m,
                } => (*height_m, *half_width_m, *position_fraction),
                TerrainProfileV1::Backstop {
                    distance_m,
                    height_m,
                    half_width_m,
                } => (*height_m, *half_width_m, *distance_m),
            };
            if !height.is_finite()
                || height < 0.0
                || !width.is_finite()
                || width <= 0.0
                || !location.is_finite()
                || location <= 0.0
            {
                return Err("invalid terrain profile".into());
            }
        }
        Ok(())
    }
}

/// Counter-derived SplitMix64 stream. Each mission index has its own stream;
/// cohort streams and independent scalar draws are part of sampler v1.
struct Draw(u64);
impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}

pub struct LocalCaseV1 {
    pub scenario: ScenarioSpec,
    pub role: String,
    pub stratum: String,
    pub parent: String,
    pub transform: TransformV1,
}

pub fn local_case(
    recipe: &EvaluationRecipeV1,
    anchor: &ScenarioSpec,
    index: u64,
) -> Result<LocalCaseV1, String> {
    let PopulationV1::Local {
        cohort,
        seed,
        independent,
        bounds,
        transforms,
        diagnostics,
        ..
    } = &recipe.population
    else {
        return Err("not a local recipe".into());
    };
    let base_index = index / transforms.len() as u64;
    let transform = transforms[index as usize % transforms.len()].clone();
    let diagnostic = !*independent && !diagnostics.is_empty() && base_index % 5 == 4;
    let grid_index = if !*independent && !diagnostics.is_empty() {
        (base_index / 5) * 4 + base_index % 5
    } else {
        base_index
    };
    let cohort_digest = pd_missions::sha256(cohort.as_bytes());
    let cohort_stream = u64::from_str_radix(&cohort_digest[..16], 16).map_err(|e| e.to_string())?;
    let mut draw = Draw(
        seed.wrapping_add(cohort_stream)
            .wrapping_add(base_index.wrapping_mul(0xd1342543de82ef95)),
    );
    let mut units = [0.0; 12];
    for value in &mut units {
        *value = draw.unit();
    }
    if !*independent {
        for (axis, value) in units.iter_mut().take(3).enumerate() {
            *value = ((grid_index >> (2 * axis)) & 3) as f64 / 3.0;
        }
        units[3] = ((grid_index + (grid_index >> 2) + (grid_index >> 4)) & 3) as f64 / 3.0;
    }
    let mut s = anchor.clone();
    s.id = format!("{}-{cohort}-{index}", recipe.task_id);
    s.name = s.id.clone();
    s.seed = seed.wrapping_add(index);
    let target = s.mission.goal.target_pad_id().to_owned();
    let target_index = s
        .world
        .landing_pads
        .iter()
        .position(|p| p.id == target)
        .ok_or("missing target")?;
    let h = bounds.clearance_m.at(units[0]);
    let mass_index = (units[7] * bounds.dry_mass_kg.len() as f64).floor() as usize;
    s.vehicle.dry_mass_kg = bounds.dry_mass_kg[mass_index.min(bounds.dry_mass_kg.len() - 1)];
    s.vehicle.initial_fuel_kg = bounds.fuel_kg.at(units[2]);
    s.sim.max_time_s = bounds.deadline_s.at(units[10]);
    let pad = &mut s.world.landing_pads[target_index];
    pad.width_m = bounds.pad_width_m.at(units[8]);
    pad.surface_y_m = bounds.pad_surface_y_m.at(units[9]);
    let px = pad.center_x_m;
    let py = pad.surface_y_m;
    let half_width = pad.half_width_m();
    let sign = if units[4] < 0.5 { -1.0 } else { 1.0 };
    let dx = sign * bounds.offset_per_clearance.at(units[4] * 2.0 % 1.0) * h;
    let angle = bounds.attitude_rad.at(units[3]);
    let bottom = s.vehicle.geometry.touchdown_base_offset_m * angle.cos()
        + s.vehicle.geometry.touchdown_half_span_m * angle.sin().abs();
    let net = (s.vehicle.max_thrust_n / (s.vehicle.dry_mass_kg + s.vehicle.initial_fuel_kg)
        - s.world.gravity_mps2)
        .max(0.0);
    let speed = bounds.braking_speed_ratio.at(units[1]) * (2.0 * net * h).sqrt();
    let approach_time = (2.0 * h / s.world.gravity_mps2).sqrt() + 2.0;
    s.initial_state.position_m = Vec2::new(px + dx, py + bottom + h);
    s.initial_state.velocity_mps = Vec2::new(
        -dx / approach_time + bounds.lateral_residual_mps.at(units[5]),
        -speed,
    );
    s.initial_state.attitude_rad = angle;
    s.initial_state.angular_rate_radps = bounds.angular_rate_radps.at(units[6]);
    let terrain_index =
        ((units[11] * bounds.terrain.len() as f64).floor() as usize).min(bounds.terrain.len() - 1);
    let terrain = &bounds.terrain[terrain_index];
    let (kind, peak) = match terrain {
        TerrainProfileV1::Flat {} => ("flat", None),
        TerrainProfileV1::ApproachRidge {
            position_fraction,
            height_m,
            half_width_m,
        } => (
            "approach_ridge",
            Some((
                px + sign * (dx.abs() * position_fraction).max(half_width + 2.0 * half_width_m),
                *height_m,
                *half_width_m,
            )),
        ),
        TerrainProfileV1::Backstop {
            distance_m,
            height_m,
            half_width_m,
        } => (
            "backstop",
            Some((
                px - sign * distance_m.max(half_width + 2.0 * half_width_m),
                *height_m,
                *half_width_m,
            )),
        ),
    };
    let mut points = vec![Vec2::new(px - 100_000.0, 0.0)];
    if let Some((x, height, width)) = peak {
        points.extend([
            Vec2::new(x - width, 0.0),
            Vec2::new(x, height),
            Vec2::new(x + width, 0.0),
        ]);
    }
    points.push(Vec2::new(px + 100_000.0, 0.0));
    s.world.terrain = TerrainDefinition::Heightfield { points_m: points };
    if diagnostic {
        match &diagnostics[(base_index / 5) as usize % diagnostics.len()] {
            DiagnosticV1::Fuel { kg } => s.vehicle.initial_fuel_kg = *kg,
            DiagnosticV1::Deadline { seconds } => s.sim.max_time_s = *seconds,
            DiagnosticV1::Ascending { vertical_mps } => {
                s.initial_state.velocity_mps.y = *vertical_mps
            }
            DiagnosticV1::NarrowPad { width_m } => {
                s.world.landing_pads[target_index].width_m = *width_m
            }
            DiagnosticV1::UnsupportedGravity { mps2 } => s.world.gravity_mps2 = *mps2,
            DiagnosticV1::BelowTarget { surface_y_m } => {
                s.world.landing_pads[target_index].surface_y_m = *surface_y_m
            }
            DiagnosticV1::NearContact {
                clearance_m,
                descent_mps,
            } => {
                s.initial_state.position_m = Vec2::new(px, py + bottom + clearance_m);
                s.initial_state.velocity_mps = Vec2::new(0.0, -descent_mps);
            }
            DiagnosticV1::Attitude {
                angle_rad,
                rate_radps,
            } => {
                s.initial_state.attitude_rad = *angle_rad;
                s.initial_state.angular_rate_radps = *rate_radps;
            }
        }
    }
    apply_transform(&mut s, &transform)?;
    Ok(LocalCaseV1 {
        scenario: s,
        role: if *independent {
            "confirmation"
        } else if diagnostic {
            "diagnostic"
        } else {
            "development"
        }
        .into(),
        stratum: format!(
            "mass{}-{kind}",
            bounds.dry_mass_kg[mass_index.min(bounds.dry_mass_kg.len() - 1)]
        ),
        parent: format!("local:{cohort}:{seed}:{base_index}"),
        transform,
    })
}

pub fn apply_transform(s: &mut ScenarioSpec, transform: &TransformV1) -> Result<(), String> {
    match transform {
        TransformV1::Identity {} => {}
        TransformV1::Relabel { seed_delta } => {
            s.seed = s.seed.wrapping_add(*seed_delta);
            s.name = "label-control".into();
            s.description = "labels carry no flight authority".into();
            s.tags = vec!["label-control".into()];
            s.metadata.clear();
        }
        TransformV1::Translate { x_m, y_m } => {
            if !x_m.is_finite() || !y_m.is_finite() {
                return Err("nonfinite translation".into());
            }
            s.initial_state.position_m.x += x_m;
            s.initial_state.position_m.y += y_m;
            for pad in &mut s.world.landing_pads {
                pad.center_x_m += x_m;
                pad.surface_y_m += y_m;
            }
            let TerrainDefinition::Heightfield { points_m } = &mut s.world.terrain;
            for p in points_m {
                p.x += x_m;
                p.y += y_m;
            }
        }
        TransformV1::Reflect {} => {
            s.initial_state.position_m.x = -s.initial_state.position_m.x;
            s.initial_state.velocity_mps.x = -s.initial_state.velocity_mps.x;
            s.initial_state.attitude_rad = -s.initial_state.attitude_rad;
            s.initial_state.angular_rate_radps = -s.initial_state.angular_rate_radps;
            for pad in &mut s.world.landing_pads {
                pad.center_x_m = -pad.center_x_m;
            }
            let TerrainDefinition::Heightfield { points_m } = &mut s.world.terrain;
            for p in points_m.iter_mut() {
                p.x = -p.x;
            }
            points_m.reverse();
        }
    }
    Ok(())
}

pub fn physical_features(
    s: &ScenarioSpec,
) -> Result<std::collections::BTreeMap<String, f64>, String> {
    let ctx = RunContext::from_scenario(s)?;
    let observation = pd_core::SimulationState::new(&ctx)
        .map_err(|e| e.to_string())?
        .build_observation(&ctx);
    Ok(std::collections::BTreeMap::from([
        (
            "touchdown_clearance_m".into(),
            observation.touchdown_clearance_m,
        ),
        ("hull_clearance_m".into(), observation.min_hull_clearance_m),
        (
            "target_dx_m".into(),
            s.initial_state.position_m.x - ctx.target_pad.center_x_m,
        ),
        (
            "target_height_m".into(),
            s.initial_state.position_m.y - ctx.target_pad.surface_y_m,
        ),
        ("vx_mps".into(), s.initial_state.velocity_mps.x),
        ("vy_mps".into(), s.initial_state.velocity_mps.y),
        ("attitude_rad".into(), s.initial_state.attitude_rad),
        (
            "angular_rate_radps".into(),
            s.initial_state.angular_rate_radps,
        ),
        ("dry_mass_kg".into(), s.vehicle.dry_mass_kg),
        ("fuel_kg".into(), s.vehicle.initial_fuel_kg),
        ("pad_width_m".into(), ctx.target_pad.width_m),
        ("deadline_s".into(), s.sim.max_time_s),
    ]))
}
