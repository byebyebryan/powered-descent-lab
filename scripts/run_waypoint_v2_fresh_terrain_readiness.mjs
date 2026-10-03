// Bounded, create-only policy 3 readiness harness for the sealed twelve-case
// fresh terrain set. Importing this module only exposes pure utilities.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {
  accessSync,
  constants as fsConstants,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const FIXTURE_PATH = 'fixtures/research/waypoint_v2_fresh_terrain_inputs_v1.json';
const SUMMARY_NAME = 'suite-summary.json';
const BASELINE_ROOT = 'outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a';
const POLICY_ID = 'piecewise_local_clearing_v2_policy_3';
const PHYSICS_HZ = 120;
const CONTROLLER_HZ = 60;
const PHYSICAL_OUTCOMES = new Set([
  'flying', 'landed_on_target', 'landed_off_target', 'crashed', 'timed_out',
]);
const MISSION_OUTCOMES = new Set([
  'in_progress', 'success', 'failed_off_target', 'failed_checkpoint',
  'failed_crash', 'failed_timeout',
]);
const PLANNING_STOPS = new Set([
  'landed', 'no_nominal', 'nominal_rejected', 'no_clearing', 'correction_limit',
  'deadline', 'no_progress', 'unsupported', 'invalid_input', 'implementation_error',
]);

// These are the established output-location and timing exclusions from the
// existing V2 suite runner. State, commands, clock, fuel and outcomes stay exact.
export const SUMMARY_IGNORED_POINTERS = [
  '/result/timings/planning_s',
  '/result/timings/execution_s',
  '/result/timings/replay_s',
  '/timings/output_s',
  '/timings/total_s',
  '/scenario_path',
  '/output_dir',
  '/result/scenario_path',
  '/result/output_dir',
];
export const FLIGHT_IGNORED_POINTERS = [
  '/timings/planning_s',
  '/timings/execution_s',
  '/timings/replay_s',
  '/scenario_path',
  '/output_dir',
];

const sha256 = value => createHash('sha256').update(value).digest('hex');
const canonical = value => Array.isArray(value) ? value.map(canonical)
  : value !== null && typeof value === 'object'
    ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
    : value;
const canonicalText = value => JSON.stringify(canonical(value));
const jsonText = value => `${JSON.stringify(value, null, 2)}\n`;
const isObject = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const deepEqual = (left, right) => canonicalText(left) === canonicalText(right);

function persistedScenario(value) {
  const scenario = structuredClone(value);
  if (!Object.hasOwn(scenario.mission, 'transfer_route')) scenario.mission.transfer_route = null;
  return scenario;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function terrainHeight(points, x) {
  assert(points.length >= 2 && x >= points[0].x && x <= points.at(-1).x,
    'terrain interpolation must remain in-domain');
  for (let index = 1; index < points.length; index++) {
    if (x <= points[index].x) {
      const a = points[index - 1], b = points[index];
      return a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x);
    }
  }
  throw new Error('missing terrain interval');
}

function recipePayload(recipe) {
  return {
    id: recipe.id,
    group: recipe.group,
    family: recipe.family,
    base_case_id: recipe.base_case_id,
    span_m: recipe.span_m,
    target_delta_m: recipe.target_delta_m,
    features: recipe.features,
  };
}

function transform805Base(scenario, original) {
  const source = scenario.world.landing_pads.find(pad => pad.id === original.source_pad_id);
  const target = scenario.world.landing_pads.find(pad => pad.id === original.target_pad_id);
  assert(source && target, 'operational base must name both pads');
  assert.equal(source.center_x_m, -845);
  assert.equal(target.center_x_m, 0);
  assert.equal(source.width_m, 36);
  assert.equal(target.width_m, 36);
  const oldTargetY = target.surface_y_m;
  const requestedTargetY = original.recipe.target_delta_m;
  const scale = oldTargetY === 0 ? 1 : requestedTargetY / oldTargetY;
  if (oldTargetY === 0) assert.equal(requestedTargetY, 0, 'flat source cannot create a slope');

  for (const point of scenario.world.terrain.points_m) {
    if (point.x <= source.center_x_m + source.width_m / 2) point.x += 40;
    else point.y *= scale;
  }
  source.center_x_m = -805;
  target.surface_y_m = requestedTargetY;
  scenario.initial_state.position_m.x = -805;
  return {source, target};
}

function compileRecipe(recipe, originals) {
  const base = originals.get(recipe.base_case_id);
  assert(base, `${recipe.id}: missing tracked base ${recipe.base_case_id}`);
  assert.equal(recipe.group === 'clear', recipe.features.length === 0,
    `${recipe.id}: clear/feature mismatch`);
  assert(['clear', 'terrain'].includes(recipe.group), `${recipe.id}: unknown group`);
  assert(['clear', 'ridge', 'plateau', 'compound'].includes(recipe.family),
    `${recipe.id}: unknown family`);
  const scenario = structuredClone(base.scenario);
  assert(scenario.mission.transfer_route == null,
    `${recipe.id}: authored routes are outside this fixture`);
  assert.equal(scenario.mission.goal?.kind, 'landing_on_pad');
  const originalSource = scenario.world.landing_pads.find(pad => pad.id === base.source_pad_id);
  const originalTarget = scenario.world.landing_pads.find(pad => pad.id === base.target_pad_id);
  assert(originalSource && originalTarget, `${recipe.id}: source pad IDs do not resolve`);
  const baseline = structuredClone(scenario.world.terrain.points_m);

  if (recipe.span_m === 805) {
    assert(recipe.base_case_id.startsWith('operational_'), `${recipe.id}: 805 m base provenance`);
    transform805Base(scenario, {source_pad_id: base.source_pad_id,
      target_pad_id: base.target_pad_id, recipe});
  } else {
    assert.equal(recipe.span_m, 900, `${recipe.id}: unsupported span`);
    assert.equal(originalSource.center_x_m, -900);
    assert.equal(originalTarget.center_x_m, 0);
  }
  const sourcePad = scenario.world.landing_pads.find(pad => pad.id === base.source_pad_id);
  const targetPad = scenario.world.landing_pads.find(pad => pad.id === base.target_pad_id);
  assert.equal(sourcePad.width_m, 36, `${recipe.id}: source shelf width changed`);
  assert.equal(targetPad.width_m, 36, `${recipe.id}: target shelf width changed`);
  assert.equal(sourcePad.center_x_m, -recipe.span_m, `${recipe.id}: span/source mismatch`);
  assert.equal(targetPad.center_x_m, 0, `${recipe.id}: target center changed`);
  assert.equal(targetPad.surface_y_m, recipe.target_delta_m,
    `${recipe.id}: target elevation differs from recipe`);
  assert.deepEqual(scenario.vehicle, base.scenario.vehicle, `${recipe.id}: vehicle changed`);
  assert.equal(scenario.world.gravity_mps2, base.scenario.world.gravity_mps2,
    `${recipe.id}: gravity changed`);
  assert.deepEqual(scenario.sim, base.scenario.sim, `${recipe.id}: clocks/horizon changed`);
  assert.deepEqual(scenario.mission, base.scenario.mission, `${recipe.id}: mission settings changed`);
  assert.equal(scenario.initial_state.position_m.x, -recipe.span_m,
    `${recipe.id}: start is not centered on source shelf`);
  assert.equal(scenario.initial_state.position_m.y, 5, `${recipe.id}: gear offset changed`);

  const basePoints = scenario.world.terrain.points_m;
  assert(basePoints.every((point, index) => Number.isFinite(point.x) && Number.isFinite(point.y)
    && (!index || point.x > basePoints[index - 1].x)), `${recipe.id}: invalid base terrain points`);
  if (recipe.span_m === 805) {
    assert.deepEqual([basePoints[0].x, basePoints.at(-1).x], [-965, 160],
      `${recipe.id}: 805 m outer terrain endpoints changed`);
    assert.deepEqual(basePoints.slice(1, 3).map(point => point.x), [-823, -787],
      `${recipe.id}: 805 m source shelf changed`);
  }
  const baseCopy = structuredClone(basePoints);
  const additions = [];
  let previousFeatureEnd = 0;
  for (const feature of recipe.features) {
    assert([3, 4].includes(feature.fractions.length), `${recipe.id}: ridge/plateau needs 3/4 vertices`);
    assert(feature.fractions.every((fraction, index) => Number.isFinite(fraction)
      && fraction > 0 && fraction < 1 && (!index || fraction > feature.fractions[index - 1])),
    `${recipe.id}: feature fractions must be strictly ordered`);
    assert(feature.fractions[0] > previousFeatureEnd, `${recipe.id}: features overlap`);
    assert(Number.isFinite(feature.height_m) && feature.height_m > 0,
      `${recipe.id}: feature height must be positive`);
    previousFeatureEnd = feature.fractions.at(-1);
    const leftX = sourcePad.center_x_m + feature.fractions[0] * recipe.span_m;
    const rightX = sourcePad.center_x_m + feature.fractions.at(-1) * recipe.span_m;
    assert(leftX > sourcePad.center_x_m + sourcePad.width_m / 2,
      `${recipe.id}: feature touches source shelf`);
    assert(rightX < targetPad.center_x_m - targetPad.width_m / 2,
      `${recipe.id}: feature touches target shelf`);
    assert(baseCopy.every(point => point.x < leftX || point.x > rightX),
      `${recipe.id}: base breakpoint lies inside inserted feature`);
    const offsets = feature.fractions.length === 3
      ? [0, feature.height_m, 0]
      : [0, feature.height_m, feature.height_m, 0];
    for (let index = 0; index < feature.fractions.length; index++) {
      const x = sourcePad.center_x_m + feature.fractions[index] * recipe.span_m;
      additions.push({x, y: terrainHeight(baseCopy, x) + offsets[index]});
    }
  }
  scenario.world.terrain.points_m = [...basePoints, ...additions].sort((a, b) => a.x - b.x);
  const points = scenario.world.terrain.points_m;
  assert(points.every((point, index) => Number.isFinite(point.x) && Number.isFinite(point.y)
    && (!index || point.x > points[index - 1].x)), `${recipe.id}: duplicate or unordered terrain vertices`);
  assert(points.every(point => point.y >= terrainHeight(baseCopy, point.x) - 1e-10),
    `${recipe.id}: terrain was lowered below the tracked base`);
  for (const pad of [sourcePad, targetPad]) {
    assert.equal(terrainHeight(points, pad.center_x_m - pad.width_m / 2), pad.surface_y_m,
      `${recipe.id}: left pad shelf edge changed`);
    assert.equal(terrainHeight(points, pad.center_x_m + pad.width_m / 2), pad.surface_y_m,
      `${recipe.id}: right pad shelf edge changed`);
  }
  scenario.id = recipe.id;
  scenario.name = recipe.id;
  scenario.metadata = {
    ...(scenario.metadata ?? {}),
    research: 'waypoint_v2_fresh_terrain_readiness_v1',
    case_id: recipe.id,
    terrain_kind: recipe.family,
  };
  scenario.tags = [...new Set([...(scenario.tags ?? []), 'waypoint_v2_fresh_terrain_readiness'])];
  // serde serializes the optional route field explicitly as null; pin that
  // normalized route-free representation in the input fixture as well.
  scenario.mission.transfer_route = null;
  return {
    case_id: recipe.id,
    group: recipe.group,
    family: recipe.family,
    source_pad_id: base.source_pad_id,
    target_pad_id: base.target_pad_id,
    scenario,
    recipe_sha256: sha256(canonicalText(recipePayload(recipe))),
    scenario_sha256: sha256(canonicalText(scenario)),
  };
}

