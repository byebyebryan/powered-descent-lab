// Contract checks for the frozen nominal-characterization corpus builder.
import assert from 'node:assert/strict';
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

import { buildCorpus, __test } from './waypoint_v2_nominal_characterization_corpus.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = join(ROOT, 'scripts/waypoint_v2_nominal_characterization_corpus.mjs');
const TEMP_ROOT = mkdtempSync(join(tmpdir(), 'pd-v2-nominal-corpus-test-'));

function scenario() {
  return {
    id: 'fake-scenario',
    name: 'fake-scenario',
    description: 'small validation fixture',
    metadata: {},
    initial_state: {
      position_m: {x: -5, y: 5},
      velocity_mps: {x: 0, y: 0},
      attitude_rad: 0,
      angular_rate_radps: 0,
    },
    mission: {goal: {kind: 'landing_on_pad', target_pad_id: 'pad_main'}},
    seed: 1,
    sim: {physics_hz: 120, controller_hz: 60, max_time_s: 10, sample_hz: 10},
    tags: [],
    vehicle: {dry_mass_kg: 1, initial_fuel_kg: 1},
    world: {
      gravity_mps2: 9.81,
      terrain: {kind: 'heightfield', points_m: [{x: -10, y: 0}, {x: 10, y: 0}]},
      landing_pads: [
        {id: 'pad_source', center_x_m: -5, surface_y_m: 0, width_m: 2},
        {id: 'pad_main', center_x_m: 5, surface_y_m: 0, width_m: 2},
      ],
    },
  };
}

function snapshot(physicsStep) {
  return {
    sim_time_s: physicsStep / 120,
    physics_step: physicsStep,
    position_m: {x: 1, y: 20},
    velocity_mps: {x: 3, y: -1},
    attitude_rad: 0.1,
    angular_rate_radps: 0,
    fuel_kg: 0.8,
    held_command: {throttle_frac: 0, target_attitude_rad: 0},
    physical_outcome: 'flying',
    mission_outcome: 'in_progress',
    end_reason: 'running',
    min_touchdown_clearance_m: 0,
    min_hull_clearance_m: 0,
    max_speed_mps: 3.2,
    max_abs_attitude_rad: 0.1,
    max_abs_angular_rate_radps: 0,
    waypoint_sequence_passed: 0,
    waypoint_sequence_first_failure_index: null,
    waypoint_handoff_window_index: null,
  };
}

function update(physicsStep, phase = 'coast', throttle = 0.4) {
  return {
    physics_step: physicsStep,
    phase,
    command: {throttle_frac: throttle, target_attitude_rad: 0.2},
  };
}

function fakeRows() {
  const s = scenario();
  return [
    {
      id: 'local_handoff:fixture:H4',
      population: 'local_handoff',
      label: 'fixture local handoff',
      scenario: structuredClone(s),
      source_pad_id: 'pad_source',
      target_pad_id: 'pad_main',
      absolute_deadline_physics_step: 20,
      expected_state: snapshot(4),
      prefix_updates: [update(0, 'upright'), update(2, 'local_boost')],
    },
    {
      id: 'historical_capture:fixture:first_coast:H4',
      population: 'historical_capture',
      label: 'fixture historical capture',
      scenario: structuredClone(s),
      source_pad_id: 'pad_source',
      target_pad_id: 'pad_main',
      absolute_deadline_physics_step: 20,
      expected_state: snapshot(4),
      prefix_updates: [update(0), update(2)],
    },
    {
      id: 'clear_start:fixture',
      population: 'clear_start',
      label: 'fixture clear start',
      scenario: structuredClone(s),
      source_pad_id: 'pad_source',
      target_pad_id: 'pad_main',
      absolute_deadline_physics_step: 20,
      expected_state: null,
      prefix_updates: [],
    },
  ];
}

function commandLog(updates) {
  return updates.map(({physics_step, command}) => ({
    physics_step,
    command: structuredClone(command),
  }));
}

