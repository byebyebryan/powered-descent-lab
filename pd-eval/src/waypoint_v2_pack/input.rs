use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use pd_core::{EvaluationGoal, ScenarioSpec, Vec2};
use pd_plan::waypoint_v2::WaypointV2Policy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    DEFAULT_PLANNER_PACK_PATH,
    model::{WaypointV2PackGroup, WaypointV2PackInput, WaypointV2PackInputIdentity},
    provenance::ExpandedDigestItem,
};
use crate::evidence_io::sha256_bytes;

pub(super) const PRACTICAL_PATH: &str =
    "fixtures/research/waypoint_v2_practical_suite_plan_v1.json";
const PRACTICAL_SHA256: &str = "92869e10225a72e8716ad87c20fbc1ca3795bd692aa9e41d011cd9ae18cf438c";
pub(super) const FRESH_PATH: &str = "fixtures/research/waypoint_v2_fresh_terrain_inputs_v1.json";
const FRESH_SHA256: &str = "554b406f62387e9598d89e53a26ed705b2fce5380b4e7a216f92f3003950cd9c";
const FRESH_EXPANDED_JS_SHA256: &str =
    "311752c8da362c2261ce069e633041407b6b59ff88c0c32381d21bd095128c52";
pub(super) const EXPECTED_CASE_COUNT: usize = 44;
pub(super) const MAX_WORKERS: usize = EXPECTED_CASE_COUNT;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PackDefinition {
    schema_id: String,
    pub(super) id: String,
    pub(super) name: String,
    description: String,
    #[serde(default = "default_policy_version")]
    pub(super) policy_version: u8,
    pub(super) sources: Vec<PackSource>,
}