export function expandFreshInputs({root = ROOT, enforceSeals = true} = {}) {
  const fixturePath = resolve(root, FIXTURE_PATH);
  const fixtureBytes = readFileSync(fixturePath);
  const fixture = JSON.parse(fixtureBytes);
  assert.equal(fixture.schema_id, 'waypoint_v2_fresh_terrain_inputs_v1');
  assert.equal(fixture.schema_version, 1);
  assert.equal(fixture.status, 'frozen_before_mission_attempts');
  const originals = new Map();
  const sourceManifestHashes = {};
  for (const [name, manifest] of Object.entries(fixture.source_manifests)) {
    const bytes = readFileSync(resolve(root, manifest.path));
    const digest = sha256(bytes);
    sourceManifestHashes[name] = digest;
    assert.equal(digest, manifest.sha256, `${manifest.path}: source manifest digest changed`);
    for (const entry of JSON.parse(bytes).cases) {
      assert(!originals.has(entry.case_id), `duplicate tracked base ${entry.case_id}`);
      originals.set(entry.case_id, entry);
    }
  }
  assert.equal(fixture.cases.length, 12, 'fresh fixture must contain exactly twelve cases');
  assert.deepEqual(fixture.cases.map(entry => entry.id), [
    'fresh_clear_flat_805', 'fresh_clear_uphill_805', 'fresh_clear_downhill_805',
    'fresh_ridge_early_900', 'fresh_ridge_middle_900', 'fresh_ridge_late_900',
    'fresh_plateau_early_900', 'fresh_plateau_broad_900', 'fresh_plateau_late_900',
    'fresh_compound_successive_900', 'fresh_compound_uphill_ridge_805',
    'fresh_compound_downhill_plateau_805',
  ], 'fixed fresh case order changed');
  const ids = new Set();
  const counts = {clear: 0, terrain: 0};
  const families = {ridge: 0, plateau: 0, compound: 0};
  const cases = fixture.cases.map(recipe => {
    assert(!ids.has(recipe.id), `duplicate fresh case ${recipe.id}`);
    ids.add(recipe.id);
    counts[recipe.group]++;
    if (recipe.group === 'terrain') families[recipe.family]++;
    const compiled = compileRecipe(recipe, originals);
    if (enforceSeals) {
      assert.match(recipe.recipe_sha256, /^[0-9a-f]{64}$/, `${recipe.id}: recipe hash is not sealed`);
      assert.match(recipe.scenario_sha256, /^[0-9a-f]{64}$/, `${recipe.id}: scenario hash is not sealed`);
      assert.equal(compiled.recipe_sha256, recipe.recipe_sha256, `${recipe.id}: recipe hash mismatch`);
      assert.equal(compiled.scenario_sha256, recipe.scenario_sha256, `${recipe.id}: expanded scenario hash mismatch`);
    }
    return compiled;
  });
  assert.deepEqual(counts, {clear: 3, terrain: 9});
  assert.deepEqual(families, {ridge: 3, plateau: 3, compound: 3});
  const expanded = cases.map(({case_id, source_pad_id, target_pad_id, scenario}) => ({
    case_id, source_pad_id, target_pad_id, scenario,
  }));
  const expandedScenariosSha256 = sha256(canonicalText(expanded));
  if (enforceSeals) {
    assert.match(fixture.expanded_scenarios_sha256, /^[0-9a-f]{64}$/,
      'expanded scenario set has no reviewed digest');
    assert.equal(expandedScenariosSha256, fixture.expanded_scenarios_sha256,
      'expanded scenario set differs from its reviewed digest');
  }
  return {
    fixture,
    fixtureSha256: sha256(fixtureBytes),
    sourceManifestHashes,
    expandedScenariosSha256,
    cases,
    counts,
  };
}

function sourceTreeSha256(root = ROOT) {
  const roots = ['pd-core', 'pd-control', 'pd-plan', 'pd-eval', 'pd-report'];
  const paths = ['Cargo.toml', 'Cargo.lock'];
  for (const crate of roots) {
    paths.push(`${crate}/Cargo.toml`);
    const visit = relative => {
      for (const entry of readdirSync(resolve(root, relative), {withFileTypes: true})) {
        const child = `${relative}/${entry.name}`;
        if (entry.isDirectory()) visit(child);
        else if (entry.isFile() && child.endsWith('.rs')) paths.push(child);
      }
    };
    visit(`${crate}/src`);
  }
  const hash = createHash('sha256');
  for (const path of paths.sort()) {
    hash.update(path).update('\0').update(readFileSync(resolve(root, path))).update('\0');
  }
  return hash.digest('hex');
}

