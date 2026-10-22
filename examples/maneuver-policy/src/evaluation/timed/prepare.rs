use crate::{
    evaluation::{
        BoundFileV1, EvaluationBudgetV1, bind_file, identity, read_bound_json, read_json,
        write_limited,
    },
    poly::{TimedProgramV2, controller},
};
use pd_control::{
    maneuver::*,
    poly_landing::{LandingKinematicsV1, build_reference, paired_mean},
};
use pd_core::{Command, RunContext, ScenarioSpec, TerrainDefinition, Vec2};
use pd_missions::{physical_identity, sha256};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

pub const SAMPLER: &str = "poly-flat-reference-defined-hash-v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatRecipeV2 {
    pub schema_version: u32,
    pub sampler: String,
    pub phase: String,
    pub cohort: String,
    pub seed: u64,
    pub originals: usize,
    pub max_proposals: usize,
    pub development_extensions: bool,
    pub budget: EvaluationBudgetV1,
    /// An eligible confirmation's prior frozen development acceptance receipt.
    pub release_gate: Option<BoundFileV1>,
    pub exposure_plans: Vec<BoundFileV1>,
}
impl FlatRecipeV2 {
    pub fn validate(&self) -> Result<(), String> {
        self.budget.validate()?;
        let ceiling = match self.phase.as_str() {
            "pilot" => 12,
            "development" => 256,
            "confirmation" => 600,
            _ => return Err("unknown timed phase".into()),
        };
        if self.schema_version != 2
            || self.sampler != SAMPLER
            || self.cohort.trim().is_empty()
            || self.originals == 0
            || self.originals > ceiling
            || self.max_proposals != self.originals * 8
            || (self.development_extensions && (self.phase != "development" || self.originals < 16))
            || (self.phase == "confirmation"
                && (self.originals != 600 || self.release_gate.is_none()))
        {
            return Err("invalid timed recipe/count/phase/release contract".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedCaseV2 {
    pub id: String,
    pub role: String,
    pub stratum: String,
    pub parent: Option<String>,
    pub independent: bool,
    pub scenario: BoundFileV1,
    pub program: BoundFileV1,
    pub physical_sha256: String,
    pub native_support: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedEvaluationPlanV2 {
    pub schema_version: u32,
    pub recipe_file: BoundFileV1,
    pub recipe: FlatRecipeV2,
    pub anchor: BoundFileV1,
    pub source_files: Vec<pd_eval::FileIdentityV1>,
    pub cases: Vec<TimedCaseV2>,
    pub proposals: Vec<serde_json::Value>,
    pub coverage: BTreeMap<String, usize>,
    pub preparation_complete: bool,
    pub exposure_overlaps: Vec<String>,
}

fn unit(recipe: &FlatRecipeV2, index: usize, axis: usize) -> Result<f64, String> {
    let hash = identity(&(SAMPLER, recipe.seed, &recipe.cohort, index, axis))?;
    let bits = u64::from_str_radix(&hash[..16], 16).map_err(|e| e.to_string())?;
    Ok((bits >> 11) as f64 / ((1_u64 << 53) as f64))
}

pub fn flat_proposal(
    anchor: &ScenarioSpec,
    recipe: &FlatRecipeV2,
    index: usize,
) -> Result<(ScenarioSpec, TimedProgramV2), String> {
    let mut u: Vec<_> = (0..6)
        .map(|axis| unit(recipe, index, axis))
        .collect::<Result<_, _>>()?;
    if recipe.phase == "development" && index < 64 {
        for (axis, value) in u.iter_mut().enumerate() {
            *value = ((index >> axis) & 1) as f64;
        }
    } else if recipe.phase == "development" && index < 76 {
        u.fill(0.5);
        u[(index - 64) / 2] = ((index - 64) % 2) as f64;
    }
    let ticks = 2 * (((8.0 + 16.0 * u[0]) * 60.0).floor() as u64);
    let time = ticks as f64 / 120.0;
    let vx = (0.5 + 2.5 * u[1]) * time;
    let sink = 8.0 + 47.0 * u[2];
    let dx = (0.28 + 0.14 * u[3]) * vx * time;
    let mixture = 0.4 + 0.2 * u[4];
    let height = time * (mixture * sink + (1.0 - mixture) * 1.5);
    let mut s = anchor.clone();
    s.id = format!("poly-{}-{index}", recipe.cohort);
    s.name = s.id.clone();
    s.description =
        "Synthetic flat, prepared, timed local task; reference-defined conditional population"
            .into();
    s.seed = u64::from_str_radix(&identity(&(recipe.seed, &recipe.cohort, index))?[..16], 16)
        .map_err(|e| e.to_string())?;
    s.tags = vec![
        "poly_landing".into(),
        "flat".into(),
        "synthetic_local".into(),
    ];
    s.metadata = BTreeMap::from([
        ("sampler".into(), SAMPLER.into()),
        ("cohort".into(), recipe.cohort.clone()),
    ]);
    s.sim.max_time_s = 40.0;
    s.sim.sample_hz = Some(120);
    s.world.terrain = TerrainDefinition::Heightfield {
        points_m: vec![Vec2::new(-dx - 80.0, 0.0), Vec2::new(80.0, 0.0)],
    };
    let mut pad = s
        .world
        .landing_pads
        .iter()
        .find(|p| p.id == s.mission.goal.target_pad_id())
        .ok_or("anchor target missing")?
        .clone();
    pad.center_x_m = 0.0;
    pad.surface_y_m = 0.0;
    pad.width_m = 36.0;
    s.world.landing_pads = vec![pad.clone()];
    s.mission.transfer_route = None;
    s.vehicle.initial_fuel_kg = 1800.0 + 4500.0 * u[5];
    s.initial_state.position_m = Vec2::new(-dx, 4.995 + height);
    s.initial_state.velocity_mps = Vec2::new(vx, -sink);
    s.initial_state.angular_rate_radps = 0.0;
    let ctx = RunContext::from_scenario(&s)?;
    let reference = build_reference(
        &ctx,
        LandingKinematicsV1 {
            position_m: s.initial_state.position_m,
            velocity_mps: s.initial_state.velocity_mps,
        },
        LandingKinematicsV1 {
            position_m: Vec2::new(0.0, 4.995),
            velocity_mps: Vec2::new(0.0, -1.5),
        },
        ticks,
    )?;
    let thrust = paired_mean(&reference, &ctx, 0);
    s.initial_state.attitude_rad = thrust.x.atan2(thrust.y);
    let program = TimedProgramV2 {
        schema_version: 3,
        backend: CHECKED_POLY_LANDING_BACKEND.into(),
        activation_physics_step: 0,
        absolute_deadline_physics_step: 4800,
        command_coverage_end_physics_step: Some(ticks + 4),
        request: ManeuverRequestV2 {
            schema_version: 2,
            id: "local-poly-landing".into(),
            goal: ManeuverGoalV2::LandOnMissionTargetAt {
                target_pad_id: pad.id,
                reference_end_physics_step: ticks,
            },
        },
        regression_prefix: vec![],
        rejection_coast: Command::idle(),
    };
    Ok((s, program))
}

/// Pure algebraic input predicate. It does not instantiate/update a controller
/// or call a physics transition. Every queried native grid point is counted.
pub fn input_admission(
    s: &ScenarioSpec,
    p: &TimedProgramV2,
    grid_queries: &mut u64,
) -> Result<(), String> {
    let ctx = RunContext::from_scenario(s)?;
    let ManeuverGoalV2::LandOnMissionTargetAt {
        reference_end_physics_step: ticks,
        ..
    } = p.request.goal
    else {
        return Err("not a timed goal".into());
    };
    let reference = build_reference(
        &ctx,
        LandingKinematicsV1 {
            position_m: s.initial_state.position_m,
            velocity_mps: s.initial_state.velocity_mps,
        },
        LandingKinematicsV1 {
            position_m: Vec2::new(0.0, 4.995),
            velocity_mps: Vec2::new(0.0, -1.5),
        },
        ticks,
    )?;
    let duration = (ticks + 4) as f64 / 120.0;
    let remaining = s.vehicle.initial_fuel_kg - s.vehicle.max_fuel_burn_kgps * duration;
    if remaining <= 0.0 {
        return Err("conservative fuel bound".into());
    }
    let lower =
        s.vehicle.max_thrust_n * s.vehicle.min_throttle_frac / (s.vehicle.dry_mass_kg + remaining);
    let upper =
        0.925 * s.vehicle.max_thrust_n / (s.vehicle.dry_mass_kg + s.vehicle.initial_fuel_kg);
    let mut previous = s.initial_state.attitude_rad;
    for tick in 0..ticks + 4 {
        *grid_queries += 1;
        let state = reference.state_at(&ctx, tick);
        if state.velocity_mps.x < -1.0e-9 || state.velocity_mps.y > 1.0e-9 {
            return Err("reference velocity shape".into());
        }
        if tick.is_multiple_of(2) {
            let thrust = paired_mean(&reference, &ctx, tick);
            if thrust.y <= 0.0 || thrust.length() < lower || thrust.length() > upper {
                return Err("conservative coupled authority bound".into());
            }
            let angle = thrust.x.atan2(thrust.y);
            if (angle - previous).abs() > s.vehicle.max_rotation_rate_radps / 120.0 {
                return Err("one-physics-tick servo bound".into());
            }
            previous = angle;
        }
    }
    let angle = s.initial_state.attitude_rad;
    if s.initial_state.position_m.y - 5.0 * angle.cos().abs() - 4.0 * angle.sin().abs() <= 0.0 {
        return Err("initial body intersects flat plane".into());
    }
    Ok(())
}

struct CaseRole<'a> {
    role: &'a str,
    stratum: &'a str,
    parent: Option<String>,
    independent: bool,
}
fn add_case(
    output: &Path,
    cases: &mut Vec<TimedCaseV2>,
    s: &ScenarioSpec,
    p: &TimedProgramV2,
    role: CaseRole<'_>,
) -> Result<(), String> {
    let index = cases.len();
    let sp = output.join(format!("inputs/{index}-scenario.json"));
    let pp = output.join(format!("inputs/{index}-program.json"));
    write_limited(&sp, s, 1024 * 1024)?;
    write_limited(&pp, p, 1024 * 1024)?;
    let support = match controller(s, p) {
        Ok(_) => "supported".into(),
        Err(reason) => reason,
    };
    cases.push(TimedCaseV2 {
        id: s.id.clone(),
        role: role.role.into(),
        stratum: role.stratum.into(),
        parent: role.parent,
        independent: role.independent,
        scenario: bind_file(&sp, 1024 * 1024)?,
        program: bind_file(&pp, 1024 * 1024)?,
        physical_sha256: physical_identity(s).map_err(|e| e.to_string())?,
        native_support: support,
    });
    Ok(())
}

pub fn prepare_timed_evaluation(
    recipe_path: &Path,
    anchor_path: &Path,
    output: &Path,
) -> Result<TimedEvaluationPlanV2, String> {
    let recipe: FlatRecipeV2 = read_json(recipe_path, 1024 * 1024)?;
    recipe.validate()?;
    let anchor: ScenarioSpec = read_json(anchor_path, 16 * 1024 * 1024)?;
    let source = crate::source_scope(vec![]).snapshot()?;
    crate::check_compiled_sources(&source)?;
    let mut exposed = std::collections::BTreeSet::new();
    for bound in &recipe.exposure_plans {
        let prior: TimedEvaluationPlanV2 = read_bound_json(bound, 32 * 1024 * 1024)?;
        exposed.extend(prior.cases.into_iter().map(|case| case.physical_sha256));
    }
    if let Some(gate) = &recipe.release_gate {
        let value: super::TimedEvaluationSummaryV2 = read_bound_json(gate, 8 * 1024 * 1024)?;
        if !value.eligible_for_confirmation
            || !value.complete
            || value.phase != "development"
            || value.sampler != SAMPLER
            || value.source_files != source.source_files
            || !recipe
                .exposure_plans
                .iter()
                .any(|p| p.sha256 == value.plan_sha256)
        {
            return Err("confirmation release gate is not eligible for this sampler".into());
        }
    }
    crate::verify::no_symlinks(output.parent().ok_or("prepared output needs a parent")?)?;
    std::fs::create_dir(output).map_err(|e| e.to_string())?;
    std::fs::create_dir(output.join("inputs")).map_err(|e| e.to_string())?;
    let mut cases = Vec::new();
    let mut proposals = Vec::new();
    let mut parents = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut coverage = BTreeMap::new();
    for index in 0..recipe.max_proposals {
        if parents.len() == recipe.originals {
            break;
        }
        let (s, p) = flat_proposal(&anchor, &recipe, index)?;
        let physical = physical_identity(&s).map_err(|e| e.to_string())?;
        let mut grid_queries = 0;
        let admission = input_admission(&s, &p, &mut grid_queries).and_then(|()| {
            if !seen.insert(physical.clone()) {
                Err("duplicate physical proposal".into())
            } else {
                Ok(())
            }
        });
        proposals.push(
            serde_json::json!({"proposal_index":index,"physical_sha256":physical,
            "admission":admission,"scenario":s,"program":p,"pure_reference_constructions":2,
            "pure_reference_grid_queries":grid_queries,"native_forecast_steps":0}),
        );
        if admission.is_ok() {
            let ticks = match p.request.goal {
                ManeuverGoalV2::LandOnMissionTargetAt {
                    reference_end_physics_step,
                    ..
                } => reference_end_physics_step,
                _ => unreachable!(),
            };
            let stratum = if ticks < 1440 {
                "8-12s"
            } else if ticks < 2160 {
                "12-18s"
            } else {
                "18-24s"
            };
            *coverage.entry(stratum.into()).or_insert(0) += 1;
            add_case(
                output,
                &mut cases,
                &s,
                &p,
                CaseRole {
                    role: "primary",
                    stratum,
                    parent: None,
                    independent: recipe.phase == "confirmation",
                },
            )?;
            parents.push((s, p));
        }
    }
    if recipe.development_extensions && parents.len() == recipe.originals {
        add_extensions(output, &mut cases, &parents)?;
    }
    let after = crate::source_scope(vec![]).snapshot()?;
    if after != source {
        return Err("source changed during pure timed preparation".into());
    }
    let exposure_overlaps = cases
        .iter()
        .filter(|case| exposed.contains(&case.physical_sha256))
        .map(|case| case.id.clone())
        .collect::<Vec<_>>();
    let plan = TimedEvaluationPlanV2 {
        schema_version: 2,
        recipe_file: bind_file(recipe_path, 1024 * 1024)?,
        recipe,
        anchor: bind_file(anchor_path, 16 * 1024 * 1024)?,
        source_files: source.source_files,
        cases,
        proposals,
        coverage,
        preparation_complete: false,
        exposure_overlaps,
    };
    let plan = TimedEvaluationPlanV2 {
        preparation_complete: parents.len() == plan.recipe.originals
            && plan.exposure_overlaps.is_empty(),
        ..plan
    };
    write_limited(&output.join("plan.json"), &plan, 32 * 1024 * 1024)?;
    write_limited(
        &output.join("plan-identity.json"),
        &serde_json::json!({"sha256":sha256(&std::fs::read(output.join("plan.json")).map_err(|e|e.to_string())?)}),
        1024,
    )?;
    if !plan.preparation_complete {
        return Err("timed preparation underfilled; every proposal retained".into());
    }
    Ok(plan)
}

fn add_extensions(
    output: &Path,
    cases: &mut Vec<TimedCaseV2>,
    parents: &[(ScenarioSpec, TimedProgramV2)],
) -> Result<(), String> {
    for kind in 0..8 {
        for (ordinal, (parent, p)) in parents.iter().take(12).enumerate() {
            let mut s = parent.clone();
            let mut p = p.clone();
            let label = match kind {
                0 => {
                    s.initial_state.attitude_rad += 0.6;
                    s.initial_state.angular_rate_radps = 0.8;
                    "attitude-rate"
                }
                1 => {
                    s.initial_state.velocity_mps.x = -s.initial_state.velocity_mps.x;
                    "receding"
                }
                2 => {
                    let a = s.initial_state.attitude_rad;
                    s.initial_state.position_m.y = 5.0 * a.cos().abs() + 4.0 * a.sin().abs() + 0.25;
                    "near-contact"
                }
                3 => {
                    s.vehicle.initial_fuel_kg = 5.0;
                    "low-fuel"
                }
                4 => {
                    if let ManeuverGoalV2::LandOnMissionTargetAt {
                        reference_end_physics_step,
                        ..
                    } = &mut p.request.goal
                    {
                        *reference_end_physics_step = 240;
                        p.command_coverage_end_physics_step = Some(244);
                    }
                    "short-time"
                }
                5 => {
                    s.world.landing_pads[0].width_m = 8.1;
                    "narrow-pad"
                }
                6 => {
                    s.vehicle.dry_mass_kg = 9000.0;
                    "other-mass"
                }
                _ => {
                    s.initial_state.velocity_mps.y = s.initial_state.velocity_mps.y.abs();
                    "ascending"
                }
            };
            s.id = format!("{}-diagnostic-{label}-{ordinal}", parent.id);
            s.name = s.id.clone();
            add_case(
                output,
                cases,
                &s,
                &p,
                CaseRole {
                    role: "diagnostic",
                    stratum: label,
                    parent: Some(parent.id.clone()),
                    independent: false,
                },
            )?;
        }
    }
    for kind in 0..3 {
        for (ordinal, (parent, p)) in parents.iter().take(16).enumerate() {
            let mut s = parent.clone();
            let label = match kind {
                0 => {
                    s.initial_state.position_m.x = -s.initial_state.position_m.x;
                    s.initial_state.velocity_mps.x = -s.initial_state.velocity_mps.x;
                    s.initial_state.attitude_rad = -s.initial_state.attitude_rad;
                    s.initial_state.angular_rate_radps = -s.initial_state.angular_rate_radps;
                    let TerrainDefinition::Heightfield { points_m } = &mut s.world.terrain;
                    for point in points_m.iter_mut() {
                        point.x = -point.x;
                    }
                    points_m.reverse();
                    "reflect"
                }
                1 => {
                    s.initial_state.position_m += Vec2::new(250.0, 120.0);
                    for pad in &mut s.world.landing_pads {
                        pad.center_x_m += 250.0;
                        pad.surface_y_m += 120.0;
                    }
                    let TerrainDefinition::Heightfield { points_m } = &mut s.world.terrain;
                    for point in points_m {
                        *point += Vec2::new(250.0, 120.0);
                    }
                    "translate"
                }
                _ => {
                    s.seed = s.seed.wrapping_add(12345);
                    "relabel"
                }
            };
            s.id = format!("{}-transform-{label}-{ordinal}", parent.id);
            s.name = s.id.clone();
            add_case(
                output,
                cases,
                &s,
                p,
                CaseRole {
                    role: "transform",
                    stratum: label,
                    parent: Some(parent.id.clone()),
                    independent: false,
                },
            )?;
        }
    }
    Ok(())
}

pub fn check_timed_evaluation(root: &Path) -> Result<TimedEvaluationPlanV2, String> {
    let plan: TimedEvaluationPlanV2 = read_json(&root.join("plan.json"), 32 * 1024 * 1024)?;
    plan.recipe.validate()?;
    let seal: serde_json::Value = read_json(&root.join("plan-identity.json"), 1024)?;
    if seal["sha256"] != sha256(&std::fs::read(root.join("plan.json")).map_err(|e| e.to_string())?)
        || plan.schema_version != 2
        || !plan.preparation_complete
    {
        return Err("timed plan identity/completeness mismatch".into());
    }
    let recipe: FlatRecipeV2 = read_bound_json(&plan.recipe_file, 1024 * 1024)?;
    let _: ScenarioSpec = read_bound_json(&plan.anchor, 16 * 1024 * 1024)?;
    for exposure in &recipe.exposure_plans {
        let _: TimedEvaluationPlanV2 = read_bound_json(exposure, 32 * 1024 * 1024)?;
    }
    if recipe != plan.recipe {
        return Err("bound timed recipe differs".into());
    }
    let primary = plan
        .cases
        .iter()
        .filter(|case| case.role == "primary")
        .count();
    let diagnostic = plan
        .cases
        .iter()
        .filter(|case| case.role == "diagnostic")
        .count();
    let transforms = plan
        .cases
        .iter()
        .filter(|case| case.role == "transform")
        .count();
    if primary != recipe.originals
        || diagnostic != if recipe.development_extensions { 96 } else { 0 }
        || transforms != if recipe.development_extensions { 48 } else { 0 }
        || plan.cases.len() != primary + diagnostic + transforms
        || plan.proposals.len() > recipe.max_proposals
    {
        return Err("timed role/proposal schedule differs from declaration".into());
    }
    for case in &plan.cases {
        let s: ScenarioSpec = read_bound_json(&case.scenario, 1024 * 1024)?;
        let p: TimedProgramV2 = read_bound_json(&case.program, 1024 * 1024)?;
        p.validate()?;
        if s.id != case.id
            || physical_identity(&s).map_err(|e| e.to_string())? != case.physical_sha256
        {
            return Err("timed case identity differs".into());
        }
    }
    Ok(plan)
}
