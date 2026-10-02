// Read-only binding of the retained V2 starts, live local handoffs, and
// historical airborne captures for nominal-constructor characterization.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, posix, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const V2_ROOT = 'outputs/research/waypoint_v2_practical_20261001/final_policy_2_b';
const AIRBORNE_ROOT = 'outputs/research/nominal_airborne_direct_canary_20260929/final_contact_v1_b';
const PLAN_PATH = 'fixtures/research/waypoint_v2_practical_suite_plan_v1.json';
const EXPECTED_SUITE_SHA256 = '92869e10225a72e8716ad87c20fbc1ca3795bd692aa9e41d011cd9ae18cf438c';
const EXPECTED_EXPANDED_SHA256 = 'c340bb444a24f6c99197ffd18e4f38460c17e80c5e46f03bb4a49cb554f8c5a5';
const EXPECTED_AIRBORNE_IDENTITY = 'fnv1a64:46f616840230427e';
const EXPECTED_INPUT_MANIFEST_SHA256 = '6df19fbd3ebb85d5a7f4a783398aaaa48e50c74a9de49454161b78c8e47658f8';
const EXPECTED_PROTOCOL_SHA256 = 'a8ac5711fe52cb121cdde80d12dc4ef4a8e78da48a25d3a341a187dfc7609eb2';
const EXPECTED_COUNTS = Object.freeze({
  clear_start: 8,
  local_handoff: 27,
  historical_capture: 12,
});
const REQUIRED_SCENARIO_KEYS = [
  'id', 'name', 'description', 'metadata', 'initial_state', 'mission', 'seed',
  'sim', 'tags', 'vehicle', 'world',
];
const REQUIRED_SNAPSHOT_KEYS = [
  'sim_time_s', 'physics_step', 'position_m', 'velocity_mps', 'attitude_rad',
  'angular_rate_radps', 'fuel_kg', 'held_command', 'physical_outcome',
  'mission_outcome', 'end_reason', 'min_touchdown_clearance_m',
  'min_hull_clearance_m', 'max_speed_mps', 'max_abs_attitude_rad',
  'max_abs_angular_rate_radps', 'waypoint_sequence_passed',
  'waypoint_sequence_first_failure_index', 'waypoint_handoff_window_index',
];

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => Array.isArray(value) ? value.map(canonical)
  : value !== null && typeof value === 'object'
    ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
    : value;
const isRecord = value => value !== null && typeof value === 'object' && !Array.isArray(value);

function finiteTree(value, label) {
  if (typeof value === 'number') assert(Number.isFinite(value), `${label}: nonfinite number`);
  else if (Array.isArray(value)) value.forEach((item, index) => finiteTree(item, `${label}[${index}]`));
  else if (isRecord(value)) {
    for (const [key, item] of Object.entries(value)) finiteTree(item, `${label}.${key}`);
  }
}

function assertScenario(scenario, label) {
  assert(isRecord(scenario), `${label}: scenario must be an object`);
  for (const key of REQUIRED_SCENARIO_KEYS) {
    assert(Object.hasOwn(scenario, key), `${label}: scenario is missing ${key}`);
  }
  assert(typeof scenario.id === 'string' && scenario.id.length > 0, `${label}: invalid scenario id`);
  assert(Array.isArray(scenario.world?.landing_pads), `${label}: scenario lacks landing pads`);
  finiteTree(scenario, `${label}.scenario`);
}

function assertSnapshot(snapshot, label) {
  assert(isRecord(snapshot), `${label}: expected full simulation snapshot`);
  for (const key of REQUIRED_SNAPSHOT_KEYS) {
    assert(Object.hasOwn(snapshot, key), `${label}: snapshot is missing ${key}`);
  }
  assert(Number.isSafeInteger(snapshot.physics_step) && snapshot.physics_step >= 0,
    `${label}: invalid snapshot physics step`);
  assert(isRecord(snapshot.position_m) && isRecord(snapshot.velocity_mps),
    `${label}: invalid snapshot kinematics`);
  assert(isRecord(snapshot.held_command), `${label}: invalid held command`);
  finiteTree(snapshot, `${label}.snapshot`);
}

function assertCommand(command, label) {
  assert(isRecord(command), `${label}: command must be an object`);
  assert(Number.isFinite(command.throttle_frac)
    && command.throttle_frac >= 0 && command.throttle_frac <= 1,
  `${label}: invalid throttle`);
  assert(Number.isFinite(command.target_attitude_rad), `${label}: invalid target attitude`);
}