function currentProvenance(bin, expanded, root = ROOT) {
  return {
    cli_binary_sha256: sha256(readFileSync(bin)),
    source_tree_sha256: sourceTreeSha256(root),
    runner_sha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
    fixture_sha256: sha256(readFileSync(resolve(root, FIXTURE_PATH))),
    source_manifest_sha256: Object.fromEntries(Object.entries(expanded.fixture.source_manifests)
      .map(([name, source]) => [name, sha256(readFileSync(resolve(root, source.path)))])),
    expanded_scenarios_sha256: expanded.expandedScenariosSha256,
  };
}

function fileExistsIncludingDanglingSymlink(path) {
  try {
    lstatSync(path);
    return true;
  } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
}

function validateCliPath(bin) {
  assert(existsSync(bin), `CLI binary does not exist: ${bin}`);
  assert(statSync(bin).isFile(), `CLI binary is not a file: ${bin}`);
  accessSync(bin, fsConstants.X_OK);
}

function createOutputRoot(path) {
  assert(!fileExistsIncludingDanglingSymlink(path), `output root already exists (create-only): ${path}`);
  mkdirSync(path);
  const inputDir = join(path, 'inputs');
  const runDir = join(path, 'runs');
  const preflightDir = join(path, 'preflight');
  mkdirSync(inputDir);
  mkdirSync(runDir);
  mkdirSync(preflightDir);
  return {inputDir, runDir, preflightDir};
}

function atomicWrite(path, value) {
  const temporary = `${path}.tmp`;
  writeFileSync(temporary, jsonText(value));
  renameSync(temporary, path);
}

function pointerSegments(pointer) {
  return pointer.split('/').slice(1).map(part => part.replaceAll('~1', '/').replaceAll('~0', '~'));
}

function removePointer(value, pointer) {
  const segments = pointerSegments(pointer);
  function removeAt(node, index) {
    if (node === null || typeof node !== 'object') return;
    const key = segments[index];
    if (key === '*') {
      for (const child of Object.values(node)) removeAt(child, index + 1);
    } else if (index === segments.length - 1) {
      if (Array.isArray(node) && /^\d+$/.test(key)) delete node[Number(key)];
      else if (isObject(node)) delete node[key];
    } else if (Object.hasOwn(node, key)) {
      removeAt(node[key], index + 1);
    }
  }
  removeAt(value, 0);
}

function comparable(value, ignoredPointers) {
  const copy = structuredClone(value);
  for (const pointer of ignoredPointers) removePointer(copy, pointer);
  return copy;
}

export function firstDifference(left, right, pointer = '') {
  if (Object.is(left, right)) return null;
  if (Array.isArray(left) && Array.isArray(right)) {
    if (left.length !== right.length) return `${pointer}/length`;
    for (let index = 0; index < left.length; index++) {
      const difference = firstDifference(left[index], right[index], `${pointer}/${index}`);
      if (difference) return difference;
    }
    return null;
  }
  if (isObject(left) && isObject(right)) {
    const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
    for (const key of keys) {
      if (!Object.hasOwn(left, key) || !Object.hasOwn(right, key)) return `${pointer}/${key}`;
      const difference = firstDifference(left[key], right[key], `${pointer}/${key}`);
      if (difference) return difference;
    }
    return null;
  }
  return pointer || '/';
}

function nearestRank(values, fraction) {
  return [...values].sort((a, b) => a - b)[Math.ceil(fraction * values.length) - 1];
}

function isLanded(result) {
  return result.physical_outcome === 'landed_on_target' && result.mission_outcome === 'success';
}

function assertStateClock(state, context) {
  assert(isObject(state), `${context}: missing state`);
  assert(Number.isSafeInteger(state.physics_step) && state.physics_step >= 0,
    `${context}: invalid physics step`);
  assert(Number.isFinite(state.sim_time_s) && Math.abs(state.sim_time_s - state.physics_step / PHYSICS_HZ) < 1e-9,
    `${context}: simulation clock does not match original physics clock`);
}