function testMinimalSchemas() {
  const expected = {clear_start: 1, local_handoff: 1, historical_capture: 1};
  const ordered = __test.sortAndValidateRows(fakeRows(), expected);
  assert.deepEqual(ordered.map(row => row.population), [
    'clear_start', 'historical_capture', 'local_handoff',
  ]);
  assert.deepEqual(ordered.map(row => row.id), [
    'clear_start:fixture',
    'historical_capture:fixture:first_coast:H4',
    'local_handoff:fixture:H4',
  ]);
  assert.throws(() => __test.sortAndValidateRows(fakeRows().slice(1), expected),
    /population counts differ/);
  const duplicate = fakeRows();
  duplicate.push(structuredClone(duplicate[0]));
  assert.throws(() => __test.sortAndValidateRows(duplicate, {
    clear_start: 1, local_handoff: 2, historical_capture: 1,
  }), /duplicate corpus row id/);
}

function testPrefixValidation() {
  const phased = [update(0, 'upright'), update(2, 'ballistic_coast')];
  const actions = commandLog(phased);
  assert.deepEqual(__test.prefixFromLogs(actions, phased, 4, 'fake prefix'), phased);
  const throttleBoundaries = [update(0, 'coast', 0), update(2, 'coast', 1)];
  assert.deepEqual(__test.prefixFromLogs(
    commandLog(throttleBoundaries), throttleBoundaries, 4, 'inclusive throttle boundaries',
  ), throttleBoundaries);
  assert.throws(() => __test.prefixFromLogs(actions, [update(0), update(0)], 4, 'duplicate'),
    /duplicate, missing, or off the 60 Hz clock/);
  assert.throws(() => __test.prefixFromLogs(actions, [update(0), update(4)], 6, 'gap'),
    /duplicate, missing, or off the 60 Hz clock/);
  const nonfinite = [update(0, 'coast', Number.POSITIVE_INFINITY), update(2)];
  assert.throws(() => __test.prefixFromLogs(commandLog(nonfinite), nonfinite, 4, 'nonfinite'),
    /invalid throttle/);
  for (const throttle of [-0.01, 1.01]) {
    const outOfRange = [update(0, 'coast', throttle), update(2)];
    assert.throws(() => __test.prefixFromLogs(
      commandLog(outOfRange), outOfRange, 4, `out-of-range throttle ${throttle}`,
    ), /invalid throttle/);
  }
  const tampered = commandLog(phased);
  tampered[1].command.target_attitude_rad = 9;
  assert.throws(() => __test.prefixFromLogs(tampered, phased, 4, 'tampered'),
    /action command differs/);
  assert.throws(() => __test.prefixFromLogs(actions, phased, 3, 'odd endpoint'),
    /positive even physics step/);
  assert.throws(() => __test.prefixFromLogs(actions, phased, 6, 'short source'),
    /does not cover the prefix/);
}

function testBindings() {
  const bindings = new Map();
  __test.recordBinding(bindings, 'fixture/a.json', Buffer.from('{"a":1}\n'));
  __test.recordBinding(bindings, 'fixture/a.json', Buffer.from('{"a":1}\n'));
  __test.recordBinding(bindings, 'fixture/b.json', Buffer.from('{"b":2}\n'));
  assert.equal(bindings.size, 2);
  assert.match(bindings.get('fixture/a.json').sha256, /^[0-9a-f]{64}$/);
  assert.throws(() => __test.recordBinding(
    bindings,
    'fixture/a.json',
    Buffer.from('{"a":3}\n'),
  ), /source changed while building corpus/);
}