function assertPhasedSchedule(updates, label) {
  assert(Array.isArray(updates), `${label}: phase-tagged updates must be an array`);
  updates.forEach((update, index) => {
    assert(isRecord(update), `${label}: invalid update ${index}`);
    assert(Number.isSafeInteger(update.physics_step) && update.physics_step === index * 2,
      `${label}: update ${index} is duplicate, missing, or off the 60 Hz clock`);
    assert(typeof update.phase === 'string' && update.phase.length > 0,
      `${label}: update ${index} has no phase`);
    assertCommand(update.command, `${label}: update ${index}`);
  });
}

function assertActionParity(actions, phasedUpdates, label) {
  assert(Array.isArray(actions), `${label}: action log must be an array`);
  assertPhasedSchedule(phasedUpdates, label);
  assert.equal(actions.length, phasedUpdates.length, `${label}: action/update counts differ`);
  actions.forEach((action, index) => {
    assert(isRecord(action), `${label}: invalid action ${index}`);
    assert(Number.isSafeInteger(action.physics_step), `${label}: invalid action tick ${index}`);
    assertCommand(action.command, `${label}: action ${index}`);
    const update = phasedUpdates[index];
    assert.equal(action.physics_step, update.physics_step,
      `${label}: action tick differs from phase schedule at index ${index}`);
    assert.deepStrictEqual(action.command, update.command,
      `${label}: action command differs from phase schedule at tick ${action.physics_step}`);
  });
}

function prefixFromLogs(actions, phasedUpdates, handoffStep, label) {
  assert(Number.isSafeInteger(handoffStep) && handoffStep > 0 && handoffStep % 2 === 0,
    `${label}: handoff must be a positive even physics step`);
  assertActionParity(actions, phasedUpdates, label);
  const prefix = [];
  for (let index = 0; index < actions.length && actions[index].physics_step < handoffStep; index++) {
    const action = actions[index];
    const update = phasedUpdates[index];
    prefix.push({
      physics_step: action.physics_step,
      phase: update.phase,
      command: structuredClone(action.command),
    });
  }
  assert.equal(prefix.length, handoffStep / 2,
    `${label}: full-source command log does not cover the prefix through H${handoffStep}`);
  prefix.forEach((update, index) => {
    assert.equal(update.physics_step, index * 2,
      `${label}: prefix is not contiguous at update ${index}`);
  });
  return prefix;
}

