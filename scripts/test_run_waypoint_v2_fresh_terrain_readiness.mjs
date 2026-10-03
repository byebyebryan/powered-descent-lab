import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import {tmpdir} from 'node:os';
import {join, resolve} from 'node:path';
import test from 'node:test';

import {
  compareFreshRoots,
  evaluateFreshAcceptance,
  expandFreshInputs,
  parseArguments,
  runSuite,
} from './run_waypoint_v2_fresh_terrain_readiness.mjs';

const BASELINE_ROOT = resolve('outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a');

const FAKE_CLI = marker => `#!/usr/bin/env node
// fake policy 3 CLI; marker=${marker}
import fs from 'node:fs';
import path from 'node:path';
const args = process.argv.slice(2);
const value = name => args[args.indexOf(name) + 1];
const scenarioPath = value('--scenario');
const scenario = JSON.parse(fs.readFileSync(scenarioPath, 'utf8'));
const id = scenario.id;
const log = process.env.FAKE_LOG;
const writeLog = item => { if (log) fs.appendFileSync(log, JSON.stringify(item) + '\\n'); };
if (args.includes('--preflight-only')) {
  writeLog({kind: 'preflight', id});
  if (process.env.FAKE_MUTATE_BINARY_PREFLIGHT_ID === id) {
    fs.appendFileSync(process.argv[1], '\\n// fake binary drift during preflight\\n');
  }
  process.stdout.write(JSON.stringify({supported: true, rejection: null, reason: null, simulation_created: false}));
  process.exit(0);
}
writeLog({kind: 'run', id});
const outputDir = value('--output-dir');
fs.mkdirSync(outputDir);
fs.copyFileSync(scenarioPath, path.join(outputDir, 'scenario.json'));
const baselineIds = ['v2_clear_845', 'v2_ridge_late', 'v2_successive_rising', 'v2_plateau_reference_900'];
if (process.env.FAKE_USE_BASELINE === '1' && baselineIds.includes(id)) {
  const sourceDir = path.join(process.env.FAKE_BASELINE, 'runs', id);
  fs.copyFileSync(path.join(sourceDir, 'summary.json'), path.join(outputDir, 'summary.json'));
  fs.copyFileSync(path.join(sourceDir, 'flight.json'), path.join(outputDir, 'flight.json'));
  fs.writeFileSync(path.join(outputDir, 'report.html'), '<html>fake rich report</html>');
  const summary = JSON.parse(fs.readFileSync(path.join(outputDir, 'summary.json'), 'utf8'));
  const r = summary.result;
  process.stdout.write(JSON.stringify({policy_version: 3, planning_stop: r.planning_stop, reason: r.reason,
    correction_count: r.correction_count, initial_nominal_terrain_blocked: r.initial_nominal_terrain_blocked,
    integrity_passed: r.integrity_passed, physical_outcome: r.physical_outcome,
    mission_outcome: r.mission_outcome, final_source_replay_passed: r.final_source_replay_passed,
    timings: r.timings, output_dir: outputDir}));
  process.exit(0);
}

const policy = {policy_id: 'piecewise_local_clearing_v2_policy_3',
  maximum_corrections: process.env.FAKE_BAD_POLICY_ID === id ? 5 : 6};
const inputFuel = scenario.vehicle.initial_fuel_kg;
const target = scenario.world.landing_pads.find(pad => pad.id === scenario.mission.goal.target_pad_id);
const source = scenario.world.landing_pads.find(pad => pad.id === 'pad_source');
const miss = process.env.FAKE_MISS_ID === id;
const crash = process.env.FAKE_CRASH_ID === id;
const oddContact = process.env.FAKE_ODD_CONTACT_ID === id;
const badSummary = process.env.FAKE_BAD_SUMMARY_ID === id;
const omitFlight = process.env.FAKE_OMIT_FLIGHT_ID === id;
const terrain = id.startsWith('fresh_') && !id.startsWith('fresh_clear_');
const planningS = 0.125;
const timings = {planning_s: planningS, execution_s: 0.001, replay_s: 0.002};
const state = (step, outcome = 'flying', positionX = source.center_x_m) => ({
  sim_time_s: step / 120,
  physics_step: step,
  position_m: {x: outcome === 'landed_on_target' ? target.center_x_m : positionX,
    y: outcome === 'landed_on_target' ? target.surface_y_m + 5 : scenario.initial_state.position_m.y + step},
  velocity_mps: {x: 0, y: 0},
  attitude_rad: 0,
  angular_rate_radps: 0,
  fuel_kg: inputFuel - (outcome === 'landed_on_target' ? 10 : 0),
  held_command: {throttle_frac: 0, target_attitude_rad: 0},
  physical_outcome: outcome,
  mission_outcome: outcome === 'landed_on_target' ? 'success'
    : outcome === 'crashed' ? 'failed_crash' : 'in_progress',
  end_reason: outcome === 'landed_on_target' ? 'touchdown_on_target'
    : outcome === 'crashed' ? 'crash' : 'running',
  min_touchdown_clearance_m: 0,
  min_hull_clearance_m: 0,
  max_speed_mps: 0,
  max_abs_attitude_rad: 0,
  max_abs_angular_rate_radps: 0,
  waypoint_sequence_passed: 0,
  waypoint_sequence_first_failure_index: null,
  waypoint_handoff_window_index: null,
});
const action = (step, index) => ({sim_time_s: step / 120, physics_step: step,
  controller_update_index: index, command: {throttle_frac: 0.5, target_attitude_rad: 0}});
const update = step => ({physics_step: step, phase: 'nominal', command: {throttle_frac: 0.5, target_attitude_rad: 0}});
const landed = !miss;
const physical = crash ? 'crashed' : landed ? 'landed_on_target' : 'flying';
const mission = crash ? 'failed_crash' : landed ? 'success' : 'in_progress';
const stop = crash ? 'no_progress' : landed ? 'landed' : 'no_clearing';
const endStep = miss ? 0 : oddContact ? 1 : terrain ? 6 : 2;
let finalState = miss ? state(0) : state(endStep, physical);
let cycles = [];
let segments = [];
let actions = [];
let correctionCount = 0;
if (terrain && landed) {
  correctionCount = 1;
  const s0 = state(0);
  const entry = state(2, 'flying', source.center_x_m + 10);
  const handoff = state(4, 'flying', source.center_x_m + 20);
  const after = state(6, 'landed_on_target');
  const certificate = state(10, 'flying', source.center_x_m + 30);
  const u0 = update(0), u2 = update(2), u4 = update(4);
  const trajectory = [entry, handoff, state(6, 'flying'), state(8, 'flying'), certificate]
    .map(s => ({state: s, body_clearance_m: 10}));
  segments = [
    {kind: 'initial_nominal', start_physics_step: 0, end_physics_step: 2,
      proposal_identity: 'fake-initial', updates: [u0], entry_state: s0, end_state: entry},
    {kind: 'local_correction', start_physics_step: 2, end_physics_step: 4,
      proposal_identity: 'fake-local', updates: [u2], entry_state: entry, end_state: handoff},
    {kind: 'airborne_nominal', start_physics_step: 4, end_physics_step: 6,
      proposal_identity: 'fake-airborne', updates: [u4], entry_state: handoff, end_state: after},
  ];
  const selected = {policy: {minimum_clearance_m: 5}, entry_state: entry, handoff_state: handoff,
    continuation_end_state: certificate, trajectory};
  cycles = [
    {cycle_index: 0, current_state: s0, decision: 'local_cleared', fixed_consumed_prefix_proven: true,
      nominal_updates: [u0], audit: {passed: false, commands_match: false},
      local_search: {selected, certificate_state: certificate,
        handoff_source_replay_passed: true, certificate_source_replay_passed: true}},
    {cycle_index: 1, current_state: handoff, decision: 'direct', fixed_consumed_prefix_proven: true,
      nominal_updates: [u4], audit: {passed: true, commands_match: true, safe_target_contact: true,
        ordinary_neutral_parity: true, clearance_scan: {all_airborne_states_passed: true}, final_state: after},
      local_search: null},
  ];
  finalState = after;
  actions = [action(0, 0), action(2, 1), action(4, 2)];
} else if (!miss) {
  const start = state(0);
  const cmd = update(0);
  finalState = state(endStep, physical);
  segments = [{kind: 'initial_nominal', start_physics_step: 0, end_physics_step: endStep,
    proposal_identity: 'fake-direct', updates: [cmd], entry_state: start, end_state: finalState}];
  cycles = [{cycle_index: 0, current_state: start, decision: crash ? 'no_progress' : 'direct',
    fixed_consumed_prefix_proven: true, nominal_updates: [cmd],
    audit: {passed: !crash, commands_match: !crash, safe_target_contact: !crash,
      ordinary_neutral_parity: !crash, clearance_scan: {all_airborne_states_passed: !crash}, final_state: finalState},
    local_search: null}];
  actions = [action(0, 0)];
} else {
  const initial = state(0);
  cycles = [{cycle_index: 0, current_state: initial, decision: 'no_clearing',
    fixed_consumed_prefix_proven: true, nominal_updates: [],
    audit: {passed: false, commands_match: false},
    local_search: {selected: null, handoff_source_replay_passed: false,
      certificate_source_replay_passed: false}}];
}

const result = {planning_stop: stop, reason: miss ? 'NoClearing' : crash ? 'crash' : null,
  correction_count: correctionCount, initial_nominal_terrain_blocked: terrain,
  integrity_passed: true, physical_outcome: physical, mission_outcome: mission,
  final_source_replay_passed: true, timings};
const manifest = {schema_version: 4, scenario_id: id, scenario_name: scenario.name, scenario_seed: scenario.seed,
  scenario_tags: scenario.tags, controller_id: 'waypoint_v2_supplied_commands', physics_hz: 120,
  controller_hz: 60, sim_time_s: endStep / 120, physics_steps: endStep,
  controller_updates: actions.length, physical_outcome: physical, mission_outcome: mission,
  end_reason: finalState.end_reason, summary: {fuel_remaining_kg: finalState.fuel_kg,
    fuel_used_kg: inputFuel - finalState.fuel_kg, min_touchdown_clearance_m: 0,
    min_hull_clearance_m: 0, max_speed_mps: 0, max_abs_attitude_rad: 0,
    max_abs_angular_rate_radps: 0, envelope_margin_ratio: 1,
    landing: physical === 'landed_on_target' ? {on_target: true} : null,
    checkpoint: null, waypoint_sequence: null}};
const runSummary = {minimum_clearance: {touchdown_m: 0, hull_m: 0, landing: manifest.summary.landing},
  fuel: {remaining_kg: finalState.fuel_kg, used_kg: inputFuel - finalState.fuel_kg},
  endpoint: {physics_step: endStep, sim_time_s: endStep / 120,
    physical_outcome: physical, mission_outcome: mission, end_reason: finalState.end_reason}};
const inputIdentity = 'fnv1a64:' + id.padStart(16, '0').slice(-16);
const flight = {policy, input_identity: inputIdentity, ...result, absolute_deadline_physics_step: 10800,
  cycles, segments, ordinary_flight: {final_state: finalState, actions, events: [], samples: []},
  final_source_replay_passed: true, manifest, failed_local_row: null};
const summary = {schema_id: 'waypoint_v2_flight_summary_v1', input_identity: inputIdentity, policy,
  result: badSummary ? {...result, planning_stop: 'no_progress'} : result,
  run_summary: runSummary, timings: {output_s: 0.001, total_s: 0.125}};
fs.writeFileSync(path.join(outputDir, 'summary.json'), JSON.stringify(summary, null, 2) + '\\n');
if (!omitFlight) fs.writeFileSync(path.join(outputDir, 'flight.json'), JSON.stringify(flight, null, 2) + '\\n');
fs.writeFileSync(path.join(outputDir, 'report.html'), '<html>fake rich report</html>');
process.stdout.write(JSON.stringify({policy_version: 3, ...summary.result, timings, output_dir: outputDir}));
`;

