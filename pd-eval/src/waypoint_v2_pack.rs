//! Native input expansion, execution and report regeneration for the named
//! policy-3 V2 lab pack. This is evaluator-only; it does not change the
//! controller or the ordinary ScenarioPackSpec workflow.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail, ensure};
use pd_core::{EvaluationGoal, ScenarioSpec, Vec2};
use pd_plan::waypoint_v2::{WaypointV2Policy, WaypointV2Stop};
use pd_report::flight_annotations::{AnnotationNavigation, NavigationLink};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    WaypointDirectNominalDirectGenerationPolicyV1, WaypointDirectNominalDirectGenerationRequest,
    WaypointV2FlightResult,
    nominal_direct_flight::{reserve_output_root, write_create_only},
    preflight_waypoint_v2_flight,
    waypoint_direct_body_aware_terminal::sha256_bytes,
    waypoint_v2_output::write_waypoint_v2_flight,
    waypoint_v2_report::render_rich_flight,
};

pub const DEFAULT_PLANNER_PACK_PATH: &str = "fixtures/packs/planner_v2_lab_suite.json";
pub const WAYPOINT_V2_BATCH_SCHEMA_ID: &str = "planner_v2_eval_batch_v1";

mod tree_report;

const PRACTICAL_PATH: &str = "fixtures/research/waypoint_v2_practical_suite_plan_v1.json";
const PRACTICAL_SHA256: &str = "92869e10225a72e8716ad87c20fbc1ca3795bd692aa9e41d011cd9ae18cf438c";
const FRESH_PATH: &str = "fixtures/research/waypoint_v2_fresh_terrain_inputs_v1.json";
const FRESH_SHA256: &str = "554b406f62387e9598d89e53a26ed705b2fce5380b4e7a216f92f3003950cd9c";
const FRESH_EXPANDED_JS_SHA256: &str =
    "311752c8da362c2261ce069e633041407b6b59ff88c0c32381d21bd095128c52";
const EXPECTED_CASE_COUNT: usize = 44;
const MAX_WORKERS: usize = EXPECTED_CASE_COUNT;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PackDefinition {
    schema_id: String,
    id: String,
    name: String,
    description: String,
    #[serde(default = "default_policy_version")]
    policy_version: u8,
    sources: Vec<PackSource>,
}