function compareValue(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

function sortAndValidateRows(rows, expectedCounts = EXPECTED_COUNTS) {
  assert(Array.isArray(rows), 'corpus rows must be an array');
  const counts = Object.fromEntries(Object.keys(expectedCounts).map(population => [population, 0]));
  const ids = new Set();
  for (const row of rows) {
    assert(isRecord(row), 'corpus row must be an object');
    const keys = [
      'id', 'population', 'label', 'scenario', 'source_pad_id', 'target_pad_id',
      'absolute_deadline_physics_step', 'expected_state', 'prefix_updates',
    ].sort();
    assert.deepStrictEqual(Object.keys(row).sort(), keys, `${row.id ?? 'row'}: unexpected row fields`);
    assert(Object.hasOwn(counts, row.population), `${row.id}: unknown population ${row.population}`);
    assert(typeof row.id === 'string' && row.id.length > 0, 'row id must be nonempty');
    assert(!ids.has(row.id), `duplicate corpus row id ${row.id}`);
    ids.add(row.id);
    counts[row.population]++;
    assert(typeof row.label === 'string' && row.label.length > 0, `${row.id}: label must be nonempty`);
    assertScenario(row.scenario, row.id);
    assert(typeof row.source_pad_id === 'string' && row.source_pad_id.length > 0,
      `${row.id}: source pad id must be nonempty`);
    assert(typeof row.target_pad_id === 'string' && row.target_pad_id.length > 0,
      `${row.id}: target pad id must be nonempty`);
    const padIds = new Set(row.scenario.world.landing_pads.map(pad => pad.id));
    assert(padIds.has(row.source_pad_id), `${row.id}: source pad missing from scenario`);
    assert(padIds.has(row.target_pad_id), `${row.id}: target pad missing from scenario`);
    assert(Number.isSafeInteger(row.absolute_deadline_physics_step)
      && row.absolute_deadline_physics_step > 0,
    `${row.id}: invalid absolute deadline`);
    assert(Array.isArray(row.prefix_updates), `${row.id}: prefix updates must be an array`);

    if (row.population === 'clear_start') {
      assert.equal(row.expected_state, null, `${row.id}: clear start must begin from a fresh sim`);
      assert.equal(row.prefix_updates.length, 0, `${row.id}: clear start must not have a prefix`);
    } else {
      assertSnapshot(row.expected_state, row.id);
      assert.notStrictEqual(row.expected_state, row.scenario.initial_state,
        `${row.id}: evidence snapshot must remain separate from scenario input`);
      const handoffStep = row.expected_state.physics_step;
      assert(handoffStep < row.absolute_deadline_physics_step,
        `${row.id}: snapshot is at or beyond the absolute deadline`);
      assertPhasedSchedule(row.prefix_updates, row.id);
      assert.equal(row.prefix_updates.length, handoffStep / 2,
        `${row.id}: prefix length does not reach the snapshot boundary`);
    }
  }
  assert.deepStrictEqual(counts, expectedCounts, 'retained corpus population counts differ');
  return rows.slice().sort((a, b) => compareValue(a.population, b.population)
    || compareValue(a.id, b.id));
}

function bindingFromBytes(path, bytes) {
  assert(typeof path === 'string' && path.length > 0, 'binding path must be nonempty');
  return {path, sha256: sha256(bytes)};
}

function recordBinding(bindings, path, bytes) {
  const binding = bindingFromBytes(path, bytes);
  const previous = bindings.get(path);
  assert(!previous || previous.sha256 === binding.sha256,
    `source changed while building corpus: ${path}`);
  bindings.set(path, binding);
}

function createReader(repoRoot) {
  const root = resolve(repoRoot);
  const realRoot = realpathSync(root);
  const bindings = new Map();
  const bytesByPath = new Map();
  const jsonByPath = new Map();
  const assertCanonicalRepoPath = path => {
    assert(typeof path === 'string' && path.length > 0,
      'source path must be a nonempty repository-relative path');
    assert(!path.includes('\\') && !path.includes('\0') && !path.includes(':')
      && !posix.isAbsolute(path) && !/^[A-Za-z]:/.test(path),
    `source path is not canonical and repository-relative: ${path}`);
    assert(path.split('/').every(part => part.length > 0 && part !== '.' && part !== '..')
      && posix.normalize(path) === path,
    `source path is not canonical and repository-relative: ${path}`);
  };
  const absolutePath = path => {
    assertCanonicalRepoPath(path);
    const absolute = resolve(root, ...path.split('/'));
    const rel = relative(root, absolute);
    assert(rel && rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel),
      `source path escapes repository root: ${path}`);
    const realTarget = realpathSync(absolute);
    const realRel = relative(realRoot, realTarget);
    assert(realRel && realRel !== '..' && !realRel.startsWith(`..${sep}`)
      && !isAbsolute(realRel),
    `source path escapes repository root through filesystem resolution: ${path}`);
    return realTarget;
  };
  const readBytes = path => {
    if (!bytesByPath.has(path)) {
      const bytes = readFileSync(absolutePath(path));
      bytesByPath.set(path, bytes);
      recordBinding(bindings, path.split(sep).join('/'), bytes);
    }
    return bytesByPath.get(path);
  };
  const readJson = path => {
    if (!jsonByPath.has(path)) {
      try {
        jsonByPath.set(path, JSON.parse(readBytes(path).toString('utf8')));
      } catch (error) {
        throw new Error(`${path}: could not parse JSON: ${error.message}`, {cause: error});
      }
    }
    return jsonByPath.get(path);
  };
  const verifyBindings = () => {
    for (const binding of bindings.values()) {
      const currentBytes = readFileSync(absolutePath(binding.path));
      assert.equal(sha256(currentBytes), binding.sha256,
        `source changed while building corpus: ${binding.path}`);
    }
  };
  return {root, bindings, readBytes, readJson, verifyBindings};
}

function assertV2Summary(summary, planBytes) {
  assert.equal(summary.schema_id, 'waypoint_v2_practical_suite_run_v1');
  assert.equal(summary.status, 'completed');
  assert.equal(summary.case_count, 32);
  assert.equal(summary.policy_version, 2);
  assert.equal(summary.selected_case_id, null);
  assert.equal(summary.suite_sha256, EXPECTED_SUITE_SHA256);
  assert.equal(sha256(planBytes), summary.suite_sha256, 'suite plan SHA-256 differs');
  assert.equal(summary.expanded_scenarios_sha256, EXPECTED_EXPANDED_SHA256);
  assert.equal(summary.provenance?.unchanged, true);
  assert.deepStrictEqual(summary.provenance.before, summary.provenance.after,
    'V2 suite provenance changed during the retained run');
  assert(Array.isArray(summary.cases) && summary.cases.length === 32,
    'V2 suite case ledger must contain all 32 cases');
}