function makeWorkspace() {
  return mkdtempSync(join(tmpdir(), 'waypoint-v2-readiness-'));
}

function makeFakeCli(workspace, name = 'fake-pd-eval', marker = '') {
  const path = join(workspace, name);
  writeFileSync(path, FAKE_CLI(marker), {mode: 0o700});
  chmodSync(path, 0o700);
  return path;
}

function readLog(path) {
  if (!existsSync(path)) return [];
  return readFileSync(path, 'utf8').trim().split('\n').filter(Boolean).map(line => JSON.parse(line));
}

function withEnvironment(t, values) {
  const saved = new Map();
  for (const [key, value] of Object.entries(values)) {
    saved.set(key, process.env[key]);
    if (value === null) delete process.env[key];
    else process.env[key] = value;
  }
  t.after(() => {
    for (const [key, value] of saved) {
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
  });
}

function freshRun(workspace, name, bin, env = {}) {
  const outputDir = join(workspace, name);
  const result = runSuite({mode: 'fresh', bin, outputDir}, expandFreshInputs());
  return {outputDir, ...result, env};
}

test('module imports without starting the CLI; parser exposes only bounded modes', () => {
  assert.deepEqual(parseArguments(['--check-only']), {checkOnly: true});
  assert.equal(parseArguments(['--fresh', '--bin', '/tmp/pd', '--output-dir', '/tmp/new']).mode, 'fresh');
  assert.throws(() => parseArguments(['--fresh', '--case', 'one']), /unknown option/);
  assert.throws(() => parseArguments(['--fresh', '--sentinels']), /choose exactly one/);
  const child = spawnSync(process.execPath, ['-e',
    'import("./scripts/run_waypoint_v2_fresh_terrain_readiness.mjs").then(m => process.stdout.write(String(m.expandFreshInputs().cases.length)))'],
  {cwd: resolve('.'), encoding: 'utf8'});
  assert.equal(child.status, 0, child.stderr);
  assert.equal(child.stdout, '12');
});

test('sealed fixture expands twelve physically distinct, shelf-preserving inputs', () => {
  const expanded = expandFreshInputs();
  assert.equal(expanded.expandedScenariosSha256,
    '311752c8da362c2261ce069e633041407b6b59ff88c0c32381d21bd095128c52');
  assert.deepEqual(expanded.counts, {clear: 3, terrain: 9});
  for (const entry of expanded.cases) {
    const terrain = entry.scenario.world.terrain.points_m;
    const source = entry.scenario.world.landing_pads[0];
    const target = entry.scenario.world.landing_pads[1];
    assert.equal(source.width_m, 36);
    assert.equal(target.width_m, 36);
    assert.equal(entry.scenario.initial_state.position_m.x, source.center_x_m);
    assert.equal(entry.scenario.initial_state.position_m.y, 5);
    assert.equal(entry.scenario.mission.transfer_route, null);
    assert.equal(entry.scenario.tags.includes('cutaway'), false);
    assert(terrain.every((point, index) => index === 0 || point.x > terrain[index - 1].x));
  }
});

test('tracked source tampering fails before expansion', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  mkdirSync(join(workspace, 'fixtures/research'), {recursive: true});
  for (const path of [
    'fixtures/research/waypoint_v2_fresh_terrain_inputs_v1.json',
    'fixtures/research/nominal_direct_operational_fresh_inputs_v1.json',
    'fixtures/research/waypoint_direct_obstacle_discrimination_fresh_inputs_v1.json',
  ]) copyFileSync(path, join(workspace, path));
  const sourcePath = join(workspace, 'fixtures/research/nominal_direct_operational_fresh_inputs_v1.json');
  writeFileSync(sourcePath, `${readFileSync(sourcePath, 'utf8')}\n`);
  assert.throws(() => expandFreshInputs({root: workspace}), /source manifest digest changed/);
});