fn default_policy_version() -> u8 {
    3
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PackSource {
    pub(super) source_id: String,
    kind: String,
    pub(super) path: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
struct SourceManifestRef {
    path: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
struct BaseManifest {
    cases: Vec<BaseCase>,
}

#[derive(Clone, Debug, Deserialize)]
struct BaseCase {
    case_id: String,
    source_pad_id: String,
    target_pad_id: String,
    scenario: ScenarioSpec,
}

#[derive(Clone, Debug, Deserialize)]
struct PracticalPlan {
    schema_id: String,
    status: String,
    source_manifests: BTreeMap<String, SourceManifestRef>,
    acceptance: Value,
    cases: Vec<PracticalRecipe>,
}

#[derive(Clone, Debug, Deserialize)]
struct PracticalRecipe {
    id: String,
    group: String,
    family: String,
    base: String,
    #[serde(default)]
    features: Vec<FeatureRecipe>,
    #[serde(default)]
    mutation: Option<PracticalMutation>,
    #[serde(default)]
    expected_preflight: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct PracticalMutation {
    target_width_m: Option<f64>,
    gravity_mps2: Option<f64>,
    dry_mass_delta_kg: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct FreshInputs {
    schema_id: String,
    schema_version: u32,
    status: String,
    source_manifests: BTreeMap<String, SourceManifestRef>,
    expanded_scenarios_sha256: String,
    cases: Vec<FreshRecipe>,
}

#[derive(Clone, Debug, Deserialize)]
struct FreshRecipe {
    id: String,
    group: String,
    family: String,
    base_case_id: String,
    span_m: f64,
    target_delta_m: f64,
    #[serde(default)]
    features: Vec<FeatureRecipe>,
    recipe_sha256: String,
    scenario_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
struct FeatureRecipe {
    fractions: Vec<f64>,
    height_m: f64,
}

pub(super) struct ExpandedPack {
    pub(super) definition: PackDefinition,
    pub(super) pack_bytes: Vec<u8>,
    pub(super) inputs: Vec<WaypointV2PackInput>,
    pub(super) input_identity: WaypointV2PackInputIdentity,
}

pub(super) fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval is a workspace crate")
        .to_path_buf()
}

pub(super) fn resolve_repo_file(repo: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    ensure!(
        !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "unsafe repository-relative input path {relative}"
    );
    let root = repo.canonicalize().context("resolve repository root")?;
    let resolved = root
        .join(path)
        .canonicalize()
        .with_context(|| format!("missing repository input {relative}"))?;
    ensure!(
        resolved.starts_with(&root) && resolved.is_file(),
        "repository input escapes root or is not a file: {relative}"
    );
    Ok(resolved)
}

pub(super) fn resolve_pack_file(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository_root().join(path)
    }
}

pub fn is_waypoint_v2_pack(path: &Path) -> Result<bool> {
    let bytes = fs::read(resolve_pack_file(path))
        .with_context(|| format!("read pack descriptor {}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse pack descriptor {}", path.display()))?;
    Ok(value.get("schema_id").and_then(Value::as_str) == Some("planner_v2_eval_pack_v1"))
}

pub(super) fn policy_for_version(version: u8) -> Result<WaypointV2Policy> {
    match version {
        1 => Ok(WaypointV2Policy::revision_1()),
        2 => Ok(WaypointV2Policy::revision_2()),
        3 => Ok(WaypointV2Policy::revision_3()),
        _ => bail!("policy_version must be 1, 2, or 3"),
    }
}

fn terrain_height(points: &[Vec2], x: f64) -> Result<f64> {
    ensure!(
        points.len() >= 2 && x >= points[0].x && x <= points[points.len() - 1].x,
        "terrain interpolation must remain in-domain"
    );
    for index in 1..points.len() {
        if x <= points[index].x {
            let a = points[index - 1];
            let b = points[index];
            // Keep the input compiler's IEEE-754 operation order exactly.
            return Ok(a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x));
        }
    }
    bail!("missing terrain interval")
}

fn append_features(
    scenario: &mut ScenarioSpec,
    source_center_x_m: f64,
    source_width_m: f64,
    target_center_x_m: f64,
    target_width_m: f64,
    fractions_span_m: f64,
    features: &[FeatureRecipe],
) -> Result<()> {
    let points = scenario.world.terrain.points();
    let baseline = points.to_vec();
    let source_surface_y_m = pad_surface(scenario, source_center_x_m)?;
    let target_surface_y_m = pad_surface(scenario, target_center_x_m)?;
    let mut additions = Vec::new();
    let mut last_end = 0.0;
    for feature in features {
        ensure!(
            matches!(feature.fractions.len(), 3 | 4),
            "feature needs three or four ordered vertices"
        );
        ensure!(
            feature.fractions.iter().enumerate().all(|(index, value)| {
                value.is_finite()
                    && *value > 0.0
                    && *value < 1.0
                    && (index == 0 || *value > feature.fractions[index - 1])
            }),
            "feature fractions must be finite, inside (0,1), and strictly ordered"
        );
        ensure!(
            feature.fractions[0] > last_end,
            "features overlap or are out of order"
        );
        ensure!(
            feature.height_m.is_finite() && feature.height_m > 0.0,
            "feature height must be positive and finite"
        );
        last_end = *feature.fractions.last().expect("feature has vertices");
        let first_x = source_center_x_m + feature.fractions[0] * fractions_span_m;
        let last_x = source_center_x_m + last_end * fractions_span_m;
        ensure!(
            first_x > source_center_x_m + source_width_m / 2.0
                && last_x < target_center_x_m - target_width_m / 2.0,
            "feature touches a landing-pad shelf"
        );
        ensure!(
            baseline
                .iter()
                .all(|point| point.x < first_x || point.x > last_x),
            "base breakpoint lies inside inserted feature"
        );
        for (index, fraction) in feature.fractions.iter().enumerate() {
            let x = source_center_x_m + *fraction * fractions_span_m;
            let offset = if index == 0 || index + 1 == feature.fractions.len() {
                0.0
            } else {
                feature.height_m
            };
            additions.push(Vec2::new(x, terrain_height(&baseline, x)? + offset));
        }
    }
    let TerrainPoints(points) = TerrainPoints::from_scenario_mut(scenario);
    points.extend(additions);
    points.sort_by(|a, b| a.x.total_cmp(&b.x));
    ensure!(
        points.iter().enumerate().all(|(index, point)| {
            point.x.is_finite()
                && point.y.is_finite()
                && (index == 0 || point.x > points[index - 1].x)
        }),
        "expanded terrain has duplicate, nonfinite, or unordered vertices"
    );
    for point in points.iter() {
        ensure!(
            point.y >= terrain_height(&baseline, point.x)? - 1e-10,
            "expanded terrain was lowered below tracked base"
        );
    }
    for (center_x, surface_y, width_m) in [
        (source_center_x_m, source_surface_y_m, source_width_m),
        (target_center_x_m, target_surface_y_m, target_width_m),
    ] {
        ensure!(
            terrain_height(points, center_x - width_m / 2.0)? == surface_y
                && terrain_height(points, center_x + width_m / 2.0)? == surface_y,
            "feature changed an exact landing-pad shelf edge"
        );
    }
    Ok(())
}

// Keeps the mutable heightfield access in one place without exposing terrain
// representation details to recipe expansion.
struct TerrainPoints<'a>(&'a mut Vec<Vec2>);

impl<'a> TerrainPoints<'a> {
    fn from_scenario_mut(scenario: &'a mut ScenarioSpec) -> Self {
        match &mut scenario.world.terrain {
            pd_core::TerrainDefinition::Heightfield { points_m } => Self(points_m),
        }
    }
}

impl std::ops::Deref for TerrainPoints<'_> {
    type Target = Vec<Vec2>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl std::ops::DerefMut for TerrainPoints<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

fn pad_surface(scenario: &ScenarioSpec, center_x_m: f64) -> Result<f64> {
    scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.center_x_m == center_x_m)
        .map(|pad| pad.surface_y_m)
        .context("landing-pad center not found while checking feature geometry")
}

fn load_bases(
    repo: &Path,
    manifests: &BTreeMap<String, SourceManifestRef>,
    source_manifest_hashes: &mut BTreeMap<String, String>,
) -> Result<BTreeMap<String, BaseCase>> {
    let mut bases = BTreeMap::new();
    let mut manifest_paths = BTreeSet::new();
    for (name, manifest) in manifests {
        ensure!(
            manifest_paths.insert(manifest.path.clone()),
            "duplicate source-manifest path {}",
            manifest.path
        );
        let path = resolve_repo_file(repo, &manifest.path)?;
        let bytes = fs::read(&path).with_context(|| format!("read {}", manifest.path))?;
        let digest = sha256_bytes(&bytes)?;
        ensure!(
            digest == manifest.sha256,
            "source manifest seal changed: {}",
            manifest.path
        );
        source_manifest_hashes.insert(manifest.path.clone(), digest);
        let source: BaseManifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse source manifest {}", manifest.path))?;
        for entry in source.cases {
            ensure!(
                !entry.case_id.is_empty()
                    && !entry.source_pad_id.is_empty()
                    && !entry.target_pad_id.is_empty(),
                "source manifest has an empty case or pad ID"
            );
            entry
                .scenario
                .validate()
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("invalid base scenario {}", entry.case_id))?;
            ensure!(
                bases.insert(entry.case_id.clone(), entry).is_none(),
                "duplicate base scenario across source manifests in {name}"
            );
        }
    }
    Ok(bases)
}