function readV2Originals(reader, plan) {
  const originals = new Map();
  for (const [sourceName, source] of Object.entries(plan.source_manifests ?? {})) {
    assert(isRecord(source) && typeof source.path === 'string', `plan source ${sourceName} is invalid`);
    const bytes = reader.readBytes(source.path);
    assert.equal(sha256(bytes), source.sha256, `plan source hash differs: ${source.path}`);
    const manifest = reader.readJson(source.path);
    assert(Array.isArray(manifest.cases), `${source.path}: missing cases`);
    for (const original of manifest.cases) {
      assert(!originals.has(original.case_id), `duplicate V2 base case ${original.case_id}`);
      originals.set(original.case_id, original);
    }
  }
  return originals;
}

function assertSourcePads(scenario, sourcePadId, targetPadId, label) {
  const pads = scenario.world.landing_pads;
  assert(pads.some(pad => pad.id === sourcePadId), `${label}: source pad ${sourcePadId} is missing`);
  assert(pads.some(pad => pad.id === targetPadId), `${label}: target pad ${targetPadId} is missing`);
  assert.equal(scenario.mission?.goal?.target_pad_id, targetPadId,
    `${label}: target pad differs from scenario mission`);
}

function normalizeScenarioCopy(scenario) {
  const normalized = structuredClone(scenario);
  if (normalized.mission && !Object.hasOwn(normalized.mission, 'transfer_route')) {
    normalized.mission.transfer_route = null;
  }
  return normalized;
}

function v2PhasedUpdates(flight, label) {
  const segments = flight.segments ?? [];
  assert(Array.isArray(segments), `${label}: flight segments must be an array`);
  const updates = [];
  let priorState = null;
  for (const [index, segment] of segments.entries()) {
    assert(isRecord(segment), `${label}: invalid segment ${index}`);
    assertSnapshot(segment.entry_state, `${label}.segments[${index}].entry_state`);
    assertSnapshot(segment.end_state, `${label}.segments[${index}].end_state`);
    assert.equal(segment.start_physics_step, segment.entry_state.physics_step,
      `${label}: segment entry clock differs`);
    assert.equal(segment.end_physics_step, segment.end_state.physics_step,
      `${label}: segment endpoint clock differs`);
    assert(segment.end_physics_step > segment.start_physics_step,
      `${label}: empty or reversed segment`);
    if (priorState !== null) {
      assert.deepStrictEqual(segment.entry_state, priorState,
        `${label}: segment state continuity differs at ${index}`);
    } else {
      assert.equal(segment.start_physics_step, 0, `${label}: first segment does not start at tick 0`);
      const initial = flight.cycles?.[0]?.current_state;
      assertSnapshot(initial, `${label}.cycles[0].current_state`);
      assert.deepStrictEqual(segment.entry_state, initial,
        `${label}: first segment entry differs from the live initial state`);
    }
    assert(Array.isArray(segment.updates), `${label}: segment ${index} lacks updates`);
    assert.equal(segment.updates.length,
      Math.ceil((segment.end_physics_step - segment.start_physics_step) / 2),
      `${label}: segment ${index} update coverage differs from its interval`);
    segment.updates.forEach((update, updateIndex) => {
      assert.equal(update.physics_step, segment.start_physics_step + updateIndex * 2,
        `${label}: segment ${index} has a gap or duplicate update`);
      assert(typeof update.phase === 'string' && update.phase.length > 0,
        `${label}: segment ${index} update lacks phase`);
      assertCommand(update.command, `${label}: segment ${index} update ${updateIndex}`);
      updates.push(update);
    });
    priorState = segment.end_state;
  }

  const actions = flight.ordinary_flight?.actions;
  if (Array.isArray(actions)) assertActionParity(actions, updates, label);
  else assert.equal(updates.length, 0, `${label}: segments lack their official action log`);
  return {segments, updates, actions: actions ?? []};
}

function addRow(rows, row) {
  rows.push({
    id: row.id,
    population: row.population,
    label: row.label,
    scenario: structuredClone(row.scenario),
    source_pad_id: row.source_pad_id,
    target_pad_id: row.target_pad_id,
    absolute_deadline_physics_step: row.absolute_deadline_physics_step,
    expected_state: row.expected_state === null ? null : structuredClone(row.expected_state),
    prefix_updates: row.prefix_updates.map(update => structuredClone(update)),
  });
}