test('preflight mode calls the fixed twelve inputs without mission runs', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  const log = join(workspace, 'calls.jsonl');
  withEnvironment(t, {FAKE_LOG: log, FAKE_MISS_ID: null, FAKE_USE_BASELINE: null});
  const outputDir = join(workspace, 'preflight');
  const result = runSuite({mode: 'preflight-only', bin, outputDir}, expandFreshInputs());
  assert.equal(result.summary.status, 'preflight_passed');
  assert.equal(result.summary.acceptance.evaluated, false);
  assert.equal(result.summary.acceptance.passed, null);
  assert.equal(result.summary.preflight.length, 12);
  assert(result.summary.preflight.every(item => item.supported && !item.simulation_created));
  const calls = readLog(log);
  assert.equal(calls.filter(item => item.kind === 'preflight').length, 12);
  assert.equal(calls.filter(item => item.kind === 'run').length, 0);
});

test('preflight drift fails the CLI without simulating missions', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  const log = join(workspace, 'calls.jsonl');
  const outputDir = join(workspace, 'preflight-drift');
  withEnvironment(t, {FAKE_LOG: log,
    FAKE_MUTATE_BINARY_PREFLIGHT_ID: 'fresh_clear_flat_805'});
  const child = spawnSync(process.execPath, [
    resolve('scripts/run_waypoint_v2_fresh_terrain_readiness.mjs'),
    '--preflight-only', '--bin', bin, '--output-dir', outputDir,
  ], {encoding: 'utf8'});
  assert.equal(child.status, 1, child.stderr);
  const summary = JSON.parse(readFileSync(join(outputDir, 'suite-summary.json'), 'utf8'));
  assert.equal(summary.status, 'preflight_failed');
  assert.equal(summary.provenance.unchanged, false);
  assert.equal(summary.acceptance.passed, false);
  assert.match(summary.stop_reason, /identity changed during run/);
  assert.notEqual(summary.provenance.before.cli_binary_sha256,
    summary.provenance.after.cli_binary_sha256);
  const calls = readLog(log);
  assert.equal(calls.filter(item => item.kind === 'preflight').length, 12);
  assert.equal(calls.filter(item => item.kind === 'run').length, 0);
});