export function validateRunEvidence(caseInfo, summaryPath, flightPath, scenarioPath, inputPath, stdout, exitCode) {
  const id = caseInfo.case_id;
  assert(statSync(scenarioPath).isFile(), `${id}: CLI did not preserve scenario.json`);
  assert(deepEqual(readJson(scenarioPath), persistedScenario(readJson(inputPath))),
    `${id}: saved scenario differs from sealed input`);
  const summary = readJson(summaryPath);
  const flight = readJson(flightPath);
  assert.equal(summary.schema_id, 'waypoint_v2_flight_summary_v1', `${id}: summary schema`);
  assert.equal(summary.policy?.policy_id, POLICY_ID, `${id}: policy version is not frozen policy 3`);
  assert.equal(flight.policy?.policy_id, POLICY_ID, `${id}: full flight policy mismatch`);
  const frozenPolicy = {policy_id: POLICY_ID, maximum_corrections: 6};
  assert.deepEqual(summary.policy, frozenPolicy, `${id}: compact policy payload changed`);
  assert.deepEqual(flight.policy, frozenPolicy, `${id}: full-flight policy payload changed`);
  assert.equal(summary.input_identity, flight.input_identity, `${id}: input identity mismatch`);
  const result = summary.result;
  assert(isObject(result), `${id}: summary result missing`);
  for (const field of ['planning_stop', 'reason', 'correction_count', 'initial_nominal_terrain_blocked',
    'integrity_passed', 'physical_outcome', 'mission_outcome', 'final_source_replay_passed']) {
    assert.deepEqual(flight[field], result[field], `${id}: summary/full-flight mismatch at ${field}`);
  }
  assert(PLANNING_STOPS.has(result.planning_stop), `${id}: unknown planning stop ${result.planning_stop}`);
  assert(Number.isSafeInteger(result.correction_count) && result.correction_count >= 0,
    `${id}: invalid correction count`);
  assert.equal(typeof result.initial_nominal_terrain_blocked, 'boolean', `${id}: blocked flag`);
  assert(PHYSICAL_OUTCOMES.has(result.physical_outcome), `${id}: unknown physical outcome`);
  assert(MISSION_OUTCOMES.has(result.mission_outcome), `${id}: unknown mission outcome`);
  assert.deepEqual(flight.timings, result.timings, `${id}: result timing mismatch`);
  for (const key of ['planning_s', 'execution_s', 'replay_s']) {
    assert(Number.isFinite(result.timings?.[key]) && result.timings[key] >= 0,
      `${id}: invalid result timing ${key}`);
  }
  assert(Number.isFinite(summary.timings?.output_s) && summary.timings.output_s >= 0,
    `${id}: invalid output timing`);
  assert(Number.isFinite(summary.timings?.total_s) && summary.timings.total_s >= 0,
    `${id}: invalid total timing`);
  assert(isObject(summary.run_summary), `${id}: supported attempt lacks run summary`);
  assert(isObject(flight.manifest), `${id}: supported attempt lacks final manifest`);
  assert(isObject(flight.ordinary_flight), `${id}: supported attempt lacks ordinary flight evidence`);
  assert(isObject(flight.ordinary_flight.final_state), `${id}: ordinary final state missing`);
  assert(isObject(summary.run_summary.minimum_clearance), `${id}: minimum-clearance evidence missing`);
  for (const field of ['touchdown_m', 'hull_m']) {
    assert(Number.isFinite(summary.run_summary.minimum_clearance[field]),
      `${id}: invalid minimum clearance ${field}`);
  }
  assert.equal(result.integrity_passed, true, `${id}: runtime integrity failed`);
  assert.equal(result.final_source_replay_passed, true, `${id}: final-source replay failed`);
  assert(['unsupported', 'invalid_input', 'implementation_error'].every(stop => result.planning_stop !== stop),
    `${id}: run ended with an input or implementation error`);
  assert(statSync(join(dirname(summaryPath), 'report.html')).isFile(), `${id}: rich report is missing`);

  for (const [field, summaryKey, manifestKey] of [
    ['physics_step', 'endpoint', 'physics_steps'],
    ['physical_outcome', 'endpoint', 'physical_outcome'],
    ['mission_outcome', 'endpoint', 'mission_outcome'],
    ['end_reason', 'endpoint', 'end_reason'],
  ]) {
    const endpoint = summary.run_summary[summaryKey];
    assert.deepEqual(endpoint?.[field], flight.manifest[manifestKey], `${id}: manifest endpoint mismatch at ${field}`);
  }
  const runEndpoint = summary.run_summary.endpoint;
  const manifest = flight.manifest;
  const manifestSummary = manifest.summary;
  const inputScenario = readJson(inputPath);
  assert.equal(manifest.scenario_id, inputScenario.id, `${id}: final manifest scenario ID mismatch`);
  assert.equal(manifest.physics_hz, PHYSICS_HZ, `${id}: physics clock changed`);
  assert.equal(manifest.controller_hz, CONTROLLER_HZ, `${id}: command clock changed`);
  assert.equal(manifest.physics_steps, runEndpoint.physics_step, `${id}: endpoint step mismatch`);
  assert(Math.abs(manifest.sim_time_s - runEndpoint.sim_time_s) < 1e-9,
    `${id}: endpoint time mismatch`);
  assert.equal(manifest.physical_outcome, result.physical_outcome, `${id}: manifest outcome mismatch`);
  assert.equal(manifest.mission_outcome, result.mission_outcome, `${id}: manifest mission mismatch`);
  assert.equal(manifest.end_reason, flight.ordinary_flight.final_state.end_reason,
    `${id}: ordinary end reason mismatch`);
  assert(Array.isArray(flight.segments), `${id}: segments missing`);
  if (flight.segments.length > 0) {
    assert.deepEqual(flight.ordinary_flight.final_state, flight.segments.at(-1).end_state,
      `${id}: last executed segment differs from ordinary final state`);
  } else {
    assert.equal(flight.ordinary_flight.final_state.physics_step, 0,
      `${id}: unadvanced ordinary flight has a nonzero endpoint`);
  }
  assert.equal(flight.ordinary_flight.final_state.physics_step, manifest.physics_steps,
    `${id}: final state step differs from manifest`);
  assert.equal(flight.ordinary_flight.final_state.physical_outcome, result.physical_outcome,
    `${id}: final state physical outcome mismatch`);
  assert.equal(flight.ordinary_flight.final_state.mission_outcome, result.mission_outcome,
    `${id}: final state mission outcome mismatch`);
  assert.deepEqual(summary.run_summary.fuel, {
    remaining_kg: manifestSummary.fuel_remaining_kg,
    used_kg: manifestSummary.fuel_used_kg,
  }, `${id}: fuel summary differs from manifest`);
  assert.deepEqual(summary.run_summary.minimum_clearance, {
    touchdown_m: manifestSummary.min_touchdown_clearance_m,
    hull_m: manifestSummary.min_hull_clearance_m,
    landing: manifestSummary.landing,
  }, `${id}: minimum-clearance summary differs from manifest`);
  assert(Number.isFinite(manifestSummary.fuel_remaining_kg)
    && Number.isFinite(manifestSummary.fuel_used_kg), `${id}: manifest fuel evidence is invalid`);
  assert(manifestSummary.fuel_used_kg >= 0, `${id}: manifest reports negative fuel use`);
  const initialFuelKg = inputScenario.vehicle.initial_fuel_kg;
  assert(Number.isFinite(initialFuelKg) && initialFuelKg > 0, `${id}: input fuel is invalid`);
  assert(Math.abs(manifestSummary.fuel_remaining_kg + manifestSummary.fuel_used_kg - initialFuelKg)
    <= Math.max(1e-9, initialFuelKg * 1e-9), `${id}: fuel use does not reconcile to the sealed input`);
  assert(Number.isFinite(summary.run_summary.fuel.remaining_kg)
    && summary.run_summary.fuel.remaining_kg > 0, `${id}: fuel was exhausted`);
  assert(Number.isFinite(flight.ordinary_flight.final_state.fuel_kg)
    && flight.ordinary_flight.final_state.fuel_kg > 0, `${id}: final state fuel was exhausted`);
  assert.equal(flight.ordinary_flight.final_state.fuel_kg, manifestSummary.fuel_remaining_kg,
    `${id}: final state fuel differs from manifest`);
  assert.notEqual(result.physical_outcome, 'crashed', `${id}: physical crash is a hard failure`);
  assert.notEqual(result.physical_outcome, 'landed_off_target', `${id}: off-target landing is a hard failure`);
  assert.notEqual(result.physical_outcome, 'timed_out', `${id}: physical horizon timeout is a hard failure`);
  assert.notEqual(result.mission_outcome, 'failed_crash', `${id}: crash is a hard failure`);
  assert.notEqual(result.mission_outcome, 'failed_timeout', `${id}: mission timeout is a hard failure`);
  assert.notEqual(result.mission_outcome, 'failed_checkpoint', `${id}: checkpoint failure is a hard failure`);
  if (result.planning_stop === 'landed' || result.physical_outcome === 'landed_on_target') {
    assert.equal(result.planning_stop, 'landed', `${id}: landing outcome contradicts planning stop`);
    assert.equal(result.physical_outcome, 'landed_on_target', `${id}: landing stop without target touchdown`);
    assert.equal(result.mission_outcome, 'success', `${id}: target touchdown without mission success`);
  }

  const cycles = flight.cycles;
  const segments = flight.segments;
  assert(Array.isArray(cycles) && cycles.length > 0, `${id}: cycles missing`);
  assert(Array.isArray(segments), `${id}: segments missing`);
  const actualBoundaryStates = [flight.ordinary_flight.final_state];
  for (const segment of segments) actualBoundaryStates.push(segment.entry_state, segment.end_state);
  const selectedCycles = [];
  for (const [index, cycle] of cycles.entries()) {
    assertStateClock(cycle.current_state, `${id}: cycle ${index} current state`);
    assert.equal(cycle.cycle_index, index, `${id}: cycle index discontinuity`);
    assert(actualBoundaryStates.some(state => deepEqual(state, cycle.current_state)),
      `${id}: cycle ${index} current state does not bind an actual segment boundary or final state`);
    if (cycle.fixed_consumed_prefix_proven !== true) {
      assert.equal(cycle.decision, 'no_nominal', `${id}: executed nominal prefix lacks replay proof`);
    }
    if (cycle.decision === 'direct') {
      assert(isObject(cycle.audit), `${id}: direct cycle lacks audit`);
      assert.equal(cycle.audit.passed, true, `${id}: selected direct audit failed`);
      assert.equal(cycle.audit.commands_match, true, `${id}: selected direct audit command mismatch`);
      assert.equal(cycle.audit.safe_target_contact, true, `${id}: direct audit lacks safe target contact`);
      assert.equal(cycle.audit.ordinary_neutral_parity, true, `${id}: direct audit lacks ordinary replay parity`);
      assert.equal(cycle.audit.clearance_scan?.all_airborne_states_passed, true,
        `${id}: direct audit phase-aware clearance scan failed`);
      const directSegment = segments.find(segment => segment.start_physics_step === cycle.current_state.physics_step
        && segment.kind !== 'local_correction');
      assert(directSegment, `${id}: direct cycle has no executed nominal segment`);
      assert.deepEqual(cycle.audit.final_state, directSegment.end_state,
        `${id}: direct audit state differs from executed segment`);
    }
    const local = cycle.local_search;
    if (local?.selected !== null && local?.selected !== undefined) {
      selectedCycles.push(cycle);
      assert.equal(local.handoff_source_replay_passed, true,
        `${id}: selected local handoff lacks source replay`);
      assert.equal(local.certificate_source_replay_passed, true,
        `${id}: selected continuation certificate lacks source replay`);
      const selected = local.selected;
      assertStateClock(selected.entry_state, `${id}: selected handoff entry`);
      assertStateClock(selected.handoff_state, `${id}: selected handoff state`);
      assertStateClock(local.certificate_state, `${id}: selected continuation certificate`);
      assert(deepEqual(local.certificate_state, selected.continuation_end_state),
        `${id}: selected certificate and continuation endpoint differ`);
      assert(local.certificate_state.physics_step > selected.handoff_state.physics_step,
        `${id}: continuation certificate does not advance the original clock`);
      assert(Array.isArray(selected.trajectory) && selected.trajectory.length > 0,
        `${id}: selected local clearance trajectory is missing`);
      const reserve = selected.policy?.minimum_clearance_m;
      assert.equal(reserve, 5, `${id}: selected local reserve differs from frozen policy`);
      for (const [pointIndex, point] of selected.trajectory.entries()) {
        assertStateClock(point.state, `${id}: selected local trajectory point ${pointIndex}`);
        assert(Number.isFinite(point.body_clearance_m) && point.body_clearance_m >= reserve,
          `${id}: selected local trajectory violates its recorded reserve`);
      }
      assert(deepEqual(selected.trajectory[0].state, selected.entry_state),
        `${id}: local trajectory does not begin at its selected entry state`);
      assert(deepEqual(selected.trajectory.at(-1).state, local.certificate_state),
        `${id}: local trajectory does not end at its replayed certificate`);
      assert(selected.trajectory.some(point => deepEqual(point.state, selected.handoff_state)),
        `${id}: local trajectory omits the selected actual handoff state`);
      const localSegment = segments.find(segment => segment.kind === 'local_correction'
        && segment.start_physics_step === selected.entry_state.physics_step);
      assert(localSegment, `${id}: selected local candidate has no executed local segment`);
      assert(deepEqual(localSegment.entry_state, selected.entry_state),
        `${id}: local segment entry differs from selected proposal`);
      assert(deepEqual(localSegment.end_state, selected.handoff_state),
        `${id}: actual local handoff differs from selected proposal`);
    }
  }
  assert.equal(selectedCycles.length, result.correction_count,
    `${id}: correction count differs from selected local handoffs`);

  for (const [index, segment] of segments.entries()) {
    assert(['initial_nominal', 'airborne_nominal', 'local_correction'].includes(segment.kind),
      `${id}: unknown executed segment kind ${segment.kind}`);
    assertStateClock(segment.entry_state, `${id}: segment ${index} entry`);
    assertStateClock(segment.end_state, `${id}: segment ${index} end`);
    assert.equal(segment.start_physics_step, segment.entry_state.physics_step,
      `${id}: segment ${index} start tick mismatch`);
    assert.equal(segment.end_physics_step, segment.end_state.physics_step,
      `${id}: segment ${index} end tick mismatch`);
    assert(segment.end_physics_step > segment.start_physics_step,
      `${id}: segment ${index} does not advance`);
    assert(Array.isArray(segment.updates) && segment.updates.length > 0,
      `${id}: segment ${index} has no commands`);
    const span = segment.end_physics_step - segment.start_physics_step;
    const normalSpan = segment.updates.length * 2;
    const oddTerminalContact = index === segments.length - 1
      && span === normalSpan - 1
      && ['landed_on_target', 'landed_off_target', 'crashed'].includes(result.physical_outcome);
    assert(span === normalSpan || oddTerminalContact,
      `${id}: segment ${index} command coverage has a gap`);
    for (const [updateIndex, update] of segment.updates.entries()) {
      assert.equal(update.physics_step, segment.start_physics_step + updateIndex * 2,
        `${id}: segment ${index} command clock discontinuity`);
      assert(isObject(update.command) && Number.isFinite(update.command.throttle_frac)
        && Number.isFinite(update.command.target_attitude_rad), `${id}: malformed command`);
    }
    if (index > 0) {
      assert.equal(segments[index - 1].end_physics_step, segment.start_physics_step,
        `${id}: executed segments reset or skip the original clock`);
      assert.deepEqual(segments[index - 1].end_state, segment.entry_state,
        `${id}: actual segment handoff state is discontinuous`);
    }
    if (segment.kind === 'initial_nominal' || segment.kind === 'airborne_nominal') {
      const cycle = cycles.find(item => deepEqual(item.current_state, segment.entry_state));
      assert(cycle, `${id}: nominal segment ${index} has no matching planning cycle`);
      assert(Array.isArray(cycle.nominal_updates), `${id}: nominal cycle lacks its fixed program`);
      assert(segment.updates.length <= cycle.nominal_updates.length,
        `${id}: executed nominal segment is longer than its selected program`);
      assert.deepEqual(segment.updates, cycle.nominal_updates.slice(0, segment.updates.length),
        `${id}: executed nominal segment is not a selected-program prefix`);
      if (cycle.decision === 'direct') {
        assert.deepEqual(segment.updates, cycle.nominal_updates,
          `${id}: direct nominal segment does not consume its full selected program`);
      } else {
        assert.equal(cycle.fixed_consumed_prefix_proven, true,
          `${id}: partial nominal segment lacks consumed-prefix replay proof`);
      }
    }
  }
  const updates = segments.flatMap(segment => segment.updates);
  const actions = flight.ordinary_flight.actions;
  assert(Array.isArray(actions), `${id}: ordinary action trace missing`);
  assert.equal(manifest.controller_updates, actions.length, `${id}: manifest command count differs from actual actions`);
  assert.equal(actions.length, updates.length, `${id}: action trace does not cover executed commands`);
  for (const [index, action] of actions.entries()) {
    const update = updates[index];
    assert.equal(action.controller_update_index, index, `${id}: controller update index reset`);
    assert.equal(action.physics_step, update.physics_step, `${id}: action/update clock mismatch`);
    assert(Math.abs(action.sim_time_s - action.physics_step / PHYSICS_HZ) < 1e-9,
      `${id}: action clock is not mission-relative`);
    assert.deepEqual(action.command, update.command, `${id}: actual command differs from executed segment`);
  }

  if (caseInfo.group === 'clear' && isLanded(result) && result.correction_count === 0) {
    assert.equal(cycles.length, 1, `${id}: clear control must have one cycle`);
    assert.equal(segments.length, 1, `${id}: clear control must have one nominal segment`);
    const cycle = cycles[0];
    const segment = segments[0];
    assert.equal(cycle.decision, 'direct', `${id}: clear control was not direct`);
    assert.equal(segment.kind, 'initial_nominal', `${id}: clear control segment kind`);
    assert.equal(cycle.audit?.passed, true, `${id}: clear fixed audit failed`);
    assert.equal(cycle.audit?.commands_match, true, `${id}: clear audit did not match nominal commands`);
    assert.deepEqual(segment.updates, cycle.nominal_updates,
      `${id}: clear consumed commands differ from selected direct program`);
  }

  let cli;
  try {
    cli = JSON.parse(stdout);
  } catch {
    throw new Error(`${id}: CLI stdout is not compact result JSON`);
  }
  assert.equal(cli.policy_version, 3, `${id}: CLI reported wrong policy version`);
  for (const field of ['planning_stop', 'reason', 'correction_count', 'initial_nominal_terrain_blocked',
    'integrity_passed', 'physical_outcome', 'mission_outcome', 'final_source_replay_passed']) {
    assert.deepEqual(cli[field], result[field], `${id}: CLI/summary mismatch at ${field}`);
  }
  assert.deepEqual(cli.timings, result.timings, `${id}: CLI/result timing mismatch`);
  assert.equal(resolve(cli.output_dir), dirname(summaryPath), `${id}: CLI output path mismatch`);
  assert.equal(exitCode, 0, `${id}: CLI reported typed input or integrity failure`);

  return {
    planning_stop: result.planning_stop,
    reason: result.reason,
    correction_count: result.correction_count,
    initial_nominal_terrain_blocked: result.initial_nominal_terrain_blocked,
    integrity_passed: result.integrity_passed,
    final_source_replay_passed: result.final_source_replay_passed,
    physical_outcome: result.physical_outcome,
    mission_outcome: result.mission_outcome,
    planning_s: result.timings.planning_s,
    run_summary: summary.run_summary,
    summary,
    flight,
  };
}