function collectV2Rows(reader, summary, plan, originals) {
  const planCases = plan.cases;
  assert(Array.isArray(planCases) && planCases.length === 32, 'V2 suite plan must contain 32 cases');
  assert.equal(summary.cases.length, planCases.length);
  const summaryById = new Map();
  summary.cases.forEach((entry, index) => {
    const recipe = planCases[index];
    assert.equal(entry.case_id, recipe.id, `V2 suite order differs at case ${index}`);
    assert.equal(entry.group, recipe.group, `${entry.case_id}: suite group differs from plan`);
    assert(!summaryById.has(entry.case_id), `duplicate V2 suite case ${entry.case_id}`);
    summaryById.set(entry.case_id, entry);
  });

  const scenarios = new Map();
  const runData = new Map();
  const unsupportedIds = [];
  const v2Deadlines = new Set();
  for (const recipe of planCases) {
    const id = recipe.id;
    const summaryEntry = summaryById.get(id);
    const original = originals.get(recipe.base);
    assert(original, `${id}: source base ${recipe.base} is missing`);
    assert.equal(summaryEntry.flight_path, `runs/${id}/flight.json`, `${id}: unexpected flight path`);
    assert.equal(summaryEntry.summary_path, `runs/${id}/summary.json`, `${id}: unexpected summary path`);

    const inputPath = `${V2_ROOT}/inputs/${id}.json`;
    const inputScenario = reader.readJson(inputPath);
    const runScenario = reader.readJson(`${V2_ROOT}/runs/${id}/scenario.json`);
    assertScenario(inputScenario, `${id}.input`);
    assertScenario(runScenario, `${id}.run_scenario`);
    assert.deepStrictEqual(normalizeScenarioCopy(runScenario), normalizeScenarioCopy(inputScenario),
      `${id}: retained scenario copies differ`);
    assert.equal(inputScenario.id, id, `${id}: scenario id differs`);
    assert.deepStrictEqual(inputScenario.initial_state, original.scenario.initial_state,
      `${id}: retained initial state differs from the base input`);

    const sourcePadId = original.source_pad_id;
    const targetPadId = original.target_pad_id;
    assertSourcePads(inputScenario, sourcePadId, targetPadId, id);
    scenarios.set(id, {scenario: inputScenario, sourcePadId, targetPadId});

    const flight = reader.readJson(`${V2_ROOT}/${summaryEntry.flight_path}`);
    assert.equal(flight.policy?.policy_id, 'piecewise_local_clearing_v2_policy_2',
      `${id}: flight is not policy 2`);
    assert.equal(flight.planning_stop, summaryEntry.result?.planning_stop,
      `${id}: suite and flight stop reasons differ`);
    if (flight.planning_stop === 'unsupported') unsupportedIds.push(id);
    if (Number.isSafeInteger(flight.absolute_deadline_physics_step)) {
      v2Deadlines.add(flight.absolute_deadline_physics_step);
    }
    if (flight.manifest) {
      assert.equal(flight.manifest.scenario_id, inputScenario.id, `${id}: manifest scenario differs`);
      assert.equal(flight.manifest.scenario_name, inputScenario.name, `${id}: manifest name differs`);
      assert.equal(flight.manifest.scenario_seed, inputScenario.seed, `${id}: manifest seed differs`);
      assert.deepStrictEqual(flight.manifest.scenario_tags, inputScenario.tags,
        `${id}: manifest tags differ`);
    }
    const commandData = v2PhasedUpdates(flight, id);
    assert.equal(flight.correction_count,
      commandData.segments.filter(segment => segment.kind === 'local_correction').length,
      `${id}: correction count differs from actual local segments`);
    runData.set(id, {flight, commandData, group: recipe.group, family: recipe.family});
  }
  assert.deepStrictEqual(unsupportedIds.sort(), ['v2_diag_lunar_gravity', 'v2_diag_other_vehicle'],
    'supported and unsupported diagnostic input population differs');
  assert.deepStrictEqual([...v2Deadlines], [9600],
    'V2 cases do not retain one original absolute mission deadline');

  const expanded = planCases.map(recipe => {
    const scenarioData = scenarios.get(recipe.id);
    return {
      case_id: recipe.id,
      source_pad_id: scenarioData.sourcePadId,
      target_pad_id: scenarioData.targetPadId,
      scenario: scenarioData.scenario,
    };
  });
  assert.equal(sha256(Buffer.from(JSON.stringify(canonical(expanded)))), EXPECTED_EXPANDED_SHA256,
    'retained V2 scenario/pad population differs from frozen inputs');

  const rows = [];
  let localCount = 0;
  let ordinaryHandoffCount = 0;
  let diagnosticHandoffCount = 0;
  let ascendingCount = 0;
  let descendingCount = 0;
  for (const recipe of planCases) {
    const id = recipe.id;
    const scenarioData = scenarios.get(id);
    const {flight, commandData, group, family} = runData.get(id);
    if (group === 'clear') {
      const startState = flight.cycles?.[0]?.current_state;
      assertSnapshot(startState, `${id}: clear start state`);
      assert(Number.isSafeInteger(flight.absolute_deadline_physics_step)
        && flight.absolute_deadline_physics_step > 0, `${id}: missing original absolute deadline`);
      assert.equal(startState.physics_step, 0, `${id}: clear control does not start at tick 0`);
      assert.deepStrictEqual(startState.position_m, scenarioData.scenario.initial_state.position_m,
        `${id}: fresh launch position differs`);
      assert.deepStrictEqual(startState.velocity_mps, scenarioData.scenario.initial_state.velocity_mps,
        `${id}: fresh launch velocity differs`);
      addRow(rows, {
        id: `clear_start:${id}`,
        population: 'clear_start',
        label: `${id} source-rest start at physics step 0`,
        scenario: scenarioData.scenario,
        source_pad_id: scenarioData.sourcePadId,
        target_pad_id: scenarioData.targetPadId,
        absolute_deadline_physics_step: flight.absolute_deadline_physics_step,
        expected_state: null,
        prefix_updates: [],
      });
    }

    const localSegments = commandData.segments.filter(segment => segment.kind === 'local_correction');
    if (group === 'clear') assert.equal(localSegments.length, 0, `${id}: clear control has a correction`);
    for (const [ordinal, segment] of localSegments.entries()) {
      const handoffStep = segment.end_physics_step;
      const handoffState = segment.end_state;
      assert(Number.isSafeInteger(flight.absolute_deadline_physics_step)
        && flight.absolute_deadline_physics_step > handoffStep,
      `${id}: local handoff lacks the original absolute deadline`);
      assert.equal(handoffState.physical_outcome, 'flying', `${id}: local handoff is not airborne`);
      assert.equal(handoffState.mission_outcome, 'in_progress', `${id}: local handoff is terminal`);
      const prefixUpdates = prefixFromLogs(
        commandData.actions,
        commandData.updates,
        handoffStep,
        `${id}: local handoff H${handoffStep}`,
      );
      const verticalVelocity = handoffState.velocity_mps.y;
      if (verticalVelocity > 0) ascendingCount++;
      else if (verticalVelocity < 0) descendingCount++;
      else assert.fail(`${id}: local handoff has no ascending/descending classification`);
      localCount++;
      if (group === 'ordinary') ordinaryHandoffCount++;
      if (group === 'diagnostic') diagnosticHandoffCount++;
      addRow(rows, {
        id: `local_handoff:${id}:H${handoffStep}`,
        population: 'local_handoff',
        label: `${group} ${family} local handoff for ${id} at H${handoffStep}`,
        scenario: scenarioData.scenario,
        source_pad_id: scenarioData.sourcePadId,
        target_pad_id: scenarioData.targetPadId,
        absolute_deadline_physics_step: flight.absolute_deadline_physics_step,
        expected_state: handoffState,
        prefix_updates: prefixUpdates,
      });
      assert.equal(ordinal, localSegments.slice(0, ordinal + 1).length - 1,
        `${id}: local handoff order is unstable`);
    }
  }
  assert.equal(localCount, 27, 'retained V2 local handoff count differs');
  assert.equal(ordinaryHandoffCount, 25, 'ordinary local handoff count differs');
  assert.equal(diagnosticHandoffCount, 2, 'diagnostic local handoff count differs');
  assert.equal(ascendingCount, 19, 'ascending local handoff count differs');
  assert.equal(descendingCount, 8, 'descending local handoff count differs');
  const diagnosticIds = [...runData].filter(([, value]) => value.group === 'diagnostic'
    && value.commandData.segments.some(segment => segment.kind === 'local_correction'))
    .map(([id]) => id);
  assert.deepStrictEqual(diagnosticIds, ['v2_diag_high_700'],
    'diagnostic handoffs differ from the retained policy 2 population');
  return rows;
}