test('even-count planning median averages the two central observations', () => {
  const times = [0.1, 0.2, 0.3, 0.4, 0.5, 1.2, 3.4, 3.5, 3.6, 3.7, 3.8, 4.0];
  const cases = [];
  let index = 0;
  for (const family of ['ridge', 'plateau', 'compound']) {
    for (let ordinal = 0; ordinal < 3; ordinal++) {
      const landed = ordinal < 2 || family === 'compound';
      cases.push({group: 'terrain', family, result: {
        physical_outcome: landed ? 'landed_on_target' : 'flying',
        mission_outcome: landed ? 'success' : 'in_progress',
        correction_count: 1,
        initial_nominal_terrain_blocked: index < 6,
        planning_s: times[index],
      }});
      index++;
    }
  }
  for (let ordinal = 0; ordinal < 3; ordinal++) {
    cases.push({group: 'clear', family: 'clear', result: {
      physical_outcome: 'landed_on_target', mission_outcome: 'success', correction_count: 0,
      initial_nominal_terrain_blocked: false, planning_s: times[index],
    }});
    index++;
  }
  const acceptance = evaluateFreshAcceptance(cases, true);
  assert.equal(acceptance.planning_performance.median_s, 2.3);
  assert.equal(acceptance.planning_performance.p95_nearest_rank_s, 4);
  assert.deepEqual(acceptance.failures, ['planning median exceeds 2 seconds']);
});