fn default_policy_version() -> u8 {
    3
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PackSource {
    source_id: String,
    kind: String,
    path: String,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaypointV2PackGroup {
    Clear,
    Ordinary,
    AdditionalTerrain,
    Diagnostic,
}

impl WaypointV2PackGroup {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Ordinary => "ordinary",
            Self::AdditionalTerrain => "additional_terrain",
            Self::Diagnostic => "diagnostic",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2PackInput {
    pub case_id: String,
    pub source_set: String,
    pub source_group: String,
    pub group: WaypointV2PackGroup,
    pub family: String,
    pub base_case_id: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub expected_preflight: Option<String>,
    pub scenario: ScenarioSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointV2BatchReport {
    pub schema_id: String,
    pub pack_id: String,
    pub name: String,
    pub status: String,
    pub policy_version: u8,
    pub case_count: usize,
    pub cases: Vec<WaypointV2BatchCase>,
    pub summary: WaypointV2BatchSummary,
    pub input_identity: WaypointV2PackInputIdentity,
    pub pack_snapshot_sha256: String,
    pub expanded_inputs_snapshot_sha256: String,
    pub pack_snapshot_path: String,
    pub expanded_inputs_snapshot_path: String,
    pub provenance: WaypointV2BatchProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointV2BatchCase {
    pub case_id: String,
    pub source_set: String,
    pub source_group: String,
    pub group: WaypointV2PackGroup,
    pub family: String,
    pub base_case_id: String,
    pub source_pad_id: String,
    pub target_pad_id: String,
    pub expected_preflight: Option<String>,
    pub status: String,
    pub outcome: Option<String>,
    pub planning_stop: Option<String>,
    pub reason: Option<String>,
    pub correction_count: Option<u32>,
    pub initial_nominal_terrain_blocked: Option<bool>,
    pub integrity_passed: Option<bool>,
    pub final_source_replay_passed: Option<bool>,
    pub physical_outcome: Option<String>,
    pub mission_outcome: Option<String>,
    pub planning_s: Option<f64>,
    pub execution_s: Option<f64>,
    pub replay_s: Option<f64>,
    pub input_path: String,
    pub scenario_path: String,
    pub flight_path: String,
    pub summary_path: String,
    pub rich_report_path: Option<String>,
    pub annotated_report_path: String,
    pub artifact_sha256: BTreeMap<String, String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2BatchSummary {
    pub clear_count: usize,
    pub ordinary_count: usize,
    pub additional_terrain_count: usize,
    pub diagnostic_count: usize,
    pub direct_landing_count: usize,
    pub corrected_landing_count: usize,
    pub valid_landing_count: usize,
    pub non_landing_count: usize,
    pub diagnostic_landing_count: usize,
    pub diagnostic_non_landing_count: usize,
    pub simulation_unverified_count: usize,
    pub unsupported_count: usize,
    pub crash_count: usize,
    pub integrity_passed_count: usize,
    pub integrity_failed_count: usize,
    pub final_source_replay_passed_count: usize,
    pub final_source_replay_failed_count: usize,
    pub initially_blocked_count: usize,
    pub planning_stops: BTreeMap<String, usize>,
    pub physical_outcomes: BTreeMap<String, usize>,
    pub mission_outcomes: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2PackInputIdentity {
    pub pack_file_sha256: String,
    pub source_fixture_sha256: BTreeMap<String, String>,
    pub source_manifest_sha256: BTreeMap<String, String>,
    /// SHA-256 of `serde_json::to_vec` over the ordered typed V2PackInput array.
    /// This Rust-native digest is not the JavaScript runner's canonical JSON seal.
    pub rust_typed_expanded_inputs_sha256: String,
    pub rust_typed_expanded_input_count: usize,
    pub digest_scheme: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaypointV2BatchProvenance {
    pub source_before: WaypointV2SourceState,
    pub source_after: Option<WaypointV2SourceState>,
    pub unchanged_during_capture: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WaypointV2SourceState {
    pub git_commit: Option<String>,
    pub git_dirty: Option<bool>,
    pub rust_source_tree_sha256: String,
    pub executable_sha256: String,
}

struct ExpandedPack {
    definition: PackDefinition,
    pack_bytes: Vec<u8>,
    inputs: Vec<WaypointV2PackInput>,
    input_identity: WaypointV2PackInputIdentity,
}

#[derive(Serialize)]
struct ExpandedDigestItem<'a> {
    case_id: &'a str,
    source_set: &'a str,
    source_group: &'a str,
    group: &'a WaypointV2PackGroup,
    family: &'a str,
    base_case_id: &'a str,
    source_pad_id: &'a str,
    target_pad_id: &'a str,
    expected_preflight: &'a Option<String>,
    scenario: &'a ScenarioSpec,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pd-eval is a workspace crate")
        .to_path_buf()
}

fn resolve_repo_file(repo: &Path, relative: &str) -> Result<PathBuf> {
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

fn resolve_pack_file(path: &Path) -> PathBuf {
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

fn policy_for_version(version: u8) -> Result<WaypointV2Policy> {
    match version {
        1 => Ok(WaypointV2Policy::default()),
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

fn load_and_expand(pack_path: &Path) -> Result<ExpandedPack> {
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

/// Frozen typed input identity for the tracked default pack. Acceptance uses
/// this read-only expansion to bind saved cases to the registered recipe and
/// fixtures; it deliberately does not compare the capture's Git HEAD to the
/// current checkout.
pub(crate) fn check_default_planner_v2_binding()
-> Result<(Vec<WaypointV2PackInput>, WaypointV2PackInputIdentity)> {
    let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH))?;
    Ok((expanded.inputs, expanded.input_identity))
}

fn request_for(input: &WaypointV2PackInput) -> WaypointDirectNominalDirectGenerationRequest {
    WaypointDirectNominalDirectGenerationRequest {
        probe_id: input.scenario.id.clone(),
        scenario: input.scenario.clone(),
        source_pad_id: input.source_pad_id.clone(),
        target_pad_id: input.target_pad_id.clone(),
        policy: WaypointDirectNominalDirectGenerationPolicyV1::default(),
    }
}

fn enum_text<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_owned)
        .context("expected a string-serialized enum")
}

fn expected_preflight_matches(
    input: &WaypointV2PackInput,
    preflight_stop: Option<WaypointV2Stop>,
) -> Result<()> {
    match input.expected_preflight.as_deref() {
        Some("unsupported") => ensure!(
            preflight_stop == Some(WaypointV2Stop::Unsupported),
            "{}: expected unsupported preflight, got {:?}",
            input.case_id,
            preflight_stop
        ),
        Some("invalid_input") => ensure!(
            preflight_stop == Some(WaypointV2Stop::InvalidInput),
            "{}: expected invalid-input preflight, got {:?}",
            input.case_id,
            preflight_stop
        ),
        Some(other) => bail!("{}: unsupported expected_preflight {other}", input.case_id),
        None => ensure!(
            preflight_stop.is_none(),
            "{}: unexpected V2 preflight rejection {:?}",
            input.case_id,
            preflight_stop
        ),
    }
    Ok(())
}

fn source_path(input: &WaypointV2PackInput) -> &'static str {
    match input.source_set.as_str() {
        "practical_suite" => PRACTICAL_PATH,
        "fresh_terrain_inputs" => FRESH_PATH,
        _ => "",
    }
}

fn bounded_worker_count(requested: usize) -> Result<usize> {
    ensure!(requested > 0, "workers must be at least one");
    Ok(requested.min(MAX_WORKERS))
}

fn run_one_case(
    input: &WaypointV2PackInput,
    policy: &WaypointV2Policy,
    capture_root: &Path,
) -> Result<WaypointV2BatchCase> {
    let run_dir = capture_root.join("runs").join(&input.case_id);
    let request = request_for(input);
    let result = write_waypoint_v2_flight(&request, policy, &run_dir)
        .with_context(|| format!("run V2 case {}", input.case_id))?;
    let manifest_present = result.manifest.is_some();
    let flight_started = result.ordinary_flight.is_some();
    let status = if manifest_present {
        "simulated"
    } else if flight_started {
        "simulation_unverified"
    } else {
        "preflight_rejected"
    };
    let report_file = run_dir.join("report.html");
    let rich_report_path = if report_file.is_file() {
        Some(format!("runs/{}/report.html", input.case_id))
    } else {
        None
    };
    ensure!(
        manifest_present == rich_report_path.is_some(),
        "{}: raw report presence disagrees with replay manifest",
        input.case_id
    );
    let mut artifact_sha256 = BTreeMap::new();
    for name in ["scenario.json", "flight.json", "summary.json"] {
        let relative = format!("runs/{}/{name}", input.case_id);
        artifact_sha256.insert(relative, sha256_bytes(&fs::read(run_dir.join(name))?)?);
    }
    if rich_report_path.is_some() {
        let relative = format!("runs/{}/report.html", input.case_id);
        artifact_sha256.insert(relative, sha256_bytes(&fs::read(&report_file)?)?);
    }
    let planning_stop = enum_text(&result.planning_stop)?;
    let physical_outcome = result
        .physical_outcome
        .as_ref()
        .map(enum_text)
        .transpose()?;
    let mission_outcome = result.mission_outcome.as_ref().map(enum_text).transpose()?;
    let outcome = physical_outcome
        .clone()
        .or_else(|| Some(planning_stop.clone()));
    Ok(WaypointV2BatchCase {
        case_id: input.case_id.clone(),
        source_set: input.source_set.clone(),
        source_group: input.source_group.clone(),
        group: input.group.clone(),
        family: input.family.clone(),
        base_case_id: input.base_case_id.clone(),
        source_pad_id: input.source_pad_id.clone(),
        target_pad_id: input.target_pad_id.clone(),
        expected_preflight: input.expected_preflight.clone(),
        status: status.into(),
        outcome,
        planning_stop: Some(planning_stop),
        reason: result.reason.clone(),
        correction_count: Some(result.correction_count),
        initial_nominal_terrain_blocked: Some(result.initial_nominal_terrain_blocked),
        integrity_passed: Some(result.integrity_passed),
        final_source_replay_passed: Some(result.final_source_replay_passed),
        physical_outcome,
        mission_outcome,
        planning_s: Some(result.timings.planning_s),
        execution_s: Some(result.timings.execution_s),
        replay_s: Some(result.timings.replay_s),
        input_path: source_path(input).into(),
        scenario_path: format!("runs/{}/scenario.json", input.case_id),
        flight_path: format!("runs/{}/flight.json", input.case_id),
        summary_path: format!("runs/{}/summary.json", input.case_id),
        rich_report_path,
        annotated_report_path: format!("runs/{}/index.html", input.case_id),
        artifact_sha256,
        error: result
            .planning_stop
            .eq(&WaypointV2Stop::ImplementationError)
            .then(|| result.reason.clone())
            .flatten(),
    })
}

pub(crate) fn summarize(cases: &[WaypointV2BatchCase]) -> Result<WaypointV2BatchSummary> {
    let mut summary = WaypointV2BatchSummary::default();
    for case in cases {
        match case.group {
            WaypointV2PackGroup::Clear => summary.clear_count += 1,
            WaypointV2PackGroup::Ordinary => summary.ordinary_count += 1,
            WaypointV2PackGroup::AdditionalTerrain => summary.additional_terrain_count += 1,
            WaypointV2PackGroup::Diagnostic => summary.diagnostic_count += 1,
        }
        if let Some(stop) = &case.planning_stop {
            *summary.planning_stops.entry(stop.clone()).or_default() += 1;
            if stop == "unsupported" {
                summary.unsupported_count += 1;
            }
        }
        if case.initial_nominal_terrain_blocked == Some(true) {
            summary.initially_blocked_count += 1;
        }
        if case.integrity_passed == Some(true) {
            summary.integrity_passed_count += 1;
        } else if case.integrity_passed == Some(false) {
            summary.integrity_failed_count += 1;
        }
        if case.physical_outcome == Some("crashed".into()) {
            summary.crash_count += 1;
        }
        if let Some(outcome) = &case.physical_outcome {
            *summary
                .physical_outcomes
                .entry(outcome.clone())
                .or_default() += 1;
        }
        if let Some(outcome) = &case.mission_outcome {
            *summary.mission_outcomes.entry(outcome.clone()).or_default() += 1;
        }
        let simulated = matches!(case.status.as_str(), "simulated" | "simulation_unverified");
        if case.status == "simulation_unverified" {
            summary.simulation_unverified_count += 1;
        }
        if simulated && case.final_source_replay_passed == Some(true) {
            summary.final_source_replay_passed_count += 1;
        } else if simulated && case.final_source_replay_passed == Some(false) {
            summary.final_source_replay_failed_count += 1;
        }
        let valid_landing = case.physical_outcome.as_deref() == Some("landed_on_target")
            && case.mission_outcome.as_deref() == Some("success")
            && case.planning_stop.as_deref() == Some("landed")
            && case.integrity_passed == Some(true)
            && case.final_source_replay_passed == Some(true);
        let diagnostic = case.group == WaypointV2PackGroup::Diagnostic;
        if valid_landing && diagnostic {
            summary.diagnostic_landing_count += 1;
        } else if valid_landing {
            summary.valid_landing_count += 1;
            if case.correction_count == Some(0) {
                summary.direct_landing_count += 1;
            } else {
                summary.corrected_landing_count += 1;
            }
        } else if simulated && diagnostic {
            summary.diagnostic_non_landing_count += 1;
        } else if simulated {
            summary.non_landing_count += 1;
        }
    }
    ensure!(
        summary.clear_count == 11
            && summary.ordinary_count == 16
            && summary.additional_terrain_count == 9
            && summary.diagnostic_count == 8,
        "native V2 pack group denominator changed"
    );
    ensure!(
        summary.clear_count + summary.ordinary_count + summary.additional_terrain_count == 36,
        "native V2 landing denominator is not 36"
    );
    Ok(summary)
}

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    (output.status.success()).then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn append_rust_files(dir: &Path, repo: &Path, bytes: &mut Vec<u8>) -> Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "symlink in Rust source tree: {}",
            path.display()
        );
        if metadata.is_dir() {
            append_rust_files(&path, repo, bytes)?;
        } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path.strip_prefix(repo)?.to_string_lossy();
            bytes.extend_from_slice(relative.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&fs::read(path)?);
            bytes.push(0xff);
        }
    }
    Ok(())
}

pub(crate) fn capture_source_state(repo: &Path) -> Result<WaypointV2SourceState> {
    let mut tree_bytes = Vec::new();
    for crate_name in ["pd-eval", "pd-core", "pd-plan", "pd-report", "pd-control"] {
        let src = repo.join(crate_name).join("src");
        if src.is_dir() {
            append_rust_files(&src, repo, &mut tree_bytes)?;
        }
    }
    for relative in [
        "Cargo.toml",
        "Cargo.lock",
        "pd-eval/Cargo.toml",
        "pd-core/Cargo.toml",
        "pd-plan/Cargo.toml",
        "pd-report/Cargo.toml",
        "pd-control/Cargo.toml",
    ] {
        let path = repo.join(relative);
        ensure!(path.is_file(), "missing source identity input {relative}");
        tree_bytes.extend_from_slice(relative.as_bytes());
        tree_bytes.push(0);
        tree_bytes.extend_from_slice(&fs::read(path)?);
        tree_bytes.push(0xff);
    }
    let executable = std::env::current_exe().context("resolve evaluator executable")?;
    let executable_sha256 = sha256_bytes(&fs::read(executable)?)?;
    let dirty = git_output(repo, &["status", "--porcelain"]);
    Ok(WaypointV2SourceState {
        git_commit: git_output(repo, &["rev-parse", "HEAD"]),
        git_dirty: dirty.as_ref().map(|status| !status.is_empty()),
        rust_source_tree_sha256: sha256_bytes(&tree_bytes)?,
        executable_sha256,
    })
}

fn input_identity_still_matches(
    repo: &Path,
    pack_path: &Path,
    expanded: &ExpandedPack,
) -> Result<bool> {
    let pack = match fs::read(resolve_pack_file(pack_path)) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };
    if sha256_bytes(&pack)? != expanded.input_identity.pack_file_sha256 {
        return Ok(false);
    }
    for source in &expanded.definition.sources {
        let Some(expected) = expanded
            .input_identity
            .source_fixture_sha256
            .get(&source.source_id)
        else {
            return Ok(false);
        };
        let path = match resolve_repo_file(repo, &source.path) {
            Ok(path) => path,
            Err(_) => return Ok(false),
        };
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false),
        };
        if sha256_bytes(&bytes)? != *expected {
            return Ok(false);
        }
    }
    for (path, expected) in &expanded.input_identity.source_manifest_sha256 {
        let resolved = match resolve_repo_file(repo, path) {
            Ok(path) => path,
            Err(_) => return Ok(false),
        };
        let bytes = match fs::read(resolved) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false),
        };
        if sha256_bytes(&bytes)? != *expected {
            return Ok(false);
        }
    }
    Ok(true)
}