export function evaluateFreshAcceptance(records, completed) {
  if (!completed || records.length !== 12) {
    return {evaluated: false, passed: null, reason: 'all twelve fresh attempts did not complete'};
  }
  const failures = [];
  const clear = records.filter(record => record.group === 'clear');
  const terrain = records.filter(record => record.group === 'terrain');
  const clearLanded = clear.filter(record => isLanded(record.result));
  if (clear.length !== 3 || clearLanded.length !== 3
    || clear.some(record => record.result.correction_count !== 0)) {
    failures.push('all three clear controls must land directly with zero corrections');
  }
  const terrainLanded = terrain.filter(record => isLanded(record.result));
  const familyLandings = {};
  for (const family of ['ridge', 'plateau', 'compound']) {
    familyLandings[family] = terrain.filter(record => record.family === family && isLanded(record.result)).length;
    if (familyLandings[family] < 2) failures.push(`${family} landing floor is 2 of 3`);
  }
  if (terrainLanded.length < 7) failures.push('terrain landing floor is 7 of 9');
  const blocked = terrain.filter(record => record.result.initial_nominal_terrain_blocked).length;
  if (blocked < 6) failures.push('at least 6 of 9 terrain cases must block the initial nominal');
  const planning = records.map(record => record.result.planning_s).sort((a, b) => a - b);
  const middle = planning.length / 2;
  const median = (planning[middle - 1] + planning[middle]) / 2;
  const p95 = nearestRank(planning, 0.95);
  if (median > 2) failures.push('planning median exceeds 2 seconds');
  if (p95 > 5) failures.push('nearest-rank planning p95 exceeds 5 seconds');
  return {
    evaluated: true,
    passed: failures.length === 0,
    failures,
    clear: {
      landed: clearLanded.length,
      required: 3,
      zero_correction_controls: clearLanded.length === 3
        && clear.every(record => record.result.correction_count === 0),
    },
    terrain: {
      landed: terrainLanded.length,
      required: 7,
      family_landings: familyLandings,
      initially_blocked: blocked,
      initially_blocked_required: 6,
    },
    planning_performance: {
      sample_count: planning.length,
      median_s: median,
      p95_nearest_rank_s: p95,
      median_limit_s: 2,
      p95_limit_s: 5,
    },
  };
}