test('finite zero-command NoClearing stays in denominator and later cases still run', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  const log = join(workspace, 'calls.jsonl');
  withEnvironment(t, {FAKE_LOG: log, FAKE_MISS_ID: 'fresh_ridge_early_900',
    FAKE_CRASH_ID: null, FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null});
  const result = freshRun(workspace, 'fresh-a', bin);
  assert.equal(result.summary.status, 'completed');
  assert.equal(result.summary.cases.length, 12);
  assert.equal(result.summary.acceptance.passed, true);
  const miss = result.summary.cases.find(item => item.case_id === 'fresh_ridge_early_900');
  assert.equal(miss.result.planning_stop, 'no_clearing');
  assert.equal(miss.result.physical_outcome, 'flying');
  assert.equal(miss.coverage_miss, true);
  assert.equal(miss.cli_exit_code, 0);
  const calls = readLog(log).filter(item => item.kind === 'run');
  assert.equal(calls.length, 12);
  assert.equal(calls.at(-1).id, 'fresh_compound_downhill_plateau_805');
});

test('hard crash stops the incomplete batch before the next mission', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  const log = join(workspace, 'calls.jsonl');
  withEnvironment(t, {FAKE_LOG: log, FAKE_CRASH_ID: 'fresh_clear_flat_805',
    FAKE_MISS_ID: null, FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null});
  const result = freshRun(workspace, 'crash-stop', bin);
  assert.equal(result.summary.status, 'stopped');
  assert.equal(result.summary.cases.length, 1);
  assert.match(result.summary.stop_reason, /physical crash is a hard failure/);
  const runs = readLog(log).filter(item => item.kind === 'run');
  assert.deepEqual(runs.map(item => item.id), ['fresh_clear_flat_805']);
});