fn write_bytes_create_only(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create-only artifact {}", path.display()))?;
    file.write_all(bytes)?;
    Ok(())
}

fn digest_expanded_inputs(inputs: &[WaypointV2PackInput]) -> Result<String> {
    let digest_items = inputs
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
    sha256_bytes(&serde_json::to_vec(&digest_items)?)
}

pub fn run_waypoint_v2_pack(
    pack_path: &Path,
    capture_root: &Path,
    workers: usize,
) -> Result<WaypointV2BatchReport> {
    let effective_workers = bounded_worker_count(workers)?;
    let expanded = load_and_expand(pack_path)?;
    let policy = policy_for_version(expanded.definition.policy_version)?;
    // Complete admission checks before reserving any output or beginning a flight.
    for input in &expanded.inputs {
        let preflight = preflight_waypoint_v2_flight(&request_for(input), &policy);
        expected_preflight_matches(input, preflight.rejection)?;
    }
    ensure!(
        !capture_root.exists() && fs::symlink_metadata(capture_root).is_err(),
        "capture root already exists: {}",
        capture_root.display()
    );
    let source_before = capture_source_state(&repository_root())?;
    reserve_output_root(capture_root)?;
    let runs = capture_root.join("runs");
    fs::create_dir(&runs)?;
    write_bytes_create_only(&capture_root.join("pack.json"), &expanded.pack_bytes)?;
    let expanded_inputs_bytes = serde_json::to_vec_pretty(&expanded.inputs)?;
    write_bytes_create_only(
        &capture_root.join("expanded-inputs.json"),
        &expanded_inputs_bytes,
    )?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(effective_workers)
        .build()
        .context("build bounded V2 evaluator worker pool")?;
    let outcomes = pool.install(|| {
        expanded
            .inputs
            .par_iter()
            .map(|input| run_one_case(input, &policy, capture_root))
            .collect::<Vec<_>>()
    });
    let mut cases = Vec::with_capacity(outcomes.len());
    let mut first_error = None;
    for outcome in outcomes {
        match outcome {
            Ok(case) => cases.push(case),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }
    if let Some(error) = first_error {
        return Err(error.context(format!(
            "V2 batch incomplete; partial capture preserved at {}",
            capture_root.display()
        )));
    }
    ensure!(
        cases.len() == EXPECTED_CASE_COUNT,
        "V2 capture did not record all cases"
    );
    let source_after = capture_source_state(&repository_root())?;
    let inputs_unchanged = input_identity_still_matches(&repository_root(), pack_path, &expanded)?;
    let summary = summarize(&cases)?;
    let report = WaypointV2BatchReport {
        schema_id: WAYPOINT_V2_BATCH_SCHEMA_ID.into(),
        pack_id: expanded.definition.id.clone(),
        name: expanded.definition.name.clone(),
        status: "completed".into(),
        policy_version: expanded.definition.policy_version,
        case_count: cases.len(),
        cases,
        summary,
        input_identity: expanded.input_identity,
        pack_snapshot_sha256: sha256_bytes(&expanded.pack_bytes)?,
        expanded_inputs_snapshot_sha256: sha256_bytes(&expanded_inputs_bytes)?,
        pack_snapshot_path: "pack.json".into(),
        expanded_inputs_snapshot_path: "expanded-inputs.json".into(),
        provenance: WaypointV2BatchProvenance {
            unchanged_during_capture: Some(source_before == source_after && inputs_unchanged),
            source_before,
            source_after: Some(source_after),
        },
    };
    write_create_only(&capture_root.join("summary.json"), &report)?;
    render_waypoint_v2_batch(capture_root)
}

fn safe_capture_file(root: &Path, relative: &str) -> Result<PathBuf> {
    let root_metadata = fs::symlink_metadata(root)?;
    ensure!(
        root_metadata.is_dir() && !root_metadata.file_type().is_symlink(),
        "capture root must be a real directory"
    );
    let relative_path = Path::new(relative);
    ensure!(
        !relative_path.is_absolute()
            && relative_path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "unsafe capture-relative path {relative}"
    );
    let canonical_root = root.canonicalize()?;
    let mut cursor = canonical_root.clone();
    for component in relative_path.components() {
        cursor.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&cursor)
            .with_context(|| format!("missing batch artifact {relative}"))?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "symlink in captured artifact path {relative}"
        );
    }
    let resolved = cursor.canonicalize()?;
    ensure!(
        resolved.starts_with(canonical_root) && resolved.is_file(),
        "batch artifact escapes capture root or is not a regular file: {relative}"
    );
    Ok(resolved)
}