function collectHistoricalRows(reader) {
  const summaryPath = `${AIRBORNE_ROOT}/summary.json`;
  const summary = reader.readJson(summaryPath);
  assert.equal(summary.schema_id, 'nominal_airborne_direct_canary_v1');
  assert.equal(summary.schema_version, 1);
  assert.equal(summary.identity, EXPECTED_AIRBORNE_IDENTITY);
  assert.equal(summary.case_count, 4);
  assert.equal(summary.passed, true);
  assert.equal(summary.input_manifest_sha256, EXPECTED_INPUT_MANIFEST_SHA256);
  assert.equal(summary.protocol_sha256, EXPECTED_PROTOCOL_SHA256);
  const manifestBytes = reader.readBytes(summary.input_manifest_path);
  assert.equal(sha256(manifestBytes), EXPECTED_INPUT_MANIFEST_SHA256,
    'historical input manifest SHA-256 differs');
  const protocolBytes = reader.readBytes(summary.protocol_path);
  assert.equal(sha256(protocolBytes), EXPECTED_PROTOCOL_SHA256,
    'historical canary protocol SHA-256 differs');
  assert.deepStrictEqual(summary.input_binding, {
    relative_path: summary.input_manifest_path,
    sha256: summary.input_manifest_sha256,
  }, 'historical input binding differs');

  const inputManifest = reader.readJson(summary.input_manifest_path);
  assert(Array.isArray(inputManifest.cases), 'historical input manifest lacks cases');
  const inputById = new Map(inputManifest.cases.map(entry => [entry.case_id, entry]));
  const rows = [];
  const expectedRoles = ['first_coast', 'near_apex', 'last_coast'];
  const caseIds = new Set();
  for (const summaryCase of summary.cases) {
    assert.equal(summaryCase.status, 'pass', `${summaryCase.case_id}: historical canary did not pass`);
    assert(!caseIds.has(summaryCase.case_id), `duplicate historical case ${summaryCase.case_id}`);
    caseIds.add(summaryCase.case_id);
    assert.equal(summaryCase.artifact_path, `cases/${summaryCase.case_id}.json`,
      `${summaryCase.case_id}: unexpected historical artifact path`);
    const casePath = `${AIRBORNE_ROOT}/${summaryCase.artifact_path}`;
    const historicalCase = reader.readJson(casePath);
    assert.equal(historicalCase.schema_id, 'nominal_airborne_direct_canary_case_v1');
    assert.equal(historicalCase.status, 'pass', `${summaryCase.case_id}: case status differs`);
    assert.equal(historicalCase.case_id, summaryCase.case_id);
    assert.equal(historicalCase.identity, summaryCase.identity);
    assert.equal(historicalCase.request?.scenario?.metadata?.case_id, summaryCase.case_id,
      `${summaryCase.case_id}: embedded scenario lineage differs`);
    const original = inputById.get(summaryCase.case_id);
    assert(original, `${summaryCase.case_id}: historical base input is missing`);
    assert.deepStrictEqual(
      normalizeScenarioCopy(historicalCase.request.scenario),
      normalizeScenarioCopy(original.scenario),
      `${summaryCase.case_id}: embedded scenario differs from the bound input manifest`);
    assert.equal(historicalCase.request.source_pad_id, original.source_pad_id,
      `${summaryCase.case_id}: source pad differs from the bound input manifest`);
    assert.equal(historicalCase.request.target_pad_id, original.target_pad_id,
      `${summaryCase.case_id}: target pad differs from the bound input manifest`);
    const scenario = historicalCase.request.scenario;
    assertScenario(scenario, summaryCase.case_id);
    assertSourcePads(scenario, original.source_pad_id, original.target_pad_id, summaryCase.case_id);
    const analyticalPolicy = historicalCase.request.policy?.analytical_policy;
    assert(isRecord(analyticalPolicy), `${summaryCase.case_id}: missing analytical policy`);
    const expectedDeadline = Math.floor((Math.min(
      analyticalPolicy.maximum_mission_time_s,
      scenario.sim.max_time_s,
    ) - analyticalPolicy.mission_time_reserve_s) * analyticalPolicy.physics_hz);
    assert(Number.isSafeInteger(expectedDeadline) && expectedDeadline > 0,
      `${summaryCase.case_id}: invalid original mission budget`);
    assert.equal(historicalCase.baseline_execution?.passed, true,
      `${summaryCase.case_id}: historical baseline execution did not pass`);
    assert.equal(historicalCase.baseline_execution?.exact_command_and_clock_parity, true,
      `${summaryCase.case_id}: baseline command/clock parity is missing`);
    assert.equal(historicalCase.baseline_execution?.ordinary_action_replay_parity, true,
      `${summaryCase.case_id}: baseline action replay parity is missing`);

    const actions = historicalCase.baseline_ordinary_run?.actions;
    const phasedUpdates = historicalCase.baseline_decision?.program?.updates;
    assertActionParity(actions, phasedUpdates, summaryCase.case_id);
    assert.equal(historicalCase.captures?.length, expectedRoles.length,
      `${summaryCase.case_id}: historical capture count differs`);
    const captureRoles = historicalCase.captures.map(capture => capture.role);
    assert.deepStrictEqual(captureRoles, expectedRoles,
      `${summaryCase.case_id}: capture role order differs`);
    for (const capture of historicalCase.captures) {
      assert.equal(capture.full_state_snapshot?.physics_step, capture.physics_step,
        `${summaryCase.case_id}/${capture.role}: snapshot tick differs`);
      assertSnapshot(capture.full_state_snapshot, `${summaryCase.case_id}/${capture.role}`);
      assert.equal(capture.full_state_snapshot.physical_outcome, 'flying',
        `${summaryCase.case_id}/${capture.role}: capture is not airborne`);
      assert.equal(capture.full_state_snapshot.mission_outcome, 'in_progress',
        `${summaryCase.case_id}/${capture.role}: capture is terminal`);
      const deadline = capture.search?.absolute_deadline_physics_step;
      assert.equal(deadline, expectedDeadline,
        `${summaryCase.case_id}/${capture.role}: deadline differs from the original mission budget`);
      assert(deadline > capture.physics_step,
        `${summaryCase.case_id}/${capture.role}: capture is at or beyond the original deadline`);
      const prefixUpdates = prefixFromLogs(
        actions,
        phasedUpdates,
        capture.physics_step,
        `${summaryCase.case_id}/${capture.role}`,
      );
      rows.push({
        id: `historical_capture:${summaryCase.case_id}:${capture.role}:H${capture.physics_step}`,
        population: 'historical_capture',
        label: `${summaryCase.case_id} ${capture.role} capture at H${capture.physics_step}`,
        scenario,
        source_pad_id: original.source_pad_id,
        target_pad_id: original.target_pad_id,
        absolute_deadline_physics_step: deadline,
        expected_state: capture.full_state_snapshot,
        prefix_updates: prefixUpdates,
      });
    }
  }
  assert.equal(caseIds.size, 4, 'historical canary case population differs');
  return rows;
}