function runCli(bin, scenarioPath, sourcePadId, targetPadId, outputDir, preflightOnly = false) {
  const args = [
    'waypoint-v2-flight', '--policy-version', '3',
    '--scenario', scenarioPath,
    '--source-pad-id', sourcePadId,
    '--target-pad-id', targetPadId,
  ];
  if (preflightOnly) args.push('--preflight-only');
  else args.push('--output-dir', outputDir);
  return spawnSync(bin, args, {
    cwd: ROOT,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
  });
}

function parsePreflight(child, id) {
  assert(!child.error, `${id}: failed to invoke preflight CLI: ${child.error?.message}`);
  assert.equal(child.signal, null, `${id}: preflight CLI was signalled`);
  let parsed;
  try {
    parsed = JSON.parse(child.stdout);
  } catch {
    throw new Error(`${id}: preflight did not emit JSON`);
  }
  assert.equal(child.status, 0, `${id}: preflight CLI rejected the frozen input: ${child.stderr}`);
  assert.equal(parsed.supported, true, `${id}: input is unsupported`);
  assert.equal(parsed.simulation_created, false, `${id}: preflight created a simulation`);
  assert.equal(parsed.rejection, null, `${id}: preflight rejection is non-null`);
  return parsed;
}

function compactIdentity(expanded) {
  return {
    fixture_sha256: expanded.fixtureSha256,
    source_manifest_sha256: expanded.sourceManifestHashes,
    expanded_scenarios_sha256: expanded.expandedScenariosSha256,
    case_scenario_sha256: Object.fromEntries(expanded.cases.map(entry => [entry.case_id, entry.scenario_sha256])),
  };
}

function baselineMetadata(baselineRoot, fixture) {
  const summaryPath = join(baselineRoot, SUMMARY_NAME);
  const suite = readJson(summaryPath);
  assert.equal(suite.schema_id, 'waypoint_v2_practical_suite_run_v1', `${summaryPath}: baseline schema`);
  assert.equal(suite.status, 'completed', `${summaryPath}: accepted baseline is incomplete`);
  assert.equal(suite.policy_version, 3, `${summaryPath}: baseline policy version`);
  assert.equal(suite.provenance?.unchanged, true, `${summaryPath}: historical provenance is not sealed`);
  for (const sentinel of fixture.sentinels) {
    const record = suite.cases.find(item => item.case_id === sentinel.id);
    assert(record, `${summaryPath}: missing sentinel ${sentinel.id}`);
    assert.equal(record.result?.planning_stop, 'landed', `${sentinel.id}: historical stop`);
    assert.equal(record.result?.physical_outcome, sentinel.physical_outcome, `${sentinel.id}: historical physical outcome`);
    assert.equal(record.result?.mission_outcome, sentinel.mission_outcome, `${sentinel.id}: historical mission outcome`);
    assert.equal(record.result?.correction_count, sentinel.correction_count, `${sentinel.id}: historical corrections`);
    const runDir = join(baselineRoot, 'runs', sentinel.id);
    const scenarioPath = join(runDir, 'scenario.json');
    const flightPath = join(runDir, 'flight.json');
    const caseSummaryPath = join(runDir, 'summary.json');
    for (const path of [scenarioPath, flightPath, caseSummaryPath, join(runDir, 'report.html')]) {
      assert(statSync(path).isFile(), `${sentinel.id}: historical capture missing ${path}`);
    }
    assert(deepEqual(persistedScenario(readJson(join(baselineRoot, 'inputs', `${sentinel.id}.json`))),
      readJson(scenarioPath)),
      `${sentinel.id}: baseline input differs from accepted capture scenario`);
  }
  return {
    summary_path: resolve(summaryPath),
    suite_sha256: sha256(readFileSync(summaryPath)),
    suite_provenance: suite.provenance,
    source_provenance_is_historical: true,
  };
}

function compareSentinelToBaseline(id, currentDir, baselineRoot) {
  const baselineDir = join(baselineRoot, 'runs', id);
  assert(deepEqual(readJson(join(currentDir, 'scenario.json')), readJson(join(baselineDir, 'scenario.json'))),
    `${id}: sentinel scenario changed`);
  for (const name of ['summary.json', 'flight.json']) {
    const current = readJson(join(currentDir, name));
    const baseline = readJson(join(baselineDir, name));
    const ignored = name === 'summary.json' ? SUMMARY_IGNORED_POINTERS : FLIGHT_IGNORED_POINTERS;
    const difference = firstDifference(comparable(current, ignored), comparable(baseline, ignored));
    assert.equal(difference, null, `${id}/${name}: historical physical result differs at ${difference}`);
  }
  return {case_id: id, preserved: true, compared: ['scenario.json', 'summary.json', 'flight.json']};
}

function normalizeBaselineScenario(baselineRoot, id) {
  const path = join(baselineRoot, 'inputs', `${id}.json`);
  const scenario = persistedScenario(readJson(path));
  assert.equal(scenario.id, id, `${id}: baseline scenario ID mismatch`);
  const source = scenario.world.landing_pads.find(pad => pad.id === 'pad_source');
  const targetId = scenario.mission.goal.target_pad_id;
  const target = scenario.world.landing_pads.find(pad => pad.id === targetId);
  assert(source && target, `${id}: baseline source or target pad is missing`);
  assert.equal(scenario.mission.transfer_route ?? null, null,
    `${id}: baseline sentinel is no longer route-free`);
  return {scenario, source_pad_id: source.id, target_pad_id: target.id};
}