fn case_result_matches(case: &WaypointV2BatchCase, result: &WaypointV2FlightResult) -> Result<()> {
    ensure!(case.planning_stop.as_deref() == Some(enum_text(&result.planning_stop)?.as_str()));
    ensure!(case.reason == result.reason);
    ensure!(case.correction_count == Some(result.correction_count));
    ensure!(case.initial_nominal_terrain_blocked == Some(result.initial_nominal_terrain_blocked));
    ensure!(case.integrity_passed == Some(result.integrity_passed));
    ensure!(case.final_source_replay_passed == Some(result.final_source_replay_passed));
    ensure!(
        case.physical_outcome
            == result
                .physical_outcome
                .as_ref()
                .map(enum_text)
                .transpose()?
    );
    ensure!(case.mission_outcome == result.mission_outcome.as_ref().map(enum_text).transpose()?);
    ensure!(case.planning_s == Some(result.timings.planning_s));
    ensure!(case.execution_s == Some(result.timings.execution_s));
    ensure!(case.replay_s == Some(result.timings.replay_s));
    ensure!(
        case.status
            == if result.manifest.is_some() {
                "simulated"
            } else if result.ordinary_flight.is_some() {
                "simulation_unverified"
            } else {
                "preflight_rejected"
            }
    );
    Ok(())
}

fn validate_full_evidence_consistency(
    input: &WaypointV2PackInput,
    result: &WaypointV2FlightResult,
    compact: &Value,
) -> Result<()> {
    let manifest = result
        .manifest
        .as_ref()
        .context("simulated V2 result is missing its manifest")?;
    let ordinary = result
        .ordinary_flight
        .as_ref()
        .context("simulated V2 result is missing ordinary flight evidence")?;
    let final_state = &ordinary.final_state;
    let scenario_id = input.scenario.id.as_str();

    ensure!(
        manifest.scenario_id == scenario_id
            && manifest.physics_hz == input.scenario.sim.physics_hz
            && manifest.controller_hz == input.scenario.sim.controller_hz,
        "full-flight manifest scenario or clock differs from captured input for {scenario_id}"
    );
    ensure!(
        result.physical_outcome.as_ref() == Some(&manifest.physical_outcome)
            && result.mission_outcome.as_ref() == Some(&manifest.mission_outcome),
        "V2 result physical or mission outcome differs from manifest for {scenario_id}"
    );
    ensure!(
        manifest.physics_steps == final_state.physics_step
            && manifest.sim_time_s == final_state.sim_time_s
            && manifest.end_reason == final_state.end_reason
            && manifest.physical_outcome == final_state.physical_outcome
            && manifest.mission_outcome == final_state.mission_outcome,
        "manifest endpoint differs from ordinary final state for {scenario_id}"
    );
    ensure!(
        manifest.controller_updates == ordinary.actions.len() as u64,
        "manifest controller update count differs from saved action count for {scenario_id}"
    );
    if ordinary.actions.is_empty() {
        ensure!(
            manifest.physics_steps == 0
                && manifest.controller_updates == 0
                && manifest.sim_time_s == 0.0
                && ordinary.samples.len() == 1
                && ordinary.samples[0].physics_step == 0
                && ordinary.samples[0].sim_time_s == 0.0,
            "zero-command full-flight evidence must retain only the initial zero-step sample for {scenario_id}"
        );
    }
    ensure!(
        manifest.summary.fuel_remaining_kg == final_state.fuel_kg
            && manifest.summary.min_touchdown_clearance_m == final_state.min_touchdown_clearance_m
            && manifest.summary.min_hull_clearance_m == final_state.min_hull_clearance_m,
        "manifest fuel or clearance summary differs from ordinary final state for {scenario_id}"
    );

    let saved_run = &compact["run_summary"];
    ensure!(
        saved_run.is_object(),
        "simulated V2 result is missing compact run summary for {scenario_id}"
    );
    ensure!(
        saved_run["endpoint"]["physics_step"] == manifest.physics_steps
            && saved_run["endpoint"]["sim_time_s"] == manifest.sim_time_s
            && saved_run["endpoint"]["physical_outcome"]
                == serde_json::to_value(&manifest.physical_outcome)?
            && saved_run["endpoint"]["mission_outcome"]
                == serde_json::to_value(&manifest.mission_outcome)?
            && saved_run["endpoint"]["end_reason"] == serde_json::to_value(&manifest.end_reason)?,
        "compact endpoint differs from full-flight manifest for {scenario_id}"
    );
    ensure!(
        saved_run["fuel"]["remaining_kg"] == manifest.summary.fuel_remaining_kg
            && saved_run["fuel"]["used_kg"] == manifest.summary.fuel_used_kg
            && saved_run["minimum_clearance"]["touchdown_m"]
                == manifest.summary.min_touchdown_clearance_m
            && saved_run["minimum_clearance"]["hull_m"] == manifest.summary.min_hull_clearance_m
            && saved_run["minimum_clearance"]["landing"]
                == serde_json::to_value(&manifest.summary.landing)?,
        "compact fuel or minimum-clearance values differ from full-flight manifest for {scenario_id}"
    );
    Ok(())
}

