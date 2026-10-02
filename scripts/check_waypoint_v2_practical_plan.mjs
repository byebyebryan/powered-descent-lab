// Read-only design checks. This does not generate trajectories or run flights.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const relative = 'fixtures/research/waypoint_v2_practical_suite_plan_v1.json';
const raw = readFileSync(resolve(root, relative));
const plan = JSON.parse(raw);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => Array.isArray(value) ? value.map(canonical)
  : value !== null && typeof value === 'object'
    ? Object.fromEntries(Object.keys(value).sort().map(k => [k, canonical(value[k])]))
    : value;
assert.equal(plan.schema_id, 'waypoint_v2_practical_suite_plan_v1');
assert.equal(plan.status, 'design_only_not_flight_accepted');
const originals = new Map();
for (const source of Object.values(plan.source_manifests)) {
  const bytes = readFileSync(resolve(root, source.path));
  assert.equal(sha(bytes), source.sha256, source.path);
  for (const entry of JSON.parse(bytes).cases) {
    assert(!originals.has(entry.case_id), `duplicate base ${entry.case_id}`);
    originals.set(entry.case_id, entry);
  }
}

function height(points, x) {
  assert(x >= points[0].x && x <= points.at(-1).x, 'strict terrain domain');
  for (let i = 1; i < points.length; i++) {
    if (x <= points[i].x) {
      const a = points[i - 1], b = points[i];
      return a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x);
    }
  }
  throw Error('missing terrain interval');
}

function compile(recipe) {
  const original = originals.get(recipe.base);
  assert(original, `missing base ${recipe.base}`);
  const scenario = structuredClone(original.scenario);
  assert(scenario.mission.transfer_route == null, 'V2 requires route-free admission');
  assert.equal(scenario.mission.goal.kind, 'landing_on_pad');
  scenario.id = recipe.id;
  scenario.name = recipe.id;
  const source = scenario.world.landing_pads.find(p => p.id === original.source_pad_id);
  const target = scenario.world.landing_pads.find(p => p.id === original.target_pad_id);
  const baseline = structuredClone(scenario.world.terrain.points_m);
  const added = [];
  let lastEnd = 0;
  for (const feature of recipe.features ?? []) {
    const f = feature.fractions;
    assert([3, 4].includes(f.length));
    assert(f.every((v, i) => Number.isFinite(v) && v > 0 && v < 1 && (!i || v > f[i - 1])));
    assert(f[0] > lastEnd, 'features must not overlap');
    assert(Number.isFinite(feature.height_m) && feature.height_m > 0);
    lastEnd = f.at(-1);
    for (let i = 0; i < f.length; i++) {
      const x = source.center_x_m + f[i] * (target.center_x_m - source.center_x_m);
      assert(x > source.center_x_m + source.width_m / 2);
      assert(x < target.center_x_m - target.width_m / 2);
      added.push({x, y: height(baseline, x) + (i === 0 || i === f.length - 1 ? 0 : feature.height_m)});
    }
  }
  // These recipes have no base breakpoints inside their new features. Refuse
  // such a recipe instead of silently bending its advertised feature shape.
  for (const point of baseline) {
    for (const feature of recipe.features ?? []) {
      const left = source.center_x_m + feature.fractions[0] * (target.center_x_m - source.center_x_m);
      const right = source.center_x_m + feature.fractions.at(-1) * (target.center_x_m - source.center_x_m);
      assert(point.x < left || point.x > right, 'base breakpoint inside inserted feature');
    }
  }
  scenario.world.terrain.points_m = [...baseline, ...added].sort((a, b) => a.x - b.x);
  const points = scenario.world.terrain.points_m;
  assert(points.every((p, i) => Number.isFinite(p.x) && Number.isFinite(p.y) && (!i || p.x > points[i - 1].x)));
  assert.deepEqual(scenario.vehicle, original.scenario.vehicle);
  assert.deepEqual(scenario.initial_state, original.scenario.initial_state);
  assert.deepEqual(scenario.sim, original.scenario.sim);
  assert.deepEqual(scenario.mission, original.scenario.mission);
  assert.deepEqual(scenario.world.landing_pads, original.scenario.world.landing_pads);
  for (const p of points) assert(p.y >= height(baseline, p.x) - 1e-10, 'floor was lowered');
  for (const pad of [source, target]) {
    assert.equal(height(points, pad.center_x_m - pad.width_m / 2), pad.surface_y_m);
    assert.equal(height(points, pad.center_x_m + pad.width_m / 2), pad.surface_y_m);
  }
  const mutation = recipe.mutation ?? {};
  assert(Object.keys(mutation).every(k => ['target_width_m', 'gravity_mps2', 'dry_mass_delta_kg'].includes(k)));
  assert(recipe.group === 'diagnostic' || !Object.keys(mutation).length);
  if (mutation.target_width_m !== undefined) target.width_m = mutation.target_width_m;
  if (mutation.gravity_mps2 !== undefined) scenario.world.gravity_mps2 = mutation.gravity_mps2;
  if (mutation.dry_mass_delta_kg !== undefined) scenario.vehicle.dry_mass_kg += mutation.dry_mass_delta_kg;
  return {recipe, original, scenario};
}