fn verify_route_free_landing(input: &WaypointV2PackInput) -> Result<()> {
    ensure!(
        input.scenario.mission.transfer_route.is_none(),
        "{}: V2 pack requires route-free admission",
        input.case_id
    );
    ensure!(
        matches!(
            &input.scenario.mission.goal,
            EvaluationGoal::LandingOnPad { target_pad_id }
                if target_pad_id == &input.target_pad_id
        ),
        "{}: target ID must match route-free landing goal",
        input.case_id
    );
    ensure!(
        input
            .scenario
            .world
            .landing_pads
            .iter()
            .any(|pad| pad.id == input.source_pad_id)
            && input
                .scenario
                .world
                .landing_pads
                .iter()
                .any(|pad| pad.id == input.target_pad_id),
        "{}: source or target pad ID does not resolve",
        input.case_id
    );
    input
        .scenario
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("invalid expanded scenario {}", input.case_id))
}

fn expand_practical(
    recipes: &[PracticalRecipe],
    bases: &BTreeMap<String, BaseCase>,
) -> Result<Vec<WaypointV2PackInput>> {
    let mut ids = BTreeSet::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut families = BTreeMap::<String, usize>::new();
    let mut inputs = Vec::with_capacity(recipes.len());
    for recipe in recipes {
        ensure!(
            ids.insert(recipe.id.clone()),
            "duplicate practical case {}",
            recipe.id
        );
        *counts.entry(recipe.group.clone()).or_default() += 1;
        if recipe.group == "ordinary" {
            *families.entry(recipe.family.clone()).or_default() += 1;
        }
        let base = bases
            .get(&recipe.base)
            .with_context(|| format!("{}: missing tracked base {}", recipe.id, recipe.base))?;
        let mut scenario = base.scenario.clone();
        scenario.id = recipe.id.clone();
        scenario.name = recipe.id.clone();
        let source = scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == base.source_pad_id)
            .context("practical source pad missing")?;
        let target = scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == base.target_pad_id)
            .context("practical target pad missing")?;
        let source_center = source.center_x_m;
        let source_width = source.width_m;
        let target_center = target.center_x_m;
        let target_width = target.width_m;
        let span = target_center - source_center;
        append_features(
            &mut scenario,
            source_center,
            source_width,
            target_center,
            target_width,
            span,
            &recipe.features,
        )?;
        if let Some(mutation) = &recipe.mutation {
            ensure!(
                recipe.group == "diagnostic",
                "{}: only diagnostics may mutate inputs",
                recipe.id
            );
            let target = scenario
                .world
                .landing_pads
                .iter_mut()
                .find(|pad| pad.id == base.target_pad_id)
                .context("diagnostic target pad missing")?;
            if let Some(width) = mutation.target_width_m {
                target.width_m = width;
            }
            if let Some(gravity) = mutation.gravity_mps2 {
                scenario.world.gravity_mps2 = gravity;
            }
            if let Some(delta) = mutation.dry_mass_delta_kg {
                scenario.vehicle.dry_mass_kg += delta;
            }
        }
        let group = match recipe.group.as_str() {
            "clear" => WaypointV2PackGroup::Clear,
            "ordinary" => WaypointV2PackGroup::Ordinary,
            "diagnostic" => WaypointV2PackGroup::Diagnostic,
            other => bail!("{}: unknown practical group {other}", recipe.id),
        };
        let input = WaypointV2PackInput {
            case_id: recipe.id.clone(),
            source_set: "practical_suite".into(),
            source_group: recipe.group.clone(),
            group,
            family: recipe.family.clone(),
            base_case_id: recipe.base.clone(),
            source_pad_id: base.source_pad_id.clone(),
            target_pad_id: base.target_pad_id.clone(),
            expected_preflight: recipe.expected_preflight.clone(),
            scenario,
        };
        verify_route_free_landing(&input)?;
        inputs.push(input);
    }
    ensure!(
        recipes.len() == 32,
        "practical fixture must contain 32 cases"
    );
    ensure!(
        counts
            == BTreeMap::from([
                ("clear".to_owned(), 8),
                ("diagnostic".to_owned(), 8),
                ("ordinary".to_owned(), 16),
            ]),
        "practical group counts changed"
    );
    ensure!(
        families
            == BTreeMap::from([
                ("plateau".to_owned(), 4),
                ("ridge".to_owned(), 4),
                ("sloped".to_owned(), 4),
                ("successive".to_owned(), 4),
            ]),
        "practical ordinary family counts changed"
    );
    Ok(inputs)
}