fn validate_batch_capture(
    root: &Path,
    report: &WaypointV2BatchReport,
) -> Result<Vec<(WaypointV2PackInput, WaypointV2FlightResult)>> {
    ensure!(report.schema_id == WAYPOINT_V2_BATCH_SCHEMA_ID);
    ensure!(report.status == "completed");
    ensure!(report.case_count == report.cases.len() && report.cases.len() == EXPECTED_CASE_COUNT);
    ensure!(report.pack_id == "planner_v2_lab_suite");
    let pack_bytes = fs::read(safe_capture_file(root, &report.pack_snapshot_path)?)?;
    ensure!(sha256_bytes(&pack_bytes)? == report.pack_snapshot_sha256);
    ensure!(report.pack_snapshot_sha256 == report.input_identity.pack_file_sha256);
    let inputs_bytes = fs::read(safe_capture_file(
        root,
        &report.expanded_inputs_snapshot_path,
    )?)?;
    ensure!(sha256_bytes(&inputs_bytes)? == report.expanded_inputs_snapshot_sha256);
    let inputs: Vec<WaypointV2PackInput> = serde_json::from_slice(&inputs_bytes)?;
    ensure!(inputs.len() == report.cases.len());
    ensure!(
        digest_expanded_inputs(&inputs)? == report.input_identity.rust_typed_expanded_inputs_sha256
    );
    let mut verified = Vec::with_capacity(inputs.len());
    let mut seen = BTreeSet::new();
    for (input, case) in inputs.into_iter().zip(&report.cases) {
        ensure!(
            seen.insert(case.case_id.as_str()),
            "duplicate report case ID"
        );
        ensure!(
            input.case_id == case.case_id
                && input.source_set == case.source_set
                && input.source_group == case.source_group
                && input.group == case.group
                && input.family == case.family
                && input.base_case_id == case.base_case_id
                && input.source_pad_id == case.source_pad_id
                && input.target_pad_id == case.target_pad_id
                && input.expected_preflight == case.expected_preflight,
            "case row does not match expanded input {}",
            case.case_id
        );
        ensure!(case.input_path == source_path(&input));
        let scenario_bytes = fs::read(safe_capture_file(root, &case.scenario_path)?)?;
        ensure!(
            case.artifact_sha256.get(&case.scenario_path) == Some(&sha256_bytes(&scenario_bytes)?),
            "scenario artifact seal mismatch for {}",
            case.case_id
        );
        let scenario: ScenarioSpec = serde_json::from_slice(&scenario_bytes)?;
        ensure!(
            scenario == input.scenario,
            "captured scenario drift for {}",
            case.case_id
        );
        let flight_bytes = fs::read(safe_capture_file(root, &case.flight_path)?)?;
        ensure!(
            case.artifact_sha256.get(&case.flight_path) == Some(&sha256_bytes(&flight_bytes)?),
            "flight artifact seal mismatch for {}",
            case.case_id
        );
        let result: WaypointV2FlightResult = serde_json::from_slice(&flight_bytes)?;
        case_result_matches(case, &result)?;
        ensure!(result.policy == policy_for_version(report.policy_version)?);
        ensure!(
            case.outcome
                == case
                    .physical_outcome
                    .clone()
                    .or_else(|| case.planning_stop.clone())
        );
        match case.expected_preflight.as_deref() {
            Some("unsupported") => ensure!(
                result.planning_stop == WaypointV2Stop::Unsupported
                    && result.manifest.is_none()
                    && result.ordinary_flight.is_none(),
                "unexpected execution for unsupported case {}",
                case.case_id
            ),
            Some("invalid_input") => ensure!(
                result.planning_stop == WaypointV2Stop::InvalidInput
                    && result.manifest.is_none()
                    && result.ordinary_flight.is_none(),
                "unexpected execution for invalid-input case {}",
                case.case_id
            ),
            None => ensure!(
                !matches!(
                    result.planning_stop,
                    WaypointV2Stop::Unsupported | WaypointV2Stop::InvalidInput
                ),
                "unexpected preflight rejection for {}",
                case.case_id
            ),
            Some(other) => bail!("unsupported expected preflight {other}"),
        }
        let run_summary_bytes = fs::read(safe_capture_file(root, &case.summary_path)?)?;
        ensure!(
            case.artifact_sha256.get(&case.summary_path)
                == Some(&sha256_bytes(&run_summary_bytes)?),
            "run summary seal mismatch for {}",
            case.case_id
        );
        let compact: Value = serde_json::from_slice(&run_summary_bytes)?;
        ensure!(compact["schema_id"] == "waypoint_v2_flight_summary_v1");
        ensure!(compact["input_identity"] == result.input_identity);
        ensure!(compact["result"]["planning_stop"] == serde_json::to_value(result.planning_stop)?);
        ensure!(compact["result"]["reason"] == serde_json::to_value(&result.reason)?);
        ensure!(compact["result"]["correction_count"] == result.correction_count);
        ensure!(
            compact["result"]["initial_nominal_terrain_blocked"]
                == result.initial_nominal_terrain_blocked
        );
        ensure!(compact["result"]["integrity_passed"] == result.integrity_passed);
        ensure!(
            compact["result"]["final_source_replay_passed"] == result.final_source_replay_passed
        );
        ensure!(
            compact["result"]["physical_outcome"]
                == serde_json::to_value(&result.physical_outcome)?
        );
        ensure!(
            compact["result"]["mission_outcome"] == serde_json::to_value(&result.mission_outcome)?
        );
        ensure!(compact["policy"] == serde_json::to_value(&result.policy)?);
        ensure!(compact["result"]["timings"] == serde_json::to_value(&result.timings)?);
        if result.manifest.is_none() {
            ensure!(
                compact["run_summary"].is_null(),
                "compact run summary claims simulator evidence without a manifest for {}",
                case.case_id
            );
        }
        if let Some(raw_report) = &case.rich_report_path {
            ensure!(raw_report == &format!("runs/{}/report.html", case.case_id));
            let bytes = fs::read(safe_capture_file(root, raw_report)?)?;
            ensure!(
                case.artifact_sha256.get(raw_report) == Some(&sha256_bytes(&bytes)?),
                "raw report seal mismatch for {}",
                case.case_id
            );
            ensure!(result.manifest.is_some() && result.ordinary_flight.is_some());
            validate_full_evidence_consistency(&input, &result, &compact)?;
        } else if case.status == "simulation_unverified" {
            ensure!(result.manifest.is_none() && result.ordinary_flight.is_some());
        } else {
            ensure!(result.manifest.is_none() && result.ordinary_flight.is_none());
            ensure!(case.status == "preflight_rejected");
        }
        ensure!(
            case.annotated_report_path == format!("runs/{}/index.html", case.case_id),
            "annotated report path mismatch for {}",
            case.case_id
        );
        verified.push((input, result));
    }
    ensure!(
        summarize(&report.cases)? == report.summary,
        "batch summary does not match case rows"
    );
    Ok(verified)
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn detail_html(
    input: &WaypointV2PackInput,
    case: &WaypointV2BatchCase,
    result: &WaypointV2FlightResult,
    index: usize,
    cases: &[WaypointV2BatchCase],
    source_base: Option<&str>,
) -> Result<String> {
    let source_link = |relative: &str, local: &str| {
        source_base.map_or_else(|| local.to_owned(), |base| format!("{base}{relative}"))
    };
    if result.manifest.is_some() {
        let prior = index.checked_sub(1).map(|i| &cases[i]);
        let next = cases.get(index + 1);
        let link = |label: &str, href: String| NavigationLink {
            label: label.into(),
            href,
        };
        let navigation = AnnotationNavigation {
            home: Some(link("Report home", "/reports/".into())),
            collection: Some(link("Batch summary", "../../index.html".into())),
            previous: prior
                .map(|case| link("Previous case", format!("../{}/index.html", case.case_id))),
            next: next.map(|case| link("Next case", format!("../{}/index.html", case.case_id))),
            source_links: vec![
                link(
                    "Scenario JSON",
                    source_link(&case.scenario_path, "scenario.json"),
                ),
                link("Flight JSON", source_link(&case.flight_path, "flight.json")),
                link(
                    "Run summary JSON",
                    source_link(&case.summary_path, "summary.json"),
                ),
                link(
                    "Expanded batch inputs",
                    source_link("expanded-inputs.json", "../../expanded-inputs.json"),
                ),
                link(
                    "Waypoint planning topic",
                    "/reports/topics/waypoint-planning/index.html".into(),
                ),
            ],
        };
        return render_rich_flight(
            &input.scenario,
            result,
            navigation,
            format!(
                "Native V2 lab · {} · {} / {} · policy {}",
                case.case_id,
                case.group.as_str(),
                case.family,
                result.policy.policy_id
            ),
        );
    }
    let stop = escape_html(case.planning_stop.as_deref().unwrap_or("missing"));
    let reason = escape_html(
        case.reason
            .as_deref()
            .unwrap_or("No preflight reason recorded."),
    );
    let status_message = if case.status == "preflight_rejected" {
        "This input was rejected during preflight. No simulator trajectory or rich flight report was created."
    } else {
        "A simulator attempt exists, but no final-source-validated trajectory report was created. Treat this as unverified evidence, not as a preflight rejection or a landing."
    };
    let previous = index
        .checked_sub(1)
        .and_then(|previous| cases.get(previous))
        .map(|previous| {
            format!(
                "<a rel=\"prev\" href=\"../{}/index.html\">Previous case</a>",
                escape_html(&previous.case_id)
            )
        })
        .unwrap_or_default();
    let next = cases
        .get(index + 1)
        .map(|next| {
            format!(
                "<a rel=\"next\" href=\"../{}/index.html\">Next case</a>",
                escape_html(&next.case_id)
            )
        })
        .unwrap_or_default();
    let navigation = format!(
        r#"<nav><a href="../../index.html">Batch summary</a> · <a href="/reports/">Report home</a> · <a href="/reports/topics/waypoint-planning/index.html">Waypoint planning</a> · {previous} {next}</nav>"#
    );
    let body = format!(
        r###"<p>{}</p><p>Planning stop: <code>{}</code></p><p>{}</p><p>Expected preflight: <code>{}</code></p><ul><li><a href="{}">Scenario JSON</a></li><li><a href="{}">Typed preflight result JSON</a></li><li><a href="{}">Run summary JSON</a></li><li><a href="{}">Expanded inputs snapshot</a></li></ul>"###,
        escape_html(status_message),
        stop,
        reason,
        escape_html(case.expected_preflight.as_deref().unwrap_or("none")),
        escape_html(&source_link(&case.scenario_path, "scenario.json")),
        escape_html(&source_link(&case.flight_path, "flight.json")),
        escape_html(&source_link(&case.summary_path, "summary.json")),
        escape_html(&source_link(
            "expanded-inputs.json",
            "../../expanded-inputs.json"
        ))
    );
    Ok(pd_report::batch::render_status_page(
        &case.case_id,
        &navigation,
        &body,
    ))
}

fn write_derived(path: &Path, contents: &[u8]) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "refusing unsafe derived page target {}",
            path.display()
        );
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temp = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&temp, path).with_context(|| format!("write derived page {}", path.display()))
}

pub fn render_waypoint_v2_batch(capture_root: &Path) -> Result<WaypointV2BatchReport> {
    let summary_path = safe_capture_file(capture_root, "summary.json")?;
    let report: WaypointV2BatchReport = serde_json::from_slice(&fs::read(summary_path)?)?;
    let verified = validate_batch_capture(capture_root, &report)?;
    let mut pages = Vec::with_capacity(report.cases.len() + 2);
    for (index, ((input, result), case)) in verified.iter().zip(&report.cases).enumerate() {
        let html = detail_html(input, case, result, index, &report.cases, None)?;
        let path = safe_output_path(capture_root, &case.annotated_report_path)?;
        pages.push((path, html));
    }
    let batch_html = tree_report::render(&report, &verified, None)?;
    pages.push((
        safe_output_path(capture_root, "index.html")?,
        batch_html.clone(),
    ));
    pages.push((safe_output_path(capture_root, "report.html")?, batch_html));
    // All raw artifacts and every generated page are validated/built before the first write.
    for (path, html) in pages {
        write_derived(&path, html.as_bytes())?;
    }
    Ok(report)
}

