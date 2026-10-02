// Run the fixed, create-only waypoint V2 development suite and evaluate its
// predeclared acceptance gates. Scenario expansion intentionally mirrors the
// read-only plan checker so the exact input digest remains independently
// pinned here.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
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
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PLAN_PATH = 'fixtures/research/waypoint_v2_practical_suite_plan_v1.json';
const EXPECTED_EXPANDED_SHA256 = 'c340bb444a24f6c99197ffd18e4f38460c17e80c5e46f03bb4a49cb554f8c5a5';
const SUMMARY_NAME = 'suite-summary.json';
const PHYSICAL_OUTCOMES = new Set([
  'flying', 'landed_on_target', 'landed_off_target', 'crashed', 'timed_out',
]);
const MISSION_OUTCOMES = new Set([
  'in_progress', 'success', 'failed_off_target', 'failed_checkpoint',
  'failed_crash', 'failed_timeout',
]);

// Only these exact summary fields vary by observation location or runtime.
// State, commands, contacts, replay data and all other payload remain compared.
const SUMMARY_IGNORED_POINTERS = [
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
const FLIGHT_IGNORED_POINTERS = [
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
const jsonText = value => `${JSON.stringify(value, null, 2)}\n`;
const isObject = value => value !== null && typeof value === 'object' && !Array.isArray(value);

function sourceTreeSha256() {
  const roots = ['pd-core', 'pd-control', 'pd-plan', 'pd-eval', 'pd-report'];
  const paths = ['Cargo.toml', 'Cargo.lock'];
  for (const crate of roots) {
    paths.push(`${crate}/Cargo.toml`);
    const visit = relative => {
      for (const entry of readdirSync(resolve(ROOT, relative), {withFileTypes: true})) {
        const child = `${relative}/${entry.name}`;
        if (entry.isDirectory()) visit(child);
        else if (entry.isFile() && child.endsWith('.rs')) paths.push(child);
      }
    };
    visit(`${crate}/src`);
  }
  const hash = createHash('sha256');
  for (const path of paths.sort()) {
    hash.update(path).update('\0').update(readFileSync(resolve(ROOT, path))).update('\0');
  }
  return hash.digest('hex');
}

function runProvenance(bin) {
  return {
    cli_binary_sha256: sha256(readFileSync(bin)),
    source_tree_sha256: sourceTreeSha256(),
    suite_runner_sha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
  };
}

function terrainHeight(points, x) {
  assert(x >= points[0].x && x <= points.at(-1).x, 'strict terrain domain');
  for (let index = 1; index < points.length; index++) {
    if (x <= points[index].x) {
      const a = points[index - 1], b = points[index];
      return a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x);
    }
  }
  throw new Error('missing terrain interval');
}

function compileRecipe(recipe, originals) {
  const original = originals.get(recipe.base);
  assert(original, `missing base ${recipe.base}`);
  const scenario = structuredClone(original.scenario);
  assert(scenario.mission.transfer_route == null, 'V2 requires route-free admission');
  assert.equal(scenario.mission.goal.kind, 'landing_on_pad');
  scenario.id = recipe.id;
  scenario.name = recipe.id;
  const source = scenario.world.landing_pads.find(pad => pad.id === original.source_pad_id);
  const target = scenario.world.landing_pads.find(pad => pad.id === original.target_pad_id);
  assert(source && target, `${recipe.id}: missing source or target pad`);
  const baseline = structuredClone(scenario.world.terrain.points_m);
  const added = [];
  let lastEnd = 0;
  for (const feature of recipe.features ?? []) {
    const fractions = feature.fractions;
    assert([3, 4].includes(fractions.length));
    assert(fractions.every((value, index) => Number.isFinite(value) && value > 0 && value < 1
      && (!index || value > fractions[index - 1])));
    assert(fractions[0] > lastEnd, 'features must not overlap');
    assert(Number.isFinite(feature.height_m) && feature.height_m > 0);
    lastEnd = fractions.at(-1);
    for (let index = 0; index < fractions.length; index++) {
      const x = source.center_x_m + fractions[index] * (target.center_x_m - source.center_x_m);
      assert(x > source.center_x_m + source.width_m / 2);
      assert(x < target.center_x_m - target.width_m / 2);
      added.push({
        x,
        y: terrainHeight(baseline, x)
          + (index === 0 || index === fractions.length - 1 ? 0 : feature.height_m),
      });
    }
  }
  // Refuse a feature whose interpolation would silently include an existing
  // terrain breakpoint; all fixture recipes are expected to avoid that case.
  for (const point of baseline) {
    for (const feature of recipe.features ?? []) {
      const left = source.center_x_m + feature.fractions[0]
        * (target.center_x_m - source.center_x_m);
      const right = source.center_x_m + feature.fractions.at(-1)
        * (target.center_x_m - source.center_x_m);
      assert(point.x < left || point.x > right, 'base breakpoint inside inserted feature');
    }
  }
  scenario.world.terrain.points_m = [...baseline, ...added].sort((a, b) => a.x - b.x);
  const points = scenario.world.terrain.points_m;
  assert(points.every((point, index) => Number.isFinite(point.x) && Number.isFinite(point.y)
    && (!index || point.x > points[index - 1].x)));
  assert.deepEqual(scenario.vehicle, original.scenario.vehicle);
  assert.deepEqual(scenario.initial_state, original.scenario.initial_state);
  assert.deepEqual(scenario.sim, original.scenario.sim);
  assert.deepEqual(scenario.mission, original.scenario.mission);
  assert.deepEqual(scenario.world.landing_pads, original.scenario.world.landing_pads);
  for (const point of points) {
    assert(point.y >= terrainHeight(baseline, point.x) - 1e-10, 'floor was lowered');
  }
  for (const pad of [source, target]) {
    assert.equal(terrainHeight(points, pad.center_x_m - pad.width_m / 2), pad.surface_y_m);
    assert.equal(terrainHeight(points, pad.center_x_m + pad.width_m / 2), pad.surface_y_m);
  }
  const mutation = recipe.mutation ?? {};
  assert(Object.keys(mutation).every(key => [
    'target_width_m', 'gravity_mps2', 'dry_mass_delta_kg',
  ].includes(key)));
  assert(recipe.group === 'diagnostic' || !Object.keys(mutation).length);
  if (mutation.target_width_m !== undefined) target.width_m = mutation.target_width_m;
  if (mutation.gravity_mps2 !== undefined) scenario.world.gravity_mps2 = mutation.gravity_mps2;
  if (mutation.dry_mass_delta_kg !== undefined) {
    scenario.vehicle.dry_mass_kg += mutation.dry_mass_delta_kg;
  }
  return {recipe, original, scenario};
}

function loadAndExpandSuite() {
  const rawPlan = readFileSync(resolve(ROOT, PLAN_PATH));
  const plan = JSON.parse(rawPlan);
  assert.equal(plan.schema_id, 'waypoint_v2_practical_suite_plan_v1');
  assert.equal(plan.status, 'design_only_not_flight_accepted');
  const originals = new Map();
  for (const source of Object.values(plan.source_manifests)) {
    const bytes = readFileSync(resolve(ROOT, source.path));
    assert.equal(sha256(bytes), source.sha256, source.path);
    for (const entry of JSON.parse(bytes).cases) {
      assert(!originals.has(entry.case_id), `duplicate base ${entry.case_id}`);
      originals.set(entry.case_id, entry);
    }
  }
  const ids = new Set();
  const counts = {clear: 0, ordinary: 0, diagnostic: 0};
  const families = new Map();
  const cases = plan.cases.map(recipe => {
    assert(!ids.has(recipe.id), `duplicate case ${recipe.id}`);
    ids.add(recipe.id);
    assert(Object.hasOwn(counts, recipe.group), `unknown group ${recipe.group}`);
    counts[recipe.group]++;
    if (recipe.group === 'ordinary') {
      families.set(recipe.family, (families.get(recipe.family) ?? 0) + 1);
    }
    if (recipe.group === 'clear') assert(!recipe.features && !recipe.mutation);
    return compileRecipe(recipe, originals);
  });
  assert.deepEqual(counts, {clear: 8, ordinary: 16, diagnostic: 8});
  for (const [group, count] of Object.entries(counts)) {
    assert.equal(plan.acceptance[`${group}_count`], count);
  }
  assert.deepEqual([...families].sort(), [
    ['plateau', 4], ['ridge', 4], ['sloped', 4], ['successive', 4],
  ]);
  assert.equal(plan.acceptance.required_ordinary_landings, 13);
  assert.equal(plan.acceptance.required_landings_per_ordinary_family, 2);
  assert.equal(plan.acceptance.required_clear_landings_without_corrections, 8);
  assert.equal(plan.acceptance.minimum_initially_blocked_ordinary_cases, 12);
  assert(plan.acceptance.ordinary_failures_remain_in_denominator);
  assert.equal(plan.acceptance.diagnostic_landings_required, 0);
  assert(ids.has(plan.acceptance.reference_case_must_land));
  const reference = cases.find(entry => entry.recipe.id === plan.acceptance.reference_case_must_land);
  assert.equal(reference.recipe.base, 'fresh_late_broad_span_900');
  assert.deepEqual(reference.scenario.world, reference.original.scenario.world);

  const expanded = cases.map(entry => ({
    case_id: entry.recipe.id,
    source_pad_id: entry.original.source_pad_id,
    target_pad_id: entry.original.target_pad_id,
    scenario: entry.scenario,
  }));
  const expandedDigest = sha256(JSON.stringify(canonical(expanded)));
  assert.equal(expandedDigest, EXPECTED_EXPANDED_SHA256,
    'expanded scenario/pad-ID digest differs from the reviewed suite');
  return {
    plan,
    cases,
    counts,
    suiteDigest: sha256(rawPlan),
    expandedDigest,
  };
}

function parseArguments(args) {
  if (args.includes('--help') || args.includes('-h')) return {help: true};
  if (args.includes('--check-only')) {
    assert.deepEqual(args, ['--check-only'], '--check-only cannot be combined with other options');
    return {checkOnly: true};
  }
  if (args[0] === '--compare') {
    assert.equal(args.length, 3, 'usage: --compare FULL_ROOT_A FULL_ROOT_B');
    return {compare: [resolve(args[1]), resolve(args[2])]};
  }
  const values = new Map();
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    assert(['--bin', '--output-dir', '--case', '--policy-version'].includes(key),
      `unknown option ${key}`);
    assert(index + 1 < args.length && !args[index + 1].startsWith('--'), `${key} needs a value`);
    assert(!values.has(key), `duplicate option ${key}`);
    values.set(key, args[index + 1]);
  }
  assert(values.has('--bin'), 'required: --bin PATH');
  assert(values.has('--output-dir'), 'required: --output-dir NEW_ROOT');
  const policyVersion = Number(values.get('--policy-version') ?? '1');
  assert([1, 2, 3].includes(policyVersion), '--policy-version must be 1, 2, or 3');
  return {
    bin: resolve(values.get('--bin')),
    outputDir: resolve(values.get('--output-dir')),
    caseId: values.get('--case'),
    policyVersion,
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

function atomicWrite(path, value) {
  const temporary = `${path}.tmp`;
  writeFileSync(temporary, jsonText(value));
  renameSync(temporary, path);
}

function readCaseResult(summaryPath) {
  const summary = JSON.parse(readFileSync(summaryPath, 'utf8'));
  assert.equal(summary.schema_id, 'waypoint_v2_flight_summary_v1',
    `${summaryPath}: unknown summary schema`);
  const result = summary?.result;
  assert(isObject(result), `${summaryPath}: missing result object`);
  assert(isObject(summary.timings), `${summaryPath}: missing top-level timings`);
  for (const name of ['output_s', 'total_s']) {
    assert(Number.isFinite(summary.timings[name]) && summary.timings[name] >= 0,
      `${summaryPath}: invalid timings.${name}`);
  }
  assert(typeof result.planning_stop === 'string'
    && /^[a-z][a-z0-9_]*$/.test(result.planning_stop),
  `${summaryPath}: planning_stop must be snake_case`);
  assert(Number.isSafeInteger(result.correction_count) && result.correction_count >= 0,
    `${summaryPath}: invalid correction_count`);
  assert(typeof result.initial_nominal_terrain_blocked === 'boolean',
    `${summaryPath}: invalid initial_nominal_terrain_blocked`);
  assert(typeof result.integrity_passed === 'boolean', `${summaryPath}: invalid integrity_passed`);
  assert(result.physical_outcome === null || PHYSICAL_OUTCOMES.has(result.physical_outcome),
    `${summaryPath}: unknown physical_outcome ${result.physical_outcome}`);
  assert(result.mission_outcome === null || MISSION_OUTCOMES.has(result.mission_outcome),
    `${summaryPath}: unknown mission_outcome ${result.mission_outcome}`);
  assert(isObject(result.timings), `${summaryPath}: missing timings`);
  for (const name of ['planning_s', 'execution_s', 'replay_s']) {
    assert(Number.isFinite(result.timings[name]) && result.timings[name] >= 0,
      `${summaryPath}: invalid timings.${name}`);
  }
  if (summary.run_summary !== null) {
    assert(isObject(summary.run_summary), `${summaryPath}: malformed run_summary`);
    for (const field of ['touchdown_m', 'hull_m']) {
      assert(Number.isFinite(summary.run_summary.minimum_clearance?.[field]),
        `${summaryPath}: invalid minimum clearance ${field}`);
    }
    for (const field of ['remaining_kg', 'used_kg']) {
      assert(Number.isFinite(summary.run_summary.fuel?.[field]),
        `${summaryPath}: invalid fuel ${field}`);
    }
    assert(Number.isSafeInteger(summary.run_summary.endpoint?.physics_step),
      `${summaryPath}: invalid endpoint physics_step`);
    assert(Number.isFinite(summary.run_summary.endpoint?.sim_time_s),
      `${summaryPath}: invalid endpoint sim_time_s`);
  }
  return {
    result,
    policyId: summary.policy?.policy_id,
    outputTimings: summary.timings,
    runSummary: summary.run_summary,
  };
}

function expectedPolicyId(policyVersion) {
  return `piecewise_local_clearing_v2_policy_${policyVersion}`;
}

function compactResult(result) {
  return {
    planning_stop: result.planning_stop,
    correction_count: result.correction_count,
    initial_nominal_terrain_blocked: result.initial_nominal_terrain_blocked,
    integrity_passed: result.integrity_passed,
    final_source_replay_passed: result.final_source_replay_passed,
    physical_outcome: result.physical_outcome,
    mission_outcome: result.mission_outcome,
    timings: {
      planning_s: result.timings.planning_s,
      execution_s: result.timings.execution_s,
      replay_s: result.timings.replay_s,
    },
  };
}

function validateFlightEvidence(flightPath, result, entry, runSummary, policyVersion) {
  const flight = JSON.parse(readFileSync(flightPath, 'utf8'));
  assert(isObject(flight), `${flightPath}: full flight must be an object`);
  assert.equal(flight.policy?.policy_id, expectedPolicyId(policyVersion),
    `${flightPath}: policy version disagrees with suite selection`);
  for (const [field, value] of Object.entries({
    planning_stop: result.planning_stop,
    correction_count: result.correction_count,
    initial_nominal_terrain_blocked: result.initial_nominal_terrain_blocked,
    integrity_passed: result.integrity_passed,
    physical_outcome: result.physical_outcome,
    mission_outcome: result.mission_outcome,
    final_source_replay_passed: result.final_source_replay_passed,
  })) {
    assert.deepEqual(flight[field], value, `${flightPath}: summary/result mismatch at ${field}`);
  }
  assert.deepEqual(flight.timings, result.timings, `${flightPath}: result timing mismatch`);
  assert(Array.isArray(flight.cycles), `${flightPath}: missing cycles`);
  assert(Array.isArray(flight.segments), `${flightPath}: missing segments`);
  const rejectedPreflight = ['unsupported', 'invalid_input'].includes(result.planning_stop);
  if (!rejectedPreflight) {
    assert.equal(result.final_source_replay_passed, true,
      `${entry.recipe.id}: supported attempt lacks final source replay`);
    assert(runSummary !== null, `${entry.recipe.id}: replayed run summary missing`);
  }
  for (const cycle of flight.cycles) {
    const local = cycle.local_search;
    if (local?.selected !== null && local?.selected !== undefined) {
      assert.equal(local.handoff_source_replay_passed, true,
        `${entry.recipe.id}: selected handoff lacks source replay`);
      assert.equal(local.certificate_source_replay_passed, true,
        `${entry.recipe.id}: selected certificate lacks source replay`);
    }
  }
  if (entry.recipe.group === 'clear') {
    assert.equal(result.planning_stop, 'landed', `${entry.recipe.id}: clear stop`);
    assert.equal(flight.cycles.length, 1, `${entry.recipe.id}: expected one direct cycle`);
    assert.equal(flight.segments.length, 1, `${entry.recipe.id}: expected one full nominal segment`);
    const cycle = flight.cycles[0];
    assert.equal(cycle.audit?.commands_match, true,
      `${entry.recipe.id}: direct audit did not match nominal commands`);
    assert.equal(cycle.audit?.passed, true, `${entry.recipe.id}: direct audit failed`);
    assert.deepEqual(flight.segments[0].updates, cycle.nominal_updates,
      `${entry.recipe.id}: consumed commands differ from declared nominal commands`);
  }
  return {
    minimum_clearance: runSummary?.minimum_clearance ?? null,
    fuel: runSummary?.fuel ?? null,
    endpoint: runSummary?.endpoint ?? null,
  };
}

const landed = result => result?.physical_outcome === 'landed_on_target'
  && result?.mission_outcome === 'success';

function evaluateAcceptance(cases, records, fullSuite) {
  if (!fullSuite) {
    return {evaluated: false, passed: null, reason: 'single-case development run'};
  }
  const byId = new Map(records.map(record => [record.case_id, record]));
  if (records.length !== cases.length || cases.some(entry => !byId.has(entry.recipe.id))) {
    return {evaluated: false, passed: null, reason: 'suite did not complete all 32 cases'};
  }
  const failures = [];
  const selected = group => cases
    .filter(entry => entry.recipe.group === group)
    .map(entry => ({entry, record: byId.get(entry.recipe.id)}));
  const clear = selected('clear');
  const ordinary = selected('ordinary');
  const diagnostic = selected('diagnostic');
  const clearLandings = clear.filter(({record}) => landed(record.result));
  const clearPass = clearLandings.length === 8
    && clear.every(({record}) => record.result.correction_count === 0);
  if (!clearPass) failures.push('clear controls must all land with zero corrections');

  const ordinaryLandings = ordinary.filter(({record}) => landed(record.result));
  if (ordinaryLandings.length < 13) failures.push('ordinary landing floor is 13 of 16');
  const familyLandings = {};
  for (const family of ['ridge', 'plateau', 'successive', 'sloped']) {
    familyLandings[family] = ordinaryLandings
      .filter(({entry}) => entry.recipe.family === family).length;
    if (familyLandings[family] < 2) failures.push(`${family} landing floor is 2 of 4`);
  }
  const reference = byId.get('v2_plateau_reference_900');
  const referencePass = landed(reference?.result) && reference.result.correction_count >= 2;
  if (!referencePass) {
    failures.push('reference plateau must land after repeated corrections (at least two)');
  }
  const initiallyBlocked = ordinary.filter(({record}) =>
    record.result.initial_nominal_terrain_blocked).length;
  if (initiallyBlocked < 12) failures.push('at least 12 ordinary cases must block the initial nominal');

  const planningTimes = [...clear, ...ordinary]
    .map(({record}) => record.result.timings.planning_s)
    .sort((a, b) => a - b);
  assert.equal(planningTimes.length, 24, 'performance denominator must include 24 attempts');
  const median = (planningTimes[11] + planningTimes[12]) / 2;
  const p95NearestRank = planningTimes[Math.ceil(0.95 * planningTimes.length) - 1];
  if (median > 2) failures.push('planning median exceeds 2 seconds');
  if (p95NearestRank > 5) failures.push('planning nearest-rank p95 exceeds 5 seconds');

  const implementationErrors = records
    .filter(record => record.result.planning_stop === 'implementation_error')
    .map(record => record.case_id);
  const integrityFailures = records
    .filter(record => record.result.integrity_passed !== true)
    .map(record => record.case_id);
  if (implementationErrors.length) failures.push(`implementation errors: ${implementationErrors.join(', ')}`);
  if (integrityFailures.length) failures.push(`integrity failures: ${integrityFailures.join(', ')}`);
  // Diagnostics intentionally have no landing quota; their integrity and
  // ImplementationError checks are the same as all other cases.
  assert.equal(diagnostic.length, 8);
  return {
    evaluated: true,
    passed: failures.length === 0,
    failures,
    clear: {landed: clearLandings.length, required: 8, zero_correction_controls: clearPass},
    ordinary: {
      landed: ordinaryLandings.length,
      required: 13,
      family_landings: familyLandings,
      initially_blocked: initiallyBlocked,
      initially_blocked_required: 12,
      reference_landed: landed(reference?.result),
      reference_corrections: reference?.result.correction_count ?? null,
    },
    diagnostics: {count: diagnostic.length, landing_quota: 0},
    planning_performance: {
      sample_count: planningTimes.length,
      median_s: median,
      p95_nearest_rank_s: p95NearestRank,
      median_limit_s: 2,
      p95_limit_s: 5,
    },
  };
}

function checkFullSuiteSummary(root, expandedCases) {
  const path = join(root, SUMMARY_NAME);
  const summary = JSON.parse(readFileSync(path, 'utf8'));
  assert.equal(summary.schema_id, 'waypoint_v2_practical_suite_run_v1', `${path}: schema`);
  assert.equal(summary.status, 'completed', `${path}: not a completed full suite`);
  assert.equal(summary.selected_case_id, null, `${path}: partial --case root is not comparable`);
  assert.equal(summary.expanded_scenarios_sha256, EXPECTED_EXPANDED_SHA256, `${path}: input digest`);
  assert([1, 2, 3].includes(summary.policy_version), `${path}: unknown policy version`);
  assert.equal(summary.provenance?.unchanged, true, `${path}: source/build provenance changed during run`);
  assert.equal(summary.cases.length, 32, `${path}: case count`);
  assert.deepEqual(summary.cases.map(record => record.case_id),
    expandedCases.map(entry => entry.recipe.id), `${path}: case order`);
  return summary;
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

function firstDifference(left, right, pointer = '') {
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
      if (!Object.hasOwn(left, key) || !Object.hasOwn(right, key)) {
        return `${pointer}/${key}`;
      }
      const difference = firstDifference(left[key], right[key], `${pointer}/${key}`);
      if (difference) return difference;
    }
    return null;
  }
  return pointer || '/';
}

function compareFullRoots(rootA, rootB, cases) {
  assert.notEqual(rootA, rootB, 'repeat roots must be different paths');
  const suiteA = checkFullSuiteSummary(rootA, cases);
  const suiteB = checkFullSuiteSummary(rootB, cases);
  assert.equal(suiteA.suite_sha256, suiteB.suite_sha256, 'suite fixture differs across roots');
  assert.equal(suiteA.policy_version, suiteB.policy_version,
    'cannot compare runs from different waypoint V2 policy versions');
  assert.deepEqual(suiteA.provenance.before, suiteB.provenance.before,
    'source tree, runner or CLI binary differs across repeat roots');
  const compared = [];
  for (const entry of cases) {
    const id = entry.recipe.id;
    const caseA = join(rootA, 'runs', id);
    const caseB = join(rootB, 'runs', id);
    for (const name of ['summary.json', 'flight.json']) {
      const leftPath = join(caseA, name), rightPath = join(caseB, name);
      assert(statSync(leftPath).isFile(), `missing ${leftPath}`);
      assert(statSync(rightPath).isFile(), `missing ${rightPath}`);
      const left = JSON.parse(readFileSync(leftPath, 'utf8'));
      const right = JSON.parse(readFileSync(rightPath, 'utf8'));
      const ignored = name === 'summary.json' ? SUMMARY_IGNORED_POINTERS : FLIGHT_IGNORED_POINTERS;
      const comparableLeft = comparable(left, ignored);
      const comparableRight = comparable(right, ignored);
      const difference = firstDifference(comparableLeft, comparableRight);
      assert.equal(difference, null, `${id}/${name} differs at ${difference}`);
      compared.push(`${id}/${name}`);
    }
  }
  return {
    deterministic_repeat_passed: true,
    full_roots: [rootA, rootB],
    compared_case_artifacts: compared.length,
    excluded_observational_fields: {
      summary_json: SUMMARY_IGNORED_POINTERS,
      flight_json: FLIGHT_IGNORED_POINTERS,
    },
  };
}

function validateCliPath(bin) {
  assert(existsSync(bin), `CLI binary does not exist: ${bin}`);
  assert(statSync(bin).isFile(), `CLI binary is not a file: ${bin}`);
  accessSync(bin, fsConstants.X_OK);
}

function runSuite(options, suite) {
  validateCliPath(options.bin);
  const provenanceBefore = runProvenance(options.bin);
  if (options.caseId) {
    assert(suite.cases.some(entry => entry.recipe.id === options.caseId),
      `unknown --case ${options.caseId}`);
  }
  assert(!fileExistsIncludingDanglingSymlink(options.outputDir),
    `output root already exists (create-only): ${options.outputDir}`);
  mkdirSync(options.outputDir);
  const inputDir = join(options.outputDir, 'inputs');
  const runsDir = join(options.outputDir, 'runs');
  mkdirSync(inputDir);
  mkdirSync(runsDir);
  const selectedCases = options.caseId
    ? suite.cases.filter(entry => entry.recipe.id === options.caseId)
    : suite.cases;
  const summary = {
    schema_id: 'waypoint_v2_practical_suite_run_v1',
    status: 'running',
    suite_sha256: suite.suiteDigest,
    expanded_scenarios_sha256: suite.expandedDigest,
    policy_version: options.policyVersion,
    case_count: selectedCases.length,
    selected_case_id: options.caseId ?? null,
    provenance: {before: provenanceBefore, after: null, unchanged: null},
    cases: [],
    acceptance: {evaluated: false, passed: null, reason: 'suite is still running'},
  };
  const summaryPath = join(options.outputDir, SUMMARY_NAME);
  const saveProgress = () => atomicWrite(summaryPath, summary);
  saveProgress();
  let stopReason = null;
  for (const entry of selectedCases) {
    const id = entry.recipe.id;
    const scenarioPath = join(inputDir, `${id}.json`);
    const caseOutput = join(runsDir, id);
    writeFileSync(scenarioPath, jsonText(entry.scenario), {flag: 'wx'});
    const args = [
      'waypoint-v2-flight',
      '--policy-version', String(options.policyVersion),
      '--scenario', scenarioPath,
      '--source-pad-id', entry.original.source_pad_id,
      '--target-pad-id', entry.original.target_pad_id,
      '--output-dir', caseOutput,
    ];
    const child = spawnSync(options.bin, args, {
      cwd: ROOT,
      encoding: 'utf8',
      maxBuffer: 16 * 1024 * 1024,
    });
    let result;
    const record = {
      case_id: id,
      group: entry.recipe.group,
      family: entry.recipe.family,
      cli_exit_code: child.status,
      cli_signal: child.signal,
      summary_path: `runs/${id}/summary.json`,
      flight_path: `runs/${id}/flight.json`,
    };
    if (child.error) {
      record.error = `failed to invoke CLI: ${child.error.message}`;
      summary.cases.push(record);
      stopReason = record.error;
      summary.status = 'stopped';
      summary.stop_reason = stopReason;
      summary.acceptance = evaluateAcceptance(selectedCases, summary.cases, !options.caseId);
      saveProgress();
      break;
    }
    try {
      const parsed = readCaseResult(join(caseOutput, 'summary.json'));
      assert.equal(parsed.policyId, expectedPolicyId(options.policyVersion),
        `${id}: CLI summary policy disagrees with suite selection`);
      result = parsed.result;
      const {outputTimings, runSummary} = parsed;
      record.result = compactResult(result);
      record.run_summary = validateFlightEvidence(
        join(caseOutput, 'flight.json'), result, entry, runSummary, options.policyVersion,
      );
      record.output_timings = {
        output_s: outputTimings.output_s,
        total_s: outputTimings.total_s,
      };
      assert(statSync(join(caseOutput, 'flight.json')).isFile(),
        `${id}: full flight.json is missing`);
    } catch (error) {
      record.error = error.message;
      summary.cases.push(record);
      stopReason = `${id}: ${error.message}`;
      summary.status = 'stopped';
      summary.stop_reason = stopReason;
      summary.acceptance = evaluateAcceptance(selectedCases, summary.cases, !options.caseId);
      saveProgress();
      break;
    }
    summary.cases.push(record);
    // Persist each complete attempt before checking fatal evidence so an
    // implementation/integrity failure leaves a reviewable partial suite.
    summary.acceptance = evaluateAcceptance(selectedCases, summary.cases, !options.caseId);
    saveProgress();
    if (result.planning_stop === 'implementation_error') {
      stopReason = `${id}: planning_stop is implementation_error`;
    } else if (!result.integrity_passed) {
      stopReason = `${id}: integrity_passed is false`;
    }
    if (stopReason) {
      summary.status = 'stopped';
      summary.stop_reason = stopReason;
      saveProgress();
      break;
    }
  }
  const provenanceAfter = runProvenance(options.bin);
  const provenanceUnchanged = Object.keys(provenanceBefore)
    .every(key => provenanceBefore[key] === provenanceAfter[key]);
  summary.provenance = {
    before: provenanceBefore,
    after: provenanceAfter,
    unchanged: provenanceUnchanged,
  };
  if (!provenanceUnchanged) {
    const provenanceFailure = 'CLI binary, suite runner or relevant source tree changed during the run';
    stopReason = stopReason ? `${stopReason}; ${provenanceFailure}` : provenanceFailure;
    summary.status = 'stopped';
    summary.stop_reason = stopReason;
    summary.acceptance = evaluateAcceptance(selectedCases, summary.cases, !options.caseId);
    saveProgress();
  } else if (!stopReason) {
    summary.status = 'completed';
    summary.acceptance = evaluateAcceptance(selectedCases, summary.cases, !options.caseId);
    saveProgress();
  } else {
    saveProgress();
  }
  return {summary, summaryPath};
}

function usage() {
  return [
    'Usage:',
    '  node scripts/run_waypoint_v2_practical_suite.mjs --bin PATH --output-dir NEW_ROOT [--case ID] [--policy-version 1|2|3]',
    '  node scripts/run_waypoint_v2_practical_suite.mjs --compare FULL_ROOT_A FULL_ROOT_B',
    '  node scripts/run_waypoint_v2_practical_suite.mjs --check-only',
  ].join('\n');
}

function main() {
  const options = parseArguments(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(`${usage()}\n`);
    return 0;
  }
  const suite = loadAndExpandSuite();
  if (options.checkOnly) {
    process.stdout.write(`${jsonText({
      structural_check_passed: true,
      suite_sha256: suite.suiteDigest,
      expanded_scenarios_sha256: suite.expandedDigest,
      case_count: suite.cases.length,
      counts: suite.counts,
    })}`);
    return 0;
  }
  if (options.compare) {
    process.stdout.write(`${jsonText(compareFullRoots(...options.compare, suite.cases))}`);
    return 0;
  }
  const {summary, summaryPath} = runSuite(options, suite);
  process.stdout.write(`${jsonText({summary_path: summaryPath, ...summary})}`);
  return summary.status === 'completed' && summary.acceptance.passed !== false ? 0 : 1;
}

try {
  process.exitCode = main();
} catch (error) {
  process.stderr.write(`${error.stack ?? error.message}\n${usage()}\n`);
  process.exitCode = 1;
}