fn transform_805(scenario: &mut ScenarioSpec, recipe: &FreshRecipe, base: &BaseCase) -> Result<()> {
    let source = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == base.source_pad_id)
        .context("fresh source pad missing")?;
    let target = scenario
        .world
        .landing_pads
        .iter()
        .find(|pad| pad.id == base.target_pad_id)
        .context("fresh target pad missing")?;
    ensure!(
        source.center_x_m == -845.0
            && target.center_x_m == 0.0
            && source.width_m == 36.0
            && target.width_m == 36.0,
        "{}: 805 m transform base geometry changed",
        recipe.id
    );
    let old_target_y = target.surface_y_m;
    let requested_target_y = recipe.target_delta_m;
    let scale = if old_target_y == 0.0 {
        ensure!(
            requested_target_y == 0.0,
            "{}: flat source cannot create a slope",
            recipe.id
        );
        1.0
    } else {
        requested_target_y / old_target_y
    };
    let old_source_center = source.center_x_m;
    let source_half_width = source.width_m / 2.0;
    let pd_core::TerrainDefinition::Heightfield { points_m } = &mut scenario.world.terrain;
    for point in points_m {
        if point.x <= old_source_center + source_half_width {
            point.x += 40.0;
        } else {
            point.y *= scale;
        }
    }
    scenario
        .world
        .landing_pads
        .iter_mut()
        .find(|pad| pad.id == base.source_pad_id)
        .context("fresh source pad missing after transform")?
        .center_x_m = -805.0;
    scenario
        .world
        .landing_pads
        .iter_mut()
        .find(|pad| pad.id == base.target_pad_id)
        .context("fresh target pad missing after transform")?
        .surface_y_m = requested_target_y;
    scenario.initial_state.position_m.x = -805.0;
    Ok(())
}