export function runSuite(options, expanded = expandFreshInputs()) {
  validateCliPath(options.bin);
  const baseline = options.mode === 'sentinels'
    ? baselineMetadata(options.baselineRoot, expanded.fixture)
    : null;
  const provenanceBefore = currentProvenance(options.bin, expanded);
  assert.deepEqual(provenanceBefore.source_manifest_sha256, expanded.sourceManifestHashes,
    'source manifests changed between expansion and provenance capture');
  assert.equal(provenanceBefore.fixture_sha256, expanded.fixtureSha256,
    'fixture changed between expansion and provenance capture');
  const dirs = createOutputRoot(options.outputDir);
  const inputCases = options.mode === 'sentinels'
    ? expanded.fixture.sentinels.map(sentinel => ({
      case_id: sentinel.id,
      group: 'sentinel',
      family: 'sentinel',
      ...normalizeBaselineScenario(options.baselineRoot, sentinel.id),
      sentinel,
    }))
    : expanded.cases;
  const summary = {
    schema_id: 'waypoint_v2_fresh_terrain_readiness_run_v1',
    mode: options.mode,
    status: options.mode === 'preflight-only' ? 'preflighting' : 'preflighting',
    fixture_sha256: expanded.fixtureSha256,
    source_manifest_sha256: expanded.sourceManifestHashes,
    expanded_scenarios_sha256: expanded.expandedScenariosSha256,
    input_identity: compactIdentity(expanded),
    policy_version: 3,
    case_count: inputCases.length,
    provenance: {before: provenanceBefore, after: null, unchanged: null},
    historical_baseline_provenance: baseline,
    preflight: [],
    cases: [],
    acceptance: {evaluated: false, passed: null, reason: 'preflight is running'},
  };
  const summaryPath = join(options.outputDir, SUMMARY_NAME);
  const saveProgress = () => atomicWrite(summaryPath, summary);
  saveProgress();
  let inputFailure = null;

  for (const entry of inputCases) {
    const inputPath = join(dirs.inputDir, `${entry.case_id}.json`);
    writeFileSync(inputPath, jsonText(entry.scenario), {flag: 'wx'});
    const preflightPath = join(dirs.preflightDir, `${entry.case_id}.json`);
    const child = runCli(options.bin, inputPath, entry.source_pad_id, entry.target_pad_id, null, true);
    const item = {case_id: entry.case_id, cli_exit_code: child.status, cli_signal: child.signal};
    try {
      const preflight = parsePreflight(child, entry.case_id);
      writeFileSync(preflightPath, jsonText(preflight), {flag: 'wx'});
      item.supported = preflight.supported;
      item.simulation_created = preflight.simulation_created;
    } catch (error) {
      item.error = error.message;
      inputFailure ??= `${entry.case_id}: ${error.message}`;
      if (child.stdout) writeFileSync(preflightPath, child.stdout, {flag: 'wx'});
    }
    summary.preflight.push(item);
    summary.acceptance = {evaluated: false, passed: null, reason: 'preflight is running'};
    saveProgress();
  }
  if (options.mode === 'preflight-only') {
    summary.status = inputFailure ? 'preflight_failed' : 'preflight_passed';
    summary.acceptance = {evaluated: false, passed: null,
      reason: inputFailure ?? 'non-simulation preflight completed; mission acceptance was not evaluated'};
  } else if (inputFailure) {
    summary.status = 'stopped';
    summary.stop_reason = `preflight failed before missions: ${inputFailure}`;
  } else {
    for (const entry of inputCases) {
      const id = entry.case_id;
      const inputPath = join(dirs.inputDir, `${id}.json`);
      const caseDir = join(dirs.runDir, id);
      summary.phase = 'running';
      summary.current_case_id = id;
      summary.acceptance = {evaluated: false, passed: null, reason: 'mission attempts are running'};
      saveProgress();
      const child = runCli(options.bin, inputPath, entry.source_pad_id, entry.target_pad_id, caseDir, false);
      const record = {
        case_id: id,
        group: entry.group,
        family: entry.family,
        cli_exit_code: child.status,
        cli_signal: child.signal,
        summary_path: `runs/${id}/summary.json`,
        flight_path: `runs/${id}/flight.json`,
      };
      try {
        assert(!child.error, `${id}: failed to invoke flight CLI: ${child.error?.message}`);
        assert.equal(child.signal, null, `${id}: flight CLI was signalled`);
        const result = validateRunEvidence(
          entry,
          join(caseDir, 'summary.json'),
          join(caseDir, 'flight.json'),
          join(caseDir, 'scenario.json'),
          inputPath,
          child.stdout,
          child.status,
        );
        record.result = {
          planning_stop: result.planning_stop,
          reason: result.reason,
          correction_count: result.correction_count,
          initial_nominal_terrain_blocked: result.initial_nominal_terrain_blocked,
          integrity_passed: result.integrity_passed,
          final_source_replay_passed: result.final_source_replay_passed,
          physical_outcome: result.physical_outcome,
          mission_outcome: result.mission_outcome,
          planning_s: result.planning_s,
        };
        if (entry.group === 'sentinel') {
          assert.equal(result.planning_stop, 'landed', `${id}: sentinel did not land`);
          assert.equal(result.physical_outcome, entry.sentinel.physical_outcome, `${id}: physical sentinel outcome`);
          assert.equal(result.mission_outcome, entry.sentinel.mission_outcome, `${id}: mission sentinel outcome`);
          assert.equal(result.correction_count, entry.sentinel.correction_count, `${id}: sentinel corrections`);
          record.sentinel_comparison = compareSentinelToBaseline(id, caseDir, options.baselineRoot);
        }
        summary.cases.push(record);
        if (!isLanded(record.result)) {
          record.coverage_miss = true;
        }
        summary.acceptance = options.mode === 'fresh'
          ? evaluateFreshAcceptance(summary.cases, false)
          : {evaluated: false, passed: null, reason: 'sentinels are preservation checks outside the fresh denominator'};
        saveProgress();
      } catch (error) {
        record.error = error.message;
        summary.cases.push(record);
        summary.status = 'stopped';
        summary.stop_reason = `${id}: ${error.message}`;
        summary.acceptance = {evaluated: false, passed: false, reason: summary.stop_reason};
        saveProgress();
        break;
      }
      // A fully evidenced finite stop is a miss, not a reason to remove the
      // case from the denominator or halt the fixed batch.
    }
    if (summary.status !== 'stopped') {
      if (options.mode === 'fresh') {
        summary.acceptance = evaluateFreshAcceptance(summary.cases, summary.cases.length === 12);
        summary.status = summary.cases.length === 12 ? 'completed' : 'incomplete';
      } else {
        summary.acceptance = {
          evaluated: summary.cases.length === 4,
          passed: summary.cases.length === 4,
          reason: summary.cases.length === 4 ? 'all four accepted physical sentinels were preserved'
            : 'sentinel batch did not complete',
        };
        summary.status = summary.cases.length === 4 ? 'completed' : 'incomplete';
      }
    }
  }

  const provenanceAfter = currentProvenance(options.bin, expanded);
  const provenanceUnchanged = deepEqual(provenanceBefore, provenanceAfter);
  summary.provenance = {before: provenanceBefore, after: provenanceAfter, unchanged: provenanceUnchanged};
  if (!provenanceUnchanged) {
    summary.status = options.mode === 'preflight-only' ? 'preflight_failed'
      : summary.status === 'completed' ? 'stopped' : summary.status;
    summary.stop_reason = [summary.stop_reason, 'runtime source, binary, runner, fixture or input identity changed during run']
      .filter(Boolean).join('; ');
    summary.acceptance = {evaluated: false, passed: false, reason: summary.stop_reason};
  }
  summary.current_case_id = null;
  delete summary.phase;
  saveProgress();
  return {summary, summaryPath};
}