export function buildCorpus(repoRoot = REPO_ROOT) {
  const reader = createReader(repoRoot);
  const summary = reader.readJson(`${V2_ROOT}/suite-summary.json`);
  const planBytes = reader.readBytes(PLAN_PATH);
  assertV2Summary(summary, planBytes);
  const plan = reader.readJson(PLAN_PATH);
  assert.equal(plan.schema_id, 'waypoint_v2_practical_suite_plan_v1');
  assert.equal(plan.status, 'design_only_not_flight_accepted');
  const originals = readV2Originals(reader, plan);
  const rows = collectV2Rows(reader, summary, plan, originals);
  rows.push(...collectHistoricalRows(reader));
  const orderedRows = sortAndValidateRows(rows);
  reader.verifyBindings();
  const bindings = [...reader.bindings.values()].sort((a, b) => compareValue(a.path, b.path));
  return {
    schema_id: 'waypoint_v2_nominal_characterization_corpus_v1',
    bindings,
    rows: orderedRows,
  };
}

function writeExclusive(path, contents) {
  writeFileSync(path, contents, {flag: 'wx'});
}

function parseArguments(args) {
  if (args.length === 1 && (args[0] === '--help' || args[0] === '-h')) {
    return {help: true};
  }
  if (args.length === 0) return {output: null};
  assert.equal(args.length, 2, 'usage: waypoint_v2_nominal_characterization_corpus.mjs [--output NEW_FILE]');
  assert.equal(args[0], '--output', 'usage: waypoint_v2_nominal_characterization_corpus.mjs [--output NEW_FILE]');
  assert(args[1].length > 0, '--output needs a file path');
  return {output: resolve(args[1])};
}

function main() {
  const args = parseArguments(process.argv.slice(2));
  if (args.help) {
    process.stdout.write('Usage: node scripts/waypoint_v2_nominal_characterization_corpus.mjs [--output NEW_FILE]\n');
    return;
  }
  const output = `${JSON.stringify(buildCorpus(), null, 2)}\n`;
  if (args.output) writeExclusive(args.output, output);
  else process.stdout.write(output);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error.stack ?? error}\n`);
    process.exitCode = 1;
  }
}

// Small pure hooks let the focused contract tests exercise malformed records
// without manufacturing another retained flight corpus.
export const __test = Object.freeze({
  assertActionParity,
  bindingFromBytes,
  prefixFromLogs,
  recordBinding,
  sortAndValidateRows,
  createReader,
  writeExclusive,
});