fn expand_fresh(
    fixture: &FreshInputs,
    bases: &BTreeMap<String, BaseCase>,
) -> Result<Vec<WaypointV2PackInput>> {
    ensure!(fixture.schema_id == "waypoint_v2_fresh_terrain_inputs_v1");
    ensure!(fixture.schema_version == 1);
    ensure!(fixture.status == "frozen_before_mission_attempts");
    ensure!(
        fixture.cases.len() == 12,
        "fresh fixture must contain twelve cases"
    );
    let expected_order = [
        "fresh_clear_flat_805",
        "fresh_clear_uphill_805",
        "fresh_clear_downhill_805",
        "fresh_ridge_early_900",
        "fresh_ridge_middle_900",
        "fresh_ridge_late_900",
        "fresh_plateau_early_900",
        "fresh_plateau_broad_900",
        "fresh_plateau_late_900",
        "fresh_compound_successive_900",
        "fresh_compound_uphill_ridge_805",
        "fresh_compound_downhill_plateau_805",
    ];
    ensure!(
        fixture
            .cases
            .iter()
            .map(|case| case.id.as_str())
            .eq(expected_order),
        "fresh fixture case order changed"
    );
    let mut ids = BTreeSet::new();
    let mut inputs = Vec::with_capacity(fixture.cases.len());
    let mut families = BTreeMap::<String, usize>::new();
    for recipe in &fixture.cases {
        ensure!(
            ids.insert(recipe.id.clone()),
            "duplicate fresh case {}",
            recipe.id
        );
        ensure!(
            matches!(recipe.group.as_str(), "clear" | "terrain"),
            "{}: unsupported fresh group {}",
            recipe.id,
            recipe.group
        );
        if recipe.group == "clear" {
            ensure!(
                recipe.features.is_empty(),
                "{}: clear case has features",
                recipe.id
            );
        } else {
            *families.entry(recipe.family.clone()).or_default() += 1;
        }
        ensure!(
            matches!(recipe.span_m, 805.0 | 900.0),
            "{}: unsupported fresh span",
            recipe.id
        );
        let base = bases.get(&recipe.base_case_id).with_context(|| {
            format!(
                "{}: missing tracked base {}",
                recipe.id, recipe.base_case_id
            )
        })?;
        let mut scenario = base.scenario.clone();
        let old_source = scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == base.source_pad_id)
            .context("fresh source pad missing")?
            .clone();
        let old_target = scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == base.target_pad_id)
            .context("fresh target pad missing")?
            .clone();
        if recipe.span_m == 805.0 {
            ensure!(
                recipe.base_case_id.starts_with("operational_"),
                "{}: 805 m base provenance changed",
                recipe.id
            );
            transform_805(&mut scenario, recipe, base)?;
        } else {
            ensure!(
                old_source.center_x_m == -900.0 && old_target.center_x_m == 0.0,
                "{}: 900 m base geometry changed",
                recipe.id
            );
        }
        let source = scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == base.source_pad_id)
            .context("fresh transformed source pad missing")?;
        let target = scenario
            .world
            .landing_pads
            .iter()
            .find(|pad| pad.id == base.target_pad_id)
            .context("fresh transformed target pad missing")?;
        ensure!(
            source.width_m == 36.0
                && target.width_m == 36.0
                && source.center_x_m == -recipe.span_m
                && target.center_x_m == 0.0
                && target.surface_y_m == recipe.target_delta_m
                && scenario.initial_state.position_m.x == -recipe.span_m
                && scenario.initial_state.position_m.y == 5.0,
            "{}: span, shelf, target or initial-state recipe mismatch",
            recipe.id
        );
        ensure!(
            scenario.vehicle == base.scenario.vehicle
                && scenario.world.gravity_mps2 == base.scenario.world.gravity_mps2
                && scenario.sim == base.scenario.sim
                && scenario.mission == base.scenario.mission,
            "{}: fresh recipe changed non-geometric mission inputs",
            recipe.id
        );
        let source_center_x_m = source.center_x_m;
        let source_width_m = source.width_m;
        let target_center_x_m = target.center_x_m;
        let target_width_m = target.width_m;
        let baseline = scenario.world.terrain.points().to_vec();
        ensure!(
            baseline.iter().enumerate().all(|(index, point)| {
                point.x.is_finite()
                    && point.y.is_finite()
                    && (index == 0 || point.x > baseline[index - 1].x)
            }),
            "{}: invalid transformed base terrain",
            recipe.id
        );
        if recipe.span_m == 805.0 {
            ensure!(
                baseline.first().map(|point| point.x) == Some(-965.0)
                    && baseline.last().map(|point| point.x) == Some(160.0)
                    && baseline.get(1).map(|point| point.x) == Some(-823.0)
                    && baseline.get(2).map(|point| point.x) == Some(-787.0),
                "{}: 805 m outer terrain or source shelf changed",
                recipe.id
            );
        }
        append_features(
            &mut scenario,
            source_center_x_m,
            source_width_m,
            target_center_x_m,
            target_width_m,
            recipe.span_m,
            &recipe.features,
        )?;
        scenario.id = recipe.id.clone();
        scenario.name = recipe.id.clone();
        scenario.metadata.insert(
            "research".into(),
            "waypoint_v2_fresh_terrain_readiness_v1".into(),
        );
        scenario
            .metadata
            .insert("case_id".into(), recipe.id.clone());
        scenario
            .metadata
            .insert("terrain_kind".into(), recipe.family.clone());
        if !scenario
            .tags
            .iter()
            .any(|tag| tag == "waypoint_v2_fresh_terrain_readiness")
        {
            scenario
                .tags
                .push("waypoint_v2_fresh_terrain_readiness".into());
        }
        // ScenarioSpec serializes the optional route as explicit null. The JS
        // fresh fixture seals that same normalized route-free value.
        scenario.mission.transfer_route = None;
        let group = match recipe.group.as_str() {
            "clear" => WaypointV2PackGroup::Clear,
            "terrain" => WaypointV2PackGroup::AdditionalTerrain,
            other => bail!("{}: unsupported fresh group {other}", recipe.id),
        };
        let input = WaypointV2PackInput {
            case_id: recipe.id.clone(),
            source_set: "fresh_terrain_inputs".into(),
            source_group: recipe.group.clone(),
            group,
            family: recipe.family.clone(),
            base_case_id: recipe.base_case_id.clone(),
            source_pad_id: base.source_pad_id.clone(),
            target_pad_id: base.target_pad_id.clone(),
            expected_preflight: None,
            scenario,
        };
        verify_route_free_landing(&input)?;
        inputs.push(input);
    }
    ensure!(
        families
            == BTreeMap::from([
                ("compound".to_owned(), 3),
                ("plateau".to_owned(), 3),
                ("ridge".to_owned(), 3),
            ]),
        "fresh terrain family counts changed"
    );
    // The JS-produced seals remain source data; this pack records a separately
    // named Rust-native digest over the typed expansion.
    ensure!(
        fixture.expanded_scenarios_sha256 == FRESH_EXPANDED_JS_SHA256,
        "fresh fixture JavaScript seal changed"
    );
    ensure!(
        fixture
            .cases
            .iter()
            .all(|case| is_sha256(&case.recipe_sha256) && is_sha256(&case.scenario_sha256)),
        "fresh fixture contains a malformed JavaScript seal"
    );
    Ok(inputs)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn load_and_expand(pack_path: &Path) -> Result<ExpandedPack> {
    let repo = repository_root();
    let resolved_pack = resolve_pack_file(pack_path);
    let pack_bytes = fs::read(&resolved_pack)
        .with_context(|| format!("read V2 pack {}", resolved_pack.display()))?;
    let definition: PackDefinition = serde_json::from_slice(&pack_bytes)
        .with_context(|| format!("parse V2 pack {}", resolved_pack.display()))?;
    ensure!(
        definition.schema_id == "planner_v2_eval_pack_v1"
            && definition.id == "planner_v2_lab_suite"
            && !definition.name.trim().is_empty()
            && !definition.description.trim().is_empty(),
        "unsupported V2 pack definition"
    );
    policy_for_version(definition.policy_version)?;
    ensure!(
        definition.sources.len() == 2
            && definition.sources[0].source_id == "practical_suite"
            && definition.sources[0].kind == "waypoint_v2_practical_suite_plan_v1"
            && definition.sources[0].path == PRACTICAL_PATH
            && definition.sources[0].sha256 == PRACTICAL_SHA256
            && definition.sources[1].source_id == "fresh_terrain_inputs"
            && definition.sources[1].kind == "waypoint_v2_fresh_terrain_inputs_v1"
            && definition.sources[1].path == FRESH_PATH
            && definition.sources[1].sha256 == FRESH_SHA256,
        "V2 pack source order or seals changed"
    );
    let pack_sha = sha256_bytes(&pack_bytes)?;
    let mut source_fixture_hashes = BTreeMap::new();
    let mut source_manifest_hashes = BTreeMap::new();
    let mut cases = Vec::with_capacity(EXPECTED_CASE_COUNT);
    let mut saw_practical = false;
    let mut saw_fresh = false;
    for source in &definition.sources {
        let path = resolve_repo_file(&repo, &source.path)?;
        let bytes = fs::read(&path).with_context(|| format!("read pack source {}", source.path))?;
        let digest = sha256_bytes(&bytes)?;
        ensure!(
            digest == source.sha256,
            "pack source fixture seal changed: {}",
            source.path
        );
        source_fixture_hashes.insert(source.source_id.clone(), digest);
        match source.source_id.as_str() {
            "practical_suite" => {
                let plan: PracticalPlan = serde_json::from_slice(&bytes)
                    .with_context(|| format!("parse {}", source.path))?;
                ensure!(
                    plan.schema_id == "waypoint_v2_practical_suite_plan_v1"
                        && plan.status == "design_only_not_flight_accepted",
                    "unsupported practical plan schema or status"
                );
                ensure!(
                    plan.acceptance["clear_count"] == 8
                        && plan.acceptance["ordinary_count"] == 16
                        && plan.acceptance["diagnostic_count"] == 8,
                    "practical fixture acceptance counts changed"
                );
                let bases = load_bases(&repo, &plan.source_manifests, &mut source_manifest_hashes)?;
                ensure!(!saw_practical, "duplicate practical source");
                cases.extend(expand_practical(&plan.cases, &bases)?);
                saw_practical = true;
            }
            "fresh_terrain_inputs" => {
                let fixture: FreshInputs = serde_json::from_slice(&bytes)
                    .with_context(|| format!("parse {}", source.path))?;
                let bases = load_bases(
                    &repo,
                    &fixture.source_manifests,
                    &mut source_manifest_hashes,
                )?;
                ensure!(!saw_fresh, "duplicate fresh source");
                cases.extend(expand_fresh(&fixture, &bases)?);
                saw_fresh = true;
            }
            other => bail!("unsupported V2 pack source {other}"),
        }
    }
    ensure!(saw_practical, "missing practical suite source");
    ensure!(saw_fresh, "missing fresh terrain source");
    ensure!(
        cases.len() == EXPECTED_CASE_COUNT,
        "V2 pack must expand to 44 cases"
    );
    let mut ids = BTreeSet::new();
    ensure!(
        cases.iter().all(|case| ids.insert(case.case_id.as_str())),
        "duplicate case ID across V2 sources"
    );
    let digest_items = cases
        .iter()
        .map(|case| ExpandedDigestItem {
            case_id: &case.case_id,
            source_set: &case.source_set,
            source_group: &case.source_group,
            group: &case.group,
            family: &case.family,
            base_case_id: &case.base_case_id,
            source_pad_id: &case.source_pad_id,
            target_pad_id: &case.target_pad_id,
            expected_preflight: &case.expected_preflight,
            scenario: &case.scenario,
        })
        .collect::<Vec<_>>();
    let rust_expanded_digest = sha256_bytes(&serde_json::to_vec(&digest_items)?)?;
    let input_identity = WaypointV2PackInputIdentity {
        pack_file_sha256: pack_sha,
        source_fixture_sha256: source_fixture_hashes,
        source_manifest_sha256: source_manifest_hashes,
        rust_typed_expanded_inputs_sha256: rust_expanded_digest,
        rust_typed_expanded_input_count: cases.len(),
        digest_scheme: "sha256(serde_json::to_vec(ordered ExpandedDigestItem[]))".into(),
    };
    Ok(ExpandedPack {
        definition,
        pack_bytes,
        inputs: cases,
        input_identity,
    })
}

/// Expand the tracked V2 pack as input data only and return one named case.
/// This fixture seam does not execute a planner or create output artifacts.
pub fn load_waypoint_v2_pack_case_input(
    pack_path: &Path,
    case_id: &str,
) -> Result<WaypointV2PackInput> {
    ensure!(
        !case_id.trim().is_empty(),
        "V2 pack case ID must be nonempty"
    );
    load_and_expand(pack_path)?
        .inputs
        .into_iter()
        .find(|input| input.case_id == case_id)
        .with_context(|| format!("V2 pack has no case named {case_id}"))
}

/// Frozen typed input identity for the tracked default pack. Acceptance uses
/// this read-only expansion to bind saved cases to the registered recipe and
/// fixtures; it deliberately does not compare the capture's Git HEAD to the
/// current checkout.
pub(crate) fn check_default_planner_v2_binding()
-> Result<(Vec<WaypointV2PackInput>, WaypointV2PackInputIdentity)> {
    let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH))?;
    Ok((expanded.inputs, expanded.input_identity))
}