function validateCompleteFreshRoot(root, expectedExpanded) {
  const path = join(root, SUMMARY_NAME);
  const summary = readJson(path);
  assert.equal(summary.schema_id, 'waypoint_v2_fresh_terrain_readiness_run_v1', `${path}: schema`);
  assert.equal(summary.mode, 'fresh', `${path}: not a fresh root`);
  assert.equal(summary.status, 'completed', `${path}: fresh batch is incomplete`);
  assert.equal(summary.acceptance?.evaluated, true, `${path}: acceptance is not evaluated`);
  assert.equal(summary.acceptance?.passed, true, `${path}: fresh batch failed acceptance`);
  assert.equal(summary.case_count, 12, `${path}: denominator is not twelve`);
  assert.equal(summary.cases.length, 12, `${path}: missing case records`);
  assert.equal(summary.expanded_scenarios_sha256, expectedExpanded.expandedScenariosSha256,
    `${path}: sealed input digest differs`);
  assert.deepEqual(summary.input_identity, compactIdentity(expectedExpanded), `${path}: input identity differs`);
  assert.equal(summary.provenance?.unchanged, true, `${path}: provenance changed during run`);
  assert(deepEqual(summary.provenance.before, summary.provenance.after), `${path}: before/after provenance mismatch`);
  assert.deepEqual(summary.cases.map(record => record.case_id), expectedExpanded.cases.map(entry => entry.case_id),
    `${path}: case order differs`);
  for (const entry of expectedExpanded.cases) {
    const inputPath = join(root, 'inputs', `${entry.case_id}.json`);
    const scenarioPath = join(root, 'runs', entry.case_id, 'scenario.json');
    assert(statSync(inputPath).isFile(), `missing ${inputPath}`);
    assert(statSync(scenarioPath).isFile(), `missing ${scenarioPath}`);
    assert(deepEqual(readJson(inputPath), entry.scenario), `${entry.case_id}: saved input was tampered`);
    assert(deepEqual(readJson(scenarioPath), entry.scenario), `${entry.case_id}: run scenario was tampered`);
  }
  return summary;
}

export function compareFreshRoots(rootA, rootB, expanded = expandFreshInputs()) {
  const leftRoot = resolve(rootA), rightRoot = resolve(rootB);
  assert.notEqual(leftRoot, rightRoot, 'repeat roots must be different paths');
  const left = validateCompleteFreshRoot(leftRoot, expanded);
  const right = validateCompleteFreshRoot(rightRoot, expanded);
  assert.deepEqual(left.provenance.before, right.provenance.before,
    'source tree, runner or CLI binary differs across fresh runs');
  assert.deepEqual(left.input_identity, right.input_identity, 'sealed input identities differ across fresh runs');
  const compared = [];
  for (const entry of expanded.cases) {
    const id = entry.case_id;
    const leftDir = join(leftRoot, 'runs', id), rightDir = join(rightRoot, 'runs', id);
    for (const name of ['scenario.json', 'summary.json', 'flight.json']) {
      const leftPath = join(leftDir, name), rightPath = join(rightDir, name);
      assert(statSync(leftPath).isFile(), `missing ${leftPath}`);
      assert(statSync(rightPath).isFile(), `missing ${rightPath}`);
      const a = readJson(leftPath), b = readJson(rightPath);
      const ignored = name === 'summary.json' ? SUMMARY_IGNORED_POINTERS
        : name === 'flight.json' ? FLIGHT_IGNORED_POINTERS : [];
      const difference = firstDifference(comparable(a, ignored), comparable(b, ignored));
      assert.equal(difference, null, `${id}/${name} differs at ${difference}`);
      compared.push(`${id}/${name}`);
    }
  }
  return {
    deterministic_repeat_passed: true,
    fresh_roots: [leftRoot, rightRoot],
    compared_case_artifacts: compared.length,
    excluded_observational_fields: {
      summary_json: SUMMARY_IGNORED_POINTERS,
      flight_json: FLIGHT_IGNORED_POINTERS,
    },
    source_provenance: left.provenance.before,
    input_identity: left.input_identity,
  };
}

export function parseArguments(args) {
  if (args.includes('--help') || args.includes('-h')) return {help: true};
  if (args.includes('--check-only')) {
    assert.deepEqual(args, ['--check-only'], '--check-only cannot be combined with other options');
    return {checkOnly: true};
  }
  if (args[0] === '--compare') {
    assert.equal(args.length, 3, 'usage: --compare FULL_FRESH_ROOT_A FULL_FRESH_ROOT_B');
    return {compare: [resolve(args[1]), resolve(args[2])]};
  }
  const modes = ['--preflight-only', '--sentinels', '--fresh'];
  const modeFlags = args.filter(value => modes.includes(value));
  assert.equal(modeFlags.length, 1, 'choose exactly one of --preflight-only, --sentinels, or --fresh');
  const mode = modeFlags[0].slice(2);
  const values = new Map();
  const options = args.filter(value => !modes.includes(value));
  for (let index = 0; index < options.length; index += 2) {
    const key = options[index];
    assert(['--bin', '--output-dir', '--baseline-root'].includes(key), `unknown option ${key}`);
    assert(index + 1 < options.length && !options[index + 1].startsWith('--'), `${key} needs a value`);
    assert(!values.has(key), `duplicate option ${key}`);
    values.set(key, options[index + 1]);
  }
  assert(values.has('--bin'), 'required: --bin PATH');
  assert(values.has('--output-dir'), 'required: --output-dir NEW_ROOT');
  assert((mode === 'sentinels') === values.has('--baseline-root'),
    '--baseline-root is required only with --sentinels');
  return {
    mode,
    bin: resolve(values.get('--bin')),
    outputDir: resolve(values.get('--output-dir')),
    baselineRoot: values.has('--baseline-root') ? resolve(values.get('--baseline-root')) : null,
  };
}

function usage() {
  return [
    'Usage:',
    '  node scripts/run_waypoint_v2_fresh_terrain_readiness.mjs --check-only',
    '  node scripts/run_waypoint_v2_fresh_terrain_readiness.mjs --preflight-only --bin PATH --output-dir NEW_ROOT',
    '  node scripts/run_waypoint_v2_fresh_terrain_readiness.mjs --sentinels --bin PATH --output-dir NEW_ROOT --baseline-root ACCEPTED_ROOT',
    '  node scripts/run_waypoint_v2_fresh_terrain_readiness.mjs --fresh --bin PATH --output-dir NEW_ROOT',
    '  node scripts/run_waypoint_v2_fresh_terrain_readiness.mjs --compare FRESH_ROOT_A FRESH_ROOT_B',
  ].join('\n');
}

function main(args = process.argv.slice(2)) {
  const options = parseArguments(args);
  if (options.help) {
    process.stdout.write(`${usage()}\n`);
    return 0;
  }
  const expanded = expandFreshInputs();
  if (options.checkOnly) {
    process.stdout.write(`${jsonText({
      structural_check_passed: true,
      fixture_sha256: expanded.fixtureSha256,
      source_manifest_sha256: expanded.sourceManifestHashes,
      expanded_scenarios_sha256: expanded.expandedScenariosSha256,
      case_count: expanded.cases.length,
      counts: expanded.counts,
      cases: expanded.cases.map(entry => ({
        case_id: entry.case_id,
        group: entry.group,
        family: entry.family,
        recipe_sha256: entry.recipe_sha256,
        scenario_sha256: entry.scenario_sha256,
      })),
    })}`);
    return 0;
  }
  if (options.compare) {
    process.stdout.write(`${jsonText(compareFreshRoots(...options.compare, expanded))}`);
    return 0;
  }
  const {summary, summaryPath} = runSuite(options, expanded);
  process.stdout.write(`${jsonText({summary_path: summaryPath, ...summary})}`);
  if (summary.status === 'completed' && summary.acceptance?.passed === true) return 0;
  if (summary.status === 'preflight_passed') return 0;
  return 1;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : null;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    process.exitCode = main();
  } catch (error) {
    process.stderr.write(`${error.stack ?? error.message}\n${usage()}\n`);
    process.exitCode = 1;
  }
}