test('contradictory and missing result evidence stop the batch', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  for (const [name, variable, expected] of [
    ['contradictory', 'FAKE_BAD_SUMMARY_ID', /summary\/full-flight mismatch/],
    ['missing', 'FAKE_OMIT_FLIGHT_ID', /ENOENT|no such file/i],
  ]) {
    const log = join(workspace, `${name}.jsonl`);
    const env = {FAKE_LOG: log, FAKE_MISS_ID: null, FAKE_CRASH_ID: null,
      FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null};
    env[variable] = 'fresh_clear_flat_805';
    const saved = new Map(Object.keys(env).map(key => [key, process.env[key]]));
    for (const [key, value] of Object.entries(env)) {
      if (value === null) delete process.env[key];
      else process.env[key] = value;
    }
    try {
      const result = freshRun(workspace, name, bin);
      assert.equal(result.summary.status, 'stopped');
      assert.equal(result.summary.cases.length, 1);
      assert.match(result.summary.stop_reason, expected);
    } finally {
      for (const [key, value] of saved) {
        if (value === undefined) delete process.env[key];
        else process.env[key] = value;
      }
    }
  }
});

test('a mutated policy correction cap stops before the next mission', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  const log = join(workspace, 'calls.jsonl');
  withEnvironment(t, {FAKE_LOG: log, FAKE_MISS_ID: null, FAKE_CRASH_ID: null,
    FAKE_BAD_POLICY_ID: 'fresh_clear_flat_805'});
  const result = freshRun(workspace, 'policy-mutant', bin);
  assert.equal(result.summary.status, 'stopped');
  assert.equal(result.summary.cases.length, 1);
  assert.match(result.summary.stop_reason, /compact policy payload changed/);
  assert.deepEqual(readLog(log).filter(item => item.kind === 'run').map(item => item.id),
    ['fresh_clear_flat_805']);
});

test('fresh repeats allow only established timing fields and reject state, command, and provenance tampering', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  withEnvironment(t, {FAKE_LOG: join(workspace, 'calls.jsonl'),
    FAKE_MISS_ID: 'fresh_ridge_early_900', FAKE_CRASH_ID: null,
    FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null});
  const left = freshRun(workspace, 'repeat-a', bin);
  const right = freshRun(workspace, 'repeat-b', bin);
  assert.equal(compareFreshRoots(left.outputDir, right.outputDir).deterministic_repeat_passed, true);

  const summaryPath = join(right.outputDir, 'runs/fresh_clear_flat_805/summary.json');
  const flightPath = join(right.outputDir, 'runs/fresh_clear_flat_805/flight.json');
  const baselineSummary = readFileSync(summaryPath);
  const baselineFlight = readFileSync(flightPath);
  const summary = JSON.parse(baselineSummary);
  const flight = JSON.parse(baselineFlight);
  summary.result.timings.planning_s += 0.25;
  summary.timings.total_s += 0.5;
  flight.timings.planning_s += 0.25;
  writeFileSync(summaryPath, JSON.stringify(summary));
  writeFileSync(flightPath, JSON.stringify(flight));
  assert.equal(compareFreshRoots(left.outputDir, right.outputDir).deterministic_repeat_passed, true);
  writeFileSync(summaryPath, baselineSummary);
  writeFileSync(flightPath, baselineFlight);

  flight.segments[0].entry_state.position_m.x += 1;
  writeFileSync(flightPath, JSON.stringify(flight));
  assert.throws(() => compareFreshRoots(left.outputDir, right.outputDir), /differs at/);
  writeFileSync(flightPath, baselineFlight);

  flight.ordinary_flight.actions[0].command.throttle_frac += 0.01;
  writeFileSync(flightPath, JSON.stringify(flight));
  assert.throws(() => compareFreshRoots(left.outputDir, right.outputDir), /differs at/);
  writeFileSync(flightPath, baselineFlight);

  const suitePath = join(right.outputDir, 'suite-summary.json');
  const suiteBytes = readFileSync(suitePath);
  const suite = JSON.parse(suiteBytes);
  suite.provenance.after.runner_sha256 = '0'.repeat(64);
  writeFileSync(suitePath, JSON.stringify(suite));
  assert.throws(() => compareFreshRoots(left.outputDir, right.outputDir), /before\/after provenance mismatch/);
});