/// Validate a native capture without changing any captured or derived artifacts.
pub fn validated_waypoint_v2_batch(capture_root: &Path) -> Result<WaypointV2BatchReport> {
    let bytes = fs::read(safe_capture_file(capture_root, "summary.json")?)?;
    let report: WaypointV2BatchReport = serde_json::from_slice(&bytes)?;
    validate_batch_capture(capture_root, &report)?;
    Ok(report)
}

/// Build the normal report-site edition entirely in memory. Raw evidence links
/// point to the capture, while normal batch/case navigation stays in the site.
pub fn render_waypoint_v2_site_pages(
    capture_root: &Path,
    source_base: &str,
) -> Result<(WaypointV2BatchReport, Vec<(PathBuf, String)>)> {
    ensure!(source_base.starts_with('/') && source_base.ends_with('/'));
    ensure!(
        source_base
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-_.%".contains(&b)),
        "unsafe capture URL"
    );
    let report: WaypointV2BatchReport =
        serde_json::from_slice(&fs::read(safe_capture_file(capture_root, "summary.json")?)?)?;
    let verified = validate_batch_capture(capture_root, &report)?;
    let mut pages = Vec::with_capacity(report.case_count + 1);
    for (index, ((input, result), case)) in verified.iter().zip(&report.cases).enumerate() {
        pages.push((
            PathBuf::from(&case.annotated_report_path),
            detail_html(input, case, result, index, &report.cases, Some(source_base))?,
        ));
    }
    pages.push((
        PathBuf::from("index.html"),
        tree_report::render_site(&report, &verified, source_base)?,
    ));
    Ok((report, pages))
}

/// Build only a separate tree preview, linking to unchanged captured details.
/// No simulation, detail regeneration, current selection or site publication.
pub fn render_waypoint_v2_batch_preview(
    capture_root: &Path,
    preview_root: &Path,
) -> Result<PathBuf> {
    let summary_path = safe_capture_file(capture_root, "summary.json")?;
    let summary_bytes = fs::read(summary_path)?;
    let report: WaypointV2BatchReport = serde_json::from_slice(&summary_bytes)?;
    let verified = validate_batch_capture(capture_root, &report)?;
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("repository root")?;
    let outputs = repo.join("outputs").canonicalize()?;
    let source = capture_root.canonicalize()?;
    let relative_source = source
        .strip_prefix(&outputs)
        .context("preview capture must be beneath served outputs")?;
    let destination = preview_destination(&outputs, &source, preview_root)?;
    let source_href = format!("/{}/", relative_source.to_string_lossy().replace('\\', "/"));
    ensure!(
        source_href
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-_.".contains(&b)),
        "unsafe capture URL"
    );
    let html = tree_report::render(&report, &verified, Some(&source_href))?;
    let renderer_source = capture_source_state(repo)?;
    reserve_output_root(&destination)?;
    write_bytes_create_only(&destination.join("index.html"), html.as_bytes())?;
    write_create_only(
        &destination.join("preview.json"),
        &serde_json::json!({
            "schema_id":"planner_v2_batch_tree_preview_v1", "source_capture":source,
            "source_summary_sha256":sha256_bytes(&summary_bytes)?, "capture_base_href":source_href,
            "rendered_html_sha256":sha256_bytes(html.as_bytes())?, "renderer_source":renderer_source,
            "case_count":report.case_count, "scope":"presentation only; current batch and detailed pages unchanged"
        }),
    )?;
    Ok(destination.join("index.html"))
}

fn preview_destination(outputs: &Path, source: &Path, requested: &Path) -> Result<PathBuf> {
    let destination = if requested.is_absolute() {
        requested.to_owned()
    } else {
        std::env::current_dir()?.join(requested)
    };
    ensure!(
        !destination
            .components()
            .any(|c| matches!(c, Component::ParentDir)),
        "preview destination cannot contain parent traversal"
    );
    ensure!(
        destination.starts_with(outputs.join("research"))
            && destination != outputs.join("research")
            && !destination.starts_with(source),
        "preview must be a separate directory beneath outputs/research"
    );
    let mut cursor = PathBuf::new();
    for component in destination.components() {
        cursor.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&cursor) {
            ensure!(
                !metadata.file_type().is_symlink(),
                "symlink in preview destination"
            );
        }
    }
    Ok(destination)
}