function testReaderIntegrity() {
  const fixtureRoot = join(TEMP_ROOT, 'reader-root');
  const dataDir = join(fixtureRoot, 'data');
  const outsidePath = join(TEMP_ROOT, 'outside.json');
  const boundPath = join(dataDir, 'bound.json');
  const escapePath = join(dataDir, 'escape.json');
  const swappedPath = join(dataDir, 'swapped.json');
  mkdirSync(dataDir, {recursive: true});
  writeFileSync(outsidePath, '{"outside":true}\n');
  writeFileSync(boundPath, '{"version":1}\n');
  writeFileSync(swappedPath, '{"version":1}\n');
  symlinkSync(outsidePath, escapePath);

  const reader = __test.createReader(fixtureRoot);
  assert.throws(() => reader.readBytes('data/../data/bound.json'), /not canonical/);
  assert.throws(() => reader.readBytes('data//bound.json'), /not canonical/);
  assert.throws(() => reader.readBytes('data\\bound.json'), /not canonical/);
  assert.throws(() => reader.readBytes('../outside.json'), /not canonical/);
  assert.throws(() => reader.readBytes(outsidePath), /not canonical/);
  assert.throws(() => reader.readBytes('data/escape.json'), /escapes repository root through filesystem resolution/);

  reader.readBytes('data/bound.json');
  writeFileSync(boundPath, '{"version":2}\n');
  assert.throws(() => reader.verifyBindings(), /source changed while building corpus/);

  const symlinkReader = __test.createReader(fixtureRoot);
  symlinkReader.readBytes('data/swapped.json');
  unlinkSync(swappedPath);
  symlinkSync(outsidePath, swappedPath);
  assert.throws(() => symlinkReader.verifyBindings(), /escapes repository root through filesystem resolution/);
}

function testRetainedCorpus() {
  const corpus = buildCorpus(ROOT);
  assert.equal(corpus.schema_id, 'waypoint_v2_nominal_characterization_corpus_v1');
  assert.equal(corpus.rows.length, 47);
  const counts = {
    clear_start: corpus.rows.filter(row => row.population === 'clear_start').length,
    local_handoff: corpus.rows.filter(row => row.population === 'local_handoff').length,
    historical_capture: corpus.rows.filter(row => row.population === 'historical_capture').length,
  };
  assert.deepEqual(counts, {clear_start: 8, local_handoff: 27, historical_capture: 12});
  const localRows = corpus.rows.filter(row => row.population === 'local_handoff');
  assert.equal(localRows.filter(row => row.expected_state.velocity_mps.y > 0).length, 19);
  assert.equal(localRows.filter(row => row.expected_state.velocity_mps.y < 0).length, 8);
  assert(localRows.every(row => row.expected_state.physics_step % 2 === 0));
  assert(localRows.every(row => row.prefix_updates.length === row.expected_state.physics_step / 2));
  assert(corpus.rows.filter(row => row.population === 'clear_start')
    .every(row => row.expected_state === null && row.prefix_updates.length === 0));
  for (const row of corpus.rows.filter(item => item.population !== 'clear_start')) {
    assert.notStrictEqual(row.expected_state, row.scenario.initial_state);
    assert.equal(row.prefix_updates[0].physics_step, 0);
    assert.equal(row.prefix_updates.at(-1).physics_step, row.expected_state.physics_step - 2);
    assert(row.prefix_updates.every(item => typeof item.phase === 'string' && item.phase.length > 0));
  }
  assert.equal(corpus.bindings.length, 107);
  assert.equal(new Set(corpus.bindings.map(binding => binding.path)).size, corpus.bindings.length);
  assert.deepEqual(corpus.bindings.map(binding => binding.path),
    corpus.bindings.map(binding => binding.path).slice().sort());
  assert(corpus.bindings.every(binding => /^[0-9a-f]{64}$/.test(binding.sha256)));
  assert.deepEqual(corpus.rows.map(row => `${row.population}\0${row.id}`),
    corpus.rows.map(row => `${row.population}\0${row.id}`).slice().sort());
}

function testCreateOnlyOutput() {
  const path = join(TEMP_ROOT, 'corpus.json');
  const first = spawnSync(process.execPath, [SCRIPT, '--output', path], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  assert.equal(first.status, 0, first.stderr || first.stdout);
  const firstBytes = readFileSync(path);
  const corpus = JSON.parse(firstBytes.toString('utf8'));
  assert.equal(corpus.rows.length, 47);
  const second = spawnSync(process.execPath, [SCRIPT, '--output', path], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  assert.equal(second.status, 1);
  assert.match(second.stderr, /EEXIST/);
  assert.deepEqual(readFileSync(path), firstBytes, 'create-only output replaced the existing corpus');
}

try {
  testMinimalSchemas();
  testPrefixValidation();
  testBindings();
  testReaderIntegrity();
  testRetainedCorpus();
  testCreateOnlyOutput();
  process.stdout.write('nominal characterization corpus checks passed\n');
} finally {
  rmSync(TEMP_ROOT, {recursive: true, force: true});
}