test('create-only output rejects existing directories and dangling symlinks before CLI calls', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  const log = join(workspace, 'calls.jsonl');
  withEnvironment(t, {FAKE_LOG: log, FAKE_MISS_ID: null});
  const existing = join(workspace, 'existing');
  mkdirSync(existing);
  assert.throws(() => runSuite({mode: 'fresh', bin, outputDir: existing}, expandFreshInputs()),
    /already exists \(create-only\)/);
  const dangling = join(workspace, 'dangling');
  symlinkSync(join(workspace, 'absent-target'), dangling);
  assert.throws(() => runSuite({mode: 'fresh', bin, outputDir: dangling}, expandFreshInputs()),
    /already exists \(create-only\)/);
  assert.equal(lstatSync(dangling).isSymbolicLink(), true);
  assert.equal(readLog(log).length, 0);
});

test('sentinels compare to historical artifacts across a different current binary build',
  {skip: !existsSync(join(BASELINE_ROOT, 'suite-summary.json'))}, t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace, 'fake-current-build', 'new-current-build');
  const log = join(workspace, 'calls.jsonl');
  withEnvironment(t, {FAKE_LOG: log, FAKE_USE_BASELINE: '1', FAKE_BASELINE: BASELINE_ROOT,
    FAKE_MISS_ID: null, FAKE_CRASH_ID: null, FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null});
  const result = runSuite({mode: 'sentinels', bin, outputDir: join(workspace, 'sentinels'),
    baselineRoot: BASELINE_ROOT}, expandFreshInputs());
  assert.equal(result.summary.status, 'completed');
  assert.equal(result.summary.cases.length, 4);
  assert.equal(result.summary.acceptance.passed, true);
  assert.equal(result.summary.historical_baseline_provenance.source_provenance_is_historical, true);
  assert.notDeepEqual(result.summary.provenance.before, result.summary.historical_baseline_provenance.suite_provenance.before);
  assert(result.summary.cases.every(item => item.sentinel_comparison?.preserved));
  assert.equal(readLog(log).filter(item => item.kind === 'run').length, 4);
});

test('repeat comparison rejects different recorded build identity', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const binA = makeFakeCli(workspace, 'fake-a', 'build-a');
  const binB = makeFakeCli(workspace, 'fake-b', 'build-b');
  withEnvironment(t, {FAKE_LOG: join(workspace, 'calls.jsonl'), FAKE_MISS_ID: null,
    FAKE_CRASH_ID: null, FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null});
  const left = freshRun(workspace, 'build-a-root', binA);
  const right = freshRun(workspace, 'build-b-root', binB);
  assert.throws(() => compareFreshRoots(left.outputDir, right.outputDir),
    /source tree, runner or CLI binary differs/);
});

test('odd terminal contact consumes the fixed command prefix without padding', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  withEnvironment(t, {FAKE_LOG: join(workspace, 'calls.jsonl'), FAKE_MISS_ID: null,
    FAKE_ODD_CONTACT_ID: 'fresh_clear_flat_805', FAKE_CRASH_ID: null,
    FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null});
  const result = freshRun(workspace, 'odd-contact', bin);
  assert.equal(result.summary.status, 'completed');
  const flight = JSON.parse(readFileSync(join(result.outputDir,
    'runs/fresh_clear_flat_805/flight.json'), 'utf8'));
  assert.equal(flight.segments[0].end_physics_step, 1);
  assert.equal(flight.segments[0].updates.length, 1);
});

test('a clear finite miss is a gate failure after all twelve cases, not a parser hard stop', t => {
  const workspace = makeWorkspace();
  t.after(() => rmSync(workspace, {recursive: true, force: true}));
  const bin = makeFakeCli(workspace);
  withEnvironment(t, {FAKE_LOG: join(workspace, 'calls.jsonl'),
    FAKE_MISS_ID: 'fresh_clear_flat_805', FAKE_CRASH_ID: null,
    FAKE_BAD_SUMMARY_ID: null, FAKE_OMIT_FLIGHT_ID: null});
  const result = freshRun(workspace, 'clear-miss', bin);
  assert.equal(result.summary.status, 'completed');
  assert.equal(result.summary.cases.length, 12);
  assert.equal(result.summary.acceptance.passed, false);
  assert.match(result.summary.acceptance.failures[0], /clear controls/);
  assert.equal(readLog(join(workspace, 'calls.jsonl')).filter(item => item.kind === 'run').length, 12);
});