const ids = new Set();
const counts = {clear: 0, ordinary: 0, diagnostic: 0};
const families = new Map();
const cases = plan.cases.map(recipe => {
  assert(!ids.has(recipe.id)); ids.add(recipe.id);
  assert(Object.hasOwn(counts, recipe.group)); counts[recipe.group]++;
  if (recipe.group === 'ordinary') families.set(recipe.family, (families.get(recipe.family) ?? 0) + 1);
  if (recipe.group === 'clear') assert(!recipe.features && !recipe.mutation);
  return compile(recipe);
});
assert.deepEqual(counts, {clear: 8, ordinary: 16, diagnostic: 8});
for (const [group, count] of Object.entries(counts)) assert.equal(plan.acceptance[`${group}_count`], count);
assert.deepEqual([...families].sort(), [['plateau', 4], ['ridge', 4], ['sloped', 4], ['successive', 4]]);
assert.equal(plan.acceptance.required_ordinary_landings, 13);
assert.equal(plan.acceptance.required_landings_per_ordinary_family, 2);
assert.equal(plan.acceptance.required_clear_landings_without_corrections, 8);
assert.equal(plan.acceptance.minimum_initially_blocked_ordinary_cases, 12);
assert(plan.acceptance.ordinary_failures_remain_in_denominator);
assert.equal(plan.acceptance.diagnostic_landings_required, 0);
assert(ids.has(plan.acceptance.reference_case_must_land));
const ref = cases.find(c => c.recipe.id === plan.acceptance.reference_case_must_land);
assert.equal(ref.recipe.base, 'fresh_late_broad_span_900');
assert.deepEqual(ref.scenario.world, ref.original.scenario.world);
const flatRecipe = {id: 'negative_recipe', group: 'ordinary', family: 'ridge', base: 'fresh_flat_control_span_900'};
assert.throws(() => compile({...flatRecipe, features: [{fractions: [0.2, 0.3, 0.4], height_m: -1}]}));
assert.throws(() => compile({...flatRecipe, features: [{fractions: [0.2, 0.1, 0.4], height_m: 100}]}));
assert.throws(() => compile({...flatRecipe, features: [{fractions: [0.001, 0.1, 0.2], height_m: 100}]}));
assert.throws(() => compile({...flatRecipe, features: [{fractions: [0.2, 0.3, 0.4], height_m: 100}, {fractions: [0.35, 0.5, 0.6], height_m: 100}]}));
assert.throws(() => compile({...flatRecipe, mutation: {gravity_mps2: 1.62}}));
assert.throws(() => compile({...flatRecipe, base: 'missing_base'}));
const p = plan.proposed_policy;
assert.equal(p.maximum_corrections, 6);
assert.equal(p.maximum_entries_per_correction, 4);
assert.equal(p.templates_per_entry, 42);
assert.equal(p.boundaries_per_template, 360);
assert.equal(p.maximum_coast_ticks, 720);
assert.equal(p.continuation_ticks, 240);
assert.equal(p.minimum_clearance_m, 5);
assert.deepEqual(p.later_entry_fractions, [0, 0.25, 0.5, 0.75]);
function entries(c, conflict) {
  assert(Number.isSafeInteger(c) && c > 0 && c % 2 === 0);
  assert(Number.isSafeInteger(conflict) && conflict > c);
  return [...new Set(p.later_entry_fractions.map(f => c + 2 * Math.floor(f * (conflict - c) / 2)))];
}
assert.deepEqual(entries(2820, 3120), [2820, 2894, 2970, 3044]);
assert.deepEqual(entries(100, 101), [100]);
assert.deepEqual(entries(100, 103), [100, 102]);
assert.throws(() => entries(0, 100));
assert.throws(() => entries(101, 200));
assert.throws(() => entries(200, 200));
const isPrefixUpdate = (tick, a, b) => tick >= a && tick < b;
assert(isPrefixUpdate(2820, 2820, 3182));
assert(isPrefixUpdate(3180, 2820, 3182));
assert(!isPrefixUpdate(3182, 2820, 3182));
assert.equal(3182 + p.continuation_ticks, 3422);
assert(3422 < 9600);
const maxRows = p.maximum_corrections * p.maximum_entries_per_correction * p.templates_per_entry;
const maxBoundaries = maxRows * p.boundaries_per_template;
assert.equal(maxRows, 1008);
assert.equal(maxBoundaries, 362880);