fn safe_output_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    ensure!(
        !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "unsafe derived output path {relative}"
    );
    let root_metadata = fs::symlink_metadata(root)?;
    ensure!(
        root_metadata.is_dir() && !root_metadata.file_type().is_symlink(),
        "capture root must be a real directory"
    );
    let canonical_root = root.canonicalize()?;
    let components = path.components().collect::<Vec<_>>();
    let mut cursor = canonical_root.clone();
    for (index, component) in components.iter().enumerate() {
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if index + 1 == components.len() => ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "unsafe derived page target {}",
                cursor.display()
            ),
            Ok(metadata) => ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "unsafe derived page parent {}",
                cursor.display()
            ),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && index + 1 == components.len() => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("pd-v2-pack-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn preview_destination_is_research_only_without_writes() {
        let fixture = temp_dir("preview-guard");
        let outputs = fixture.join("outputs");
        let research = outputs.join("research");
        let source = research.join("capture");
        fs::create_dir_all(&source).expect("fixture");
        let allowed = research.join("new-preview");
        assert_eq!(
            preview_destination(&outputs, &source, &allowed).unwrap(),
            allowed
        );
        assert!(!allowed.exists(), "validation must not create anything");
        for rejected in [
            outputs.join("reports/new-preview"),
            outputs.join("eval/new-preview"),
            research.clone(),
            fixture.join("outside"),
            research.join("nested/../new-preview"),
            source.join("new-preview"),
        ] {
            assert!(
                preview_destination(&outputs, &source, &rejected).is_err(),
                "{}",
                rejected.display()
            );
            assert!(!rejected.exists() || rejected == research);
        }
        #[cfg(unix)]
        {
            let link = research.join("linked");
            std::os::unix::fs::symlink(fixture.join("missing"), &link).expect("symlink");
            assert!(preview_destination(&outputs, &source, &link.join("preview")).is_err());
        }
        fs::remove_dir_all(&fixture).expect("remove owned test fixture");
    }

    fn report_case(input: &WaypointV2PackInput, index: usize) -> WaypointV2BatchCase {
        let preflight_rejected = input.expected_preflight.as_deref() == Some("unsupported");
        let landed = index < 2;
        WaypointV2BatchCase {
            case_id: input.case_id.clone(),
            source_set: input.source_set.clone(),
            source_group: input.source_group.clone(),
            group: input.group.clone(),
            family: input.family.clone(),
            base_case_id: input.base_case_id.clone(),
            source_pad_id: input.source_pad_id.clone(),
            target_pad_id: input.target_pad_id.clone(),
            expected_preflight: input.expected_preflight.clone(),
            status: if preflight_rejected {
                "preflight_rejected"
            } else {
                "simulated"
            }
            .into(),
            outcome: Some(
                if landed {
                    "landed_on_target"
                } else {
                    "no_clearing"
                }
                .into(),
            ),
            planning_stop: Some(if preflight_rejected {
                "unsupported".into()
            } else if landed {
                "landed".into()
            } else {
                "no_clearing".into()
            }),
            reason: None,
            correction_count: Some(if index == 1 { 2 } else { 0 }),
            initial_nominal_terrain_blocked: Some(index >= 11),
            integrity_passed: Some(true),
            final_source_replay_passed: Some(!preflight_rejected),
            physical_outcome: if preflight_rejected {
                None
            } else if landed {
                Some("landed_on_target".into())
            } else {
                Some("flying".into())
            },
            mission_outcome: if preflight_rejected {
                None
            } else if landed {
                Some("success".into())
            } else {
                Some("in_progress".into())
            },
            planning_s: Some(index as f64),
            execution_s: Some(0.1),
            replay_s: Some(0.2),
            input_path: source_path(input).into(),
            scenario_path: format!("runs/{}/scenario.json", input.case_id),
            flight_path: format!("runs/{}/flight.json", input.case_id),
            summary_path: format!("runs/{}/summary.json", input.case_id),
            rich_report_path: (!preflight_rejected)
                .then(|| format!("runs/{}/report.html", input.case_id)),
            annotated_report_path: format!("runs/{}/index.html", input.case_id),
            artifact_sha256: BTreeMap::new(),
            error: None,
        }
    }

    fn full_evidence_fixture() -> (
        WaypointV2PackInput,
        WaypointV2FlightResult,
        serde_json::Value,
    ) {
        let input = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH))
            .unwrap()
            .inputs
            .remove(0);
        let final_state = pd_core::SimulationStateSnapshotV1 {
            sim_time_s: 0.1,
            physics_step: 12,
            position_m: Vec2::default(),
            velocity_mps: Vec2::default(),
            attitude_rad: 0.0,
            angular_rate_radps: 0.0,
            fuel_kg: 99.0,
            held_command: pd_core::Command::idle(),
            physical_outcome: pd_core::PhysicalOutcome::LandedOnTarget,
            mission_outcome: pd_core::MissionOutcome::Success,
            end_reason: pd_core::EndReason::TouchdownOnTarget,
            min_touchdown_clearance_m: 0.1,
            min_hull_clearance_m: 0.2,
            max_speed_mps: 0.0,
            max_abs_attitude_rad: 0.0,
            max_abs_angular_rate_radps: 0.0,
            waypoint_sequence_passed: 0,
            waypoint_sequence_first_failure_index: None,
            waypoint_handoff_window_index: None,
        };
        let summary = pd_core::RunSummary {
            fuel_remaining_kg: 99.0,
            fuel_used_kg: 1.0,
            min_touchdown_clearance_m: 0.1,
            min_hull_clearance_m: 0.2,
            landing: Some(pd_core::LandingRunSummary::default()),
            ..pd_core::RunSummary::default()
        };
        let manifest = pd_core::RunManifest {
            schema_version: 1,
            scenario_id: input.scenario.id.clone(),
            scenario_name: input.scenario.name.clone(),
            scenario_seed: 0,
            scenario_tags: input.scenario.tags.clone(),
            controller_id: "synthetic-test-controller".into(),
            physics_hz: input.scenario.sim.physics_hz,
            controller_hz: input.scenario.sim.controller_hz,
            sim_time_s: 0.1,
            physics_steps: 12,
            controller_updates: 1,
            physical_outcome: pd_core::PhysicalOutcome::LandedOnTarget,
            mission_outcome: pd_core::MissionOutcome::Success,
            end_reason: pd_core::EndReason::TouchdownOnTarget,
            summary,
        };
        let ordinary = crate::LocalClearingOrdinaryEvidenceV1 {
            final_state: final_state.clone(),
            incoming_contact: None,
            actions: vec![pd_core::ActionLogEntry {
                sim_time_s: 0.0,
                physics_step: 0,
                controller_update_index: 1,
                command: pd_core::Command::idle(),
            }],
            events: Vec::new(),
            samples: Vec::new(),
        };
        let result = WaypointV2FlightResult {
            policy: WaypointV2Policy::revision_3(),
            input_identity: "synthetic-consistency-test".into(),
            planning_stop: WaypointV2Stop::Landed,
            reason: None,
            correction_count: 0,
            initial_nominal_terrain_blocked: false,
            integrity_passed: true,
            physical_outcome: Some(pd_core::PhysicalOutcome::LandedOnTarget),
            mission_outcome: Some(pd_core::MissionOutcome::Success),
            absolute_deadline_physics_step: None,
            cycles: Vec::new(),
            segments: Vec::new(),
            ordinary_flight: Some(ordinary),
            final_source_replay_passed: true,
            manifest: Some(manifest.clone()),
            failed_local_row: None,
            timings: crate::WaypointV2Timings::default(),
        };
        let compact = serde_json::json!({
            "run_summary": {
                "minimum_clearance": {
                    "touchdown_m": manifest.summary.min_touchdown_clearance_m,
                    "hull_m": manifest.summary.min_hull_clearance_m,
                    "landing": manifest.summary.landing,
                },
                "fuel": {
                    "remaining_kg": manifest.summary.fuel_remaining_kg,
                    "used_kg": manifest.summary.fuel_used_kg,
                },
                "endpoint": {
                    "physics_step": manifest.physics_steps,
                    "sim_time_s": manifest.sim_time_s,
                    "physical_outcome": manifest.physical_outcome,
                    "mission_outcome": manifest.mission_outcome,
                    "end_reason": manifest.end_reason,
                }
            }
        });
        (input, result, compact)
    }

    #[test]
    fn full_flight_manifest_and_compact_endpoint_are_cross_bound() {
        let (input, result, compact) = full_evidence_fixture();
        validate_full_evidence_consistency(&input, &result, &compact).unwrap();

        let mut wrong_compact_clock = compact.clone();
        wrong_compact_clock["run_summary"]["endpoint"]["physics_step"] = serde_json::json!(13);
        assert!(validate_full_evidence_consistency(&input, &result, &wrong_compact_clock).is_err());

        let mut wrong_action_count = result.clone();
        wrong_action_count
            .manifest
            .as_mut()
            .unwrap()
            .controller_updates += 1;
        assert!(validate_full_evidence_consistency(&input, &wrong_action_count, &compact).is_err());

        let mut wrong_final_outcome = result;
        wrong_final_outcome
            .ordinary_flight
            .as_mut()
            .unwrap()
            .final_state
            .physical_outcome = pd_core::PhysicalOutcome::Flying;
        assert!(
            validate_full_evidence_consistency(&input, &wrong_final_outcome, &compact).is_err()
        );
    }

    #[test]
    fn zero_command_flight_requires_zero_clock_and_only_initial_sample() {
        let (input, mut result, mut compact) = full_evidence_fixture();
        let ordinary = result.ordinary_flight.as_mut().unwrap();
        ordinary.actions.clear();
        ordinary.samples = vec![pd_core::SampleRecord {
            sim_time_s: 0.0,
            physics_step: 0,
            observation: pd_core::Observation {
                sim_time_s: 0.0,
                physics_step: 0,
                position_m: Vec2::default(),
                velocity_mps: Vec2::default(),
                attitude_rad: 0.0,
                angular_rate_radps: 0.0,
                mass_kg: 100.0,
                fuel_kg: 99.0,
                gravity_mps2: 1.62,
                target_dx_m: 0.0,
                height_above_target_m: 0.0,
                target_surface_y_m: 0.0,
                target_pad_half_width_m: 2.0,
                touchdown_clearance_m: 0.0,
                min_hull_clearance_m: 0.0,
            },
            held_command: pd_core::Command::idle(),
        }];
        ordinary.final_state.physics_step = 0;
        ordinary.final_state.sim_time_s = 0.0;
        let manifest = result.manifest.as_mut().unwrap();
        manifest.physics_steps = 0;
        manifest.controller_updates = 0;
        manifest.sim_time_s = 0.0;
        compact["run_summary"]["endpoint"]["physics_step"] = serde_json::json!(0);
        compact["run_summary"]["endpoint"]["sim_time_s"] = serde_json::json!(0.0);
        validate_full_evidence_consistency(&input, &result, &compact).unwrap();

        let mut nonzero_clock = result.clone();
        nonzero_clock
            .ordinary_flight
            .as_mut()
            .unwrap()
            .final_state
            .physics_step = 1;
        nonzero_clock.manifest.as_mut().unwrap().physics_steps = 1;
        let mut nonzero_compact = compact.clone();
        nonzero_compact["run_summary"]["endpoint"]["physics_step"] = serde_json::json!(1);
        assert!(
            validate_full_evidence_consistency(&input, &nonzero_clock, &nonzero_compact).is_err()
        );

        let mut missing_initial_sample = result;
        missing_initial_sample
            .ordinary_flight
            .as_mut()
            .unwrap()
            .samples
            .clear();
        assert!(
            validate_full_evidence_consistency(&input, &missing_initial_sample, &compact).is_err()
        );
    }

    #[test]
    fn native_pack_expands_exactly_44_route_free_inputs_in_source_order() {
        let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
        assert_eq!(expanded.inputs.len(), 44);
        assert_eq!(expanded.inputs[0].case_id, "v2_clear_685");
        assert_eq!(expanded.inputs[31].case_id, "v2_diag_other_vehicle");
        assert_eq!(expanded.inputs[32].case_id, "fresh_clear_flat_805");
        assert_eq!(
            expanded.inputs[43].case_id,
            "fresh_compound_downhill_plateau_805"
        );
        let counts = expanded
            .inputs
            .iter()
            .fold(BTreeMap::new(), |mut counts, input| {
                *counts.entry(input.group.as_str()).or_insert(0usize) += 1;
                counts
            });
        assert_eq!(counts["clear"], 11);
        assert_eq!(counts["ordinary"], 16);
        assert_eq!(counts["additional_terrain"], 9);
        assert_eq!(counts["diagnostic"], 8);
        assert!(expanded.inputs.iter().all(|input| {
            input.scenario.id == input.case_id
                && input.scenario.mission.transfer_route.is_none()
                && matches!(
                    input.scenario.mission.goal,
                    EvaluationGoal::LandingOnPad { .. }
                )
        }));
        let all_cases = expanded
            .inputs
            .iter()
            .enumerate()
            .map(|(index, input)| report_case(input, index))
            .collect::<Vec<_>>();
        let diagnostic = expanded
            .inputs
            .iter()
            .enumerate()
            .filter(|(_, input)| input.expected_preflight.is_some())
            .collect::<Vec<_>>();
        assert_eq!(diagnostic.len(), 2);
        for (index, input) in diagnostic {
            let preflight =
                preflight_waypoint_v2_flight(&request_for(input), &WaypointV2Policy::revision_3());
            assert_eq!(preflight.rejection, Some(WaypointV2Stop::Unsupported));
            assert!(!preflight.simulation_created);
            let result =
                crate::run_waypoint_v2_flight(&request_for(input), &WaypointV2Policy::revision_3())
                    .unwrap();
            assert!(result.manifest.is_none());
            assert!(result.ordinary_flight.is_none());
            let page =
                detail_html(input, &all_cases[index], &result, index, &all_cases, None).unwrap();
            assert!(page.contains("No simulator trajectory or rich flight report was created."));
            assert!(!page.contains("<svg"));
            assert!(page.contains("Previous case"));
            assert!(page.contains("Next case"));
        }
        assert_eq!(
            digest_expanded_inputs(&expanded.inputs).unwrap(),
            expanded.input_identity.rust_typed_expanded_inputs_sha256
        );
    }

    #[test]
    fn worker_count_clamps_to_cases_and_rejects_zero() {
        assert_eq!(bounded_worker_count(1).unwrap(), 1);
        assert_eq!(bounded_worker_count(44).unwrap(), 44);
        assert_eq!(bounded_worker_count(128).unwrap(), 44);
        assert!(bounded_worker_count(0).is_err());
    }

    #[test]
    #[ignore = "compares typed expansion with retained accepted archive captures"]
    fn expanded_scenarios_match_retained_archive_values_exactly() {
        let repo = repository_root();
        let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
        for input in &expanded.inputs {
            let run_root = if input.source_set == "practical_suite" {
                repo.join(crate::waypoint_v2_report::PINNED_CAPTURE)
                    .join("runs")
            } else {
                repo.join(
                    "outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a/runs",
                )
            };
            let archive: ScenarioSpec = serde_json::from_slice(
                &fs::read(run_root.join(&input.case_id).join("scenario.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                input.scenario, archive,
                "typed scenario drift for {}",
                input.case_id
            );
        }
    }

    #[test]
    fn invalid_policy_fails_before_reserving_capture_root() {
        let temp = temp_dir("invalid-policy");
        fs::create_dir_all(&temp).unwrap();
        let original = fs::read(repository_root().join(DEFAULT_PLANNER_PACK_PATH)).unwrap();
        let mut value: Value = serde_json::from_slice(&original).unwrap();
        value["policy_version"] = Value::from(4);
        let bad_pack = temp.join("bad-pack.json");
        fs::write(&bad_pack, serde_json::to_vec(&value).unwrap()).unwrap();
        let capture = temp.join("capture");
        assert!(run_waypoint_v2_pack(&bad_pack, &capture, 1).is_err());
        assert!(!capture.exists());
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn summary_separates_not_applicable_diagnostic_replay_from_finite_misses() {
        let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
        let cases = expanded
            .inputs
            .iter()
            .enumerate()
            .map(|(index, input)| report_case(input, index))
            .collect::<Vec<_>>();
        let summary = summarize(&cases).unwrap();
        assert_eq!(summary.valid_landing_count, 2);
        assert_eq!(summary.direct_landing_count, 1);
        assert_eq!(summary.corrected_landing_count, 1);
        assert_eq!(summary.unsupported_count, 2);
        assert_eq!(summary.non_landing_count, 34);
        assert_eq!(summary.diagnostic_landing_count, 0);
        assert_eq!(summary.diagnostic_non_landing_count, 6);
        assert_eq!(summary.final_source_replay_passed_count, 42);
        assert_eq!(summary.final_source_replay_failed_count, 0);
        assert_eq!(summary.planning_stops["no_clearing"], 40);
        assert_eq!(summary.integrity_passed_count, 44);
    }

    #[test]
    fn report_only_rejects_missing_raw_input_before_writing_any_derived_pages() {
        let temp = temp_dir("tamper");
        fs::create_dir_all(&temp).unwrap();
        let expanded = load_and_expand(Path::new(DEFAULT_PLANNER_PACK_PATH)).unwrap();
        let pack_bytes = expanded.pack_bytes.clone();
        let inputs_bytes = serde_json::to_vec_pretty(&expanded.inputs).unwrap();
        write_bytes_create_only(&temp.join("pack.json"), &pack_bytes).unwrap();
        write_bytes_create_only(&temp.join("expanded-inputs.json"), &inputs_bytes).unwrap();
        let cases = expanded
            .inputs
            .iter()
            .enumerate()
            .map(|(index, input)| report_case(input, index))
            .collect::<Vec<_>>();
        let summary = summarize(&cases).unwrap();
        let report = WaypointV2BatchReport {
            schema_id: WAYPOINT_V2_BATCH_SCHEMA_ID.into(),
            pack_id: expanded.definition.id.clone(),
            name: expanded.definition.name.clone(),
            status: "completed".into(),
            policy_version: expanded.definition.policy_version,
            case_count: cases.len(),
            cases,
            summary,
            input_identity: expanded.input_identity,
            pack_snapshot_sha256: sha256_bytes(&pack_bytes).unwrap(),
            expanded_inputs_snapshot_sha256: sha256_bytes(&inputs_bytes).unwrap(),
            pack_snapshot_path: "pack.json".into(),
            expanded_inputs_snapshot_path: "expanded-inputs.json".into(),
            provenance: WaypointV2BatchProvenance {
                source_before: WaypointV2SourceState {
                    git_commit: None,
                    git_dirty: None,
                    rust_source_tree_sha256: "test".into(),
                    executable_sha256: "test".into(),
                },
                source_after: None,
                unchanged_during_capture: None,
            },
        };
        write_create_only(&temp.join("summary.json"), &report).unwrap();
        assert!(validated_waypoint_v2_batch(&temp).is_err());
        assert!(render_waypoint_v2_site_pages(&temp, "/eval/capture/").is_err());
        assert!(render_waypoint_v2_batch(&temp).is_err());
        assert!(!temp.join("index.html").exists());
        assert!(!temp.join("report.html").exists());
        assert!(render_waypoint_v2_batch_preview(&temp, &temp.join("preview")).is_err());
        assert!(!temp.join("preview").exists());
        fs::remove_dir_all(temp).unwrap();
    }
}