const args = process.argv.slice(2);
assert(!args.length || (args.length === 2 && args[0] === '--preflight-bin'), 'usage: node script [--preflight-bin PATH]');
let preflight = null;
if (args.length) {
  preflight = {supported: 0, expected_unsupported: 0, simulation_created: false};
  for (const c of cases) {
    // Node's subprocess stdin is a socket, which Rust fs::read cannot reopen
    // through /dev/stdin on Linux. A read-only shell pipe supplies a real pipe;
    // positional arguments avoid interpreting paths or pad IDs as shell text.
    const run = spawnSync('rtk', ['proxy', 'bash', '-o', 'pipefail', '-c',
      'rtk proxy cat | rtk proxy "$1" nominal-direct-flight --scenario /dev/stdin --source-pad-id "$2" --target-pad-id "$3" --preflight-only',
      'preflight-input', resolve(root, args[1]), c.original.source_pad_id, c.original.target_pad_id], {
      cwd: root, input: JSON.stringify(c.scenario), encoding: 'utf8', timeout: 10000
    });
    assert(!run.error, `${c.recipe.id}: ${run.error}`);
    assert(run.stdout.trim(), `${c.recipe.id}: preflight returned no JSON, status ${run.status}: ${run.stderr}`);
    const result = JSON.parse(run.stdout);
    assert.equal(result.simulation_created, false, c.recipe.id);
    if (c.recipe.expected_preflight === 'unsupported') {
      assert.equal(c.recipe.group, 'diagnostic');
      assert.notEqual(run.status, 0);
      assert.equal(result.supported, false);
      assert.equal(result.rejection.status, 'unsupported');
      preflight.expected_unsupported++;
    } else {
      assert.equal(run.status, 0, `${c.recipe.id}: ${run.stderr}`);
      assert.equal(result.supported, true, c.recipe.id);
      preflight.supported++;
    }
  }
  assert.deepEqual(preflight, {supported: 30, expected_unsupported: 2, simulation_created: false});
}
console.log(JSON.stringify({
  design_checks_passed: true, suite_sha256: sha(raw), counts,
  expanded_scenarios_sha256: sha(JSON.stringify(canonical(cases.map(c => ({
    case_id: c.recipe.id, source_pad_id: c.original.source_pad_id,
    target_pad_id: c.original.target_pad_id, scenario: c.scenario
  }))))),
  ordinary_success_floor: '13/16 (81.25 percent), at least 2/4 per family',
  maximum_local_rows: maxRows, maximum_boundary_checks: maxBoundaries,
  later_entries_at_reference_handoff: entries(2820, 3120), preflight,
  flight_acceptance: 'not evaluated'
}, null, 2));
