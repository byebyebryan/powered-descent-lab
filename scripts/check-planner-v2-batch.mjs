#!/usr/bin/env node
// Independent acceptance audit for one captured native Planner V2 batch.
// This reads frozen inputs and accepted captures, writes only to a new check
// directory, and never starts a simulation or changes the selected report site.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  realpathSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import {dirname, isAbsolute, join, relative, resolve, sep} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {isDeepStrictEqual} from 'node:util';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PACK_REL = 'fixtures/packs/planner_v2_lab_suite.json';
const PRACTICAL_REL = 'fixtures/research/waypoint_v2_practical_suite_plan_v1.json';
const FRESH_REL = 'fixtures/research/waypoint_v2_fresh_terrain_inputs_v1.json';
const DEFAULT_PRACTICAL_ARCHIVE = 'outputs/research/waypoint_v2_airborne_integration_20261002/final_hardened_policy_3_a';
const DEFAULT_FRESH_ARCHIVE = 'outputs/research/waypoint_v2_fresh_terrain_readiness_20261003/fresh_a';
const EXPECTED_SOURCES = [
  {
    source_id: 'practical_suite',
    kind: 'waypoint_v2_practical_suite_plan_v1',
    path: PRACTICAL_REL,
    sha256: '92869e10225a72e8716ad87c20fbc1ca3795bd692aa9e41d011cd9ae18cf438c',
  },
  {
    source_id: 'fresh_terrain_inputs',
    kind: 'waypoint_v2_fresh_terrain_inputs_v1',
    path: FRESH_REL,
    sha256: '554b406f62387e9598d89e53a26ed705b2fce5380b4e7a216f92f3003950cd9c',
  },
];
const TIMING_POINTERS = [
  '/timings/planning_s',
  '/timings/execution_s',
  '/timings/replay_s',
];
// These are the only historically observed output-location fields. They are
// ignored only at the root of flight.json and only when both values are paths.
const PATH_POINTERS = ['/scenario_path', '/output_dir'];
const EXPECTED_GROUP_COUNTS = {
  clear_count: 11,
  ordinary_count: 16,
  additional_terrain_count: 9,
  diagnostic_count: 8,
};
const SUMMARY_COUNT_KEYS = [
  ...Object.keys(EXPECTED_GROUP_COUNTS),
  'direct_landing_count',
  'corrected_landing_count',
  'valid_landing_count',
  'non_landing_count',
  'diagnostic_landing_count',
  'diagnostic_non_landing_count',
  'simulation_unverified_count',
  'unsupported_count',
  'crash_count',
  'integrity_passed_count',
  'integrity_failed_count',
  'final_source_replay_passed_count',
  'final_source_replay_failed_count',
  'initially_blocked_count',
];

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const readJson = path => JSON.parse(readFileSync(path, 'utf8'));
const isRecord = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const isSha256 = value => typeof value === 'string' && /^[a-f0-9]{64}$/i.test(value);
const inside = (parent, child) => {
  const rel = relative(parent, child);
  return rel === '' || (!rel.startsWith(`..${sep}`) && rel !== '..' && !isAbsolute(rel));
};

function fail(message) {
  throw new Error(message);
}

function check(condition, message) {
  if (!condition) fail(message);
}

function firstDifference(actual, expected, path = '$') {
  if (Object.is(actual, expected)) return null;
  if (Array.isArray(actual) || Array.isArray(expected)) {
    if (!Array.isArray(actual) || !Array.isArray(expected)) return `${path}: array/value type differs`;
    if (actual.length !== expected.length) return `${path}: array length ${actual.length} != ${expected.length}`;
    for (let index = 0; index < actual.length; index += 1) {
      const difference = firstDifference(actual[index], expected[index], `${path}[${index}]`);
      if (difference) return difference;
    }
    return null;
  }
  if (isRecord(actual) || isRecord(expected)) {
    if (!isRecord(actual) || !isRecord(expected)) return `${path}: object/value type differs`;
    const actualKeys = Object.keys(actual).sort();
    const expectedKeys = Object.keys(expected).sort();
    if (!isDeepStrictEqual(actualKeys, expectedKeys)) {
      const missing = expectedKeys.filter(key => !Object.hasOwn(actual, key));
      const extra = actualKeys.filter(key => !Object.hasOwn(expected, key));
      return `${path}: object keys differ (missing ${missing.join(', ') || 'none'}; extra ${extra.join(', ') || 'none'})`;
    }
    for (const key of actualKeys) {
      const difference = firstDifference(actual[key], expected[key], `${path}.${key}`);
      if (difference) return difference;
    }
    return null;
  }
  return `${path}: ${String(actual)} != ${String(expected)}`;
}

export function assertTypedEqual(actual, expected, label) {
  if (!isDeepStrictEqual(actual, expected)) {
    fail(`${label}: exact typed JSON differs at ${firstDifference(actual, expected)}`);
  }
}

function resolveCaptureFile(captureRoot, value, label, {required = true} = {}) {
  if (value === null || value === undefined || value === '') {
    check(!required, `${label}: missing required relative path`);
    return null;
  }
  check(typeof value === 'string', `${label}: path must be a string`);
  const normalized = value.replaceAll('\\', '/');
  check(!normalized.startsWith('/') && !/^[A-Za-z]:/.test(normalized), `${label}: absolute path is not allowed`);
  const pieces = normalized.split('/');
  check(pieces.length > 0 && pieces.every(piece => piece && piece !== '.' && piece !== '..'), `${label}: unsafe relative path ${value}`);
  const path = resolve(captureRoot, ...pieces);
  check(inside(captureRoot, path), `${label}: path escapes the capture root`);
  if (required) check(lstatSync(path, {throwIfNoEntry: false}), `${label}: missing file ${value}`);
  return path;
}

function hashTree(root, {includeInventory = false} = {}) {
  const absolute = realpathSync(root);
  check(statSync(absolute).isDirectory(), `${root}: expected a directory`);
  const inventory = [];
  function visit(directory, prefix = '') {
    for (const entry of readdirSync(directory, {withFileTypes: true}).sort((a, b) => a.name.localeCompare(b.name))) {
      const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
      const path = join(directory, entry.name);
      const info = lstatSync(path);
      if (info.isDirectory()) {
        visit(path, rel);
      } else if (info.isSymbolicLink()) {
        const target = readlinkSync(path);
        inventory.push({path: rel, type: 'symlink', size: Buffer.byteLength(target), sha256: sha256(Buffer.from(target))});
      } else if (info.isFile()) {
        const bytes = readFileSync(path);
        inventory.push({path: rel, type: 'file', size: bytes.byteLength, sha256: sha256(bytes)});
      } else {
        fail(`${rel}: unsupported filesystem entry in evidence tree`);
      }
    }
  }
  visit(absolute);
  const treeHash = createHash('sha256');
  for (const item of inventory) {
    treeHash.update(item.path).update('\0').update(item.type).update('\0')
      .update(String(item.size)).update('\0').update(item.sha256).update('\0');
  }
  return {sha256: treeHash.digest('hex'), file_count: inventory.length, ...(includeInventory ? {inventory} : {})};
}

function readArchive(root, expectedSchema, sourceLabel) {
  const summaryPath = join(root, 'suite-summary.json');
  const summary = readJson(summaryPath);
  check(summary.status === 'completed', `${sourceLabel}: accepted archive is not completed`);
  check(Array.isArray(summary.cases), `${sourceLabel}: suite-summary.json has no cases array`);
  check(summary.case_count === summary.cases.length, `${sourceLabel}: case_count does not match cases`);
  if (expectedSchema) check(summary.schema_id === expectedSchema, `${sourceLabel}: unexpected archive schema ${summary.schema_id}`);
  if (summary.provenance) {
    check(summary.provenance.unchanged === true, `${sourceLabel}: accepted archive provenance was not unchanged`);
    assertTypedEqual(summary.provenance.before, summary.provenance.after, `${sourceLabel} provenance before/after`);
  }
  return {summary, cases: new Map(summary.cases.map(record => [record.case_id, record]))};
}

function readPackInputs() {
  const packPath = join(ROOT, PACK_REL);
  const planPath = join(ROOT, PRACTICAL_REL);
  const freshPath = join(ROOT, FRESH_REL);
  const packBytes = readFileSync(packPath);
  const pack = JSON.parse(packBytes);
  check(pack.schema_id === 'planner_v2_eval_pack_v1' && pack.id === 'planner_v2_lab_suite', 'unexpected registered V2 pack identity');
  assertTypedEqual(pack.sources, EXPECTED_SOURCES, 'registered V2 pack source contract');

  const planBytes = readFileSync(planPath);
  const freshBytes = readFileSync(freshPath);
  check(sha256(planBytes) === EXPECTED_SOURCES[0].sha256, 'frozen practical fixture digest changed');
  check(sha256(freshBytes) === EXPECTED_SOURCES[1].sha256, 'frozen fresh-terrain fixture digest changed');
  const plan = JSON.parse(planBytes);
  const fresh = JSON.parse(freshBytes);
  check(plan.schema_id === EXPECTED_SOURCES[0].kind && plan.status === 'design_only_not_flight_accepted', 'practical fixture identity/status changed');
  check(fresh.schema_id === EXPECTED_SOURCES[1].kind && fresh.status === 'frozen_before_mission_attempts', 'fresh fixture identity/status changed');
  check(plan.cases.length === 32 && fresh.cases.length === 12, 'expected exactly 32 practical and 12 fresh fixture IDs');

  const manifestHashes = {};
  const frozenPaths = [PACK_REL, PRACTICAL_REL, FRESH_REL];
  for (const [sourceLabel, fixture] of [['practical', plan], ['fresh', fresh]]) {
    for (const [name, manifest] of Object.entries(fixture.source_manifests ?? {})) {
      const path = resolve(ROOT, manifest.path);
      check(inside(ROOT, path) && statSync(path).isFile(), `${sourceLabel} source manifest path is unavailable: ${manifest.path}`);
      const digest = sha256(readFileSync(path));
      check(digest === manifest.sha256, `${manifest.path}: frozen source manifest digest changed`);
      frozenPaths.push(manifest.path);
      if (manifestHashes[manifest.path]) check(manifestHashes[manifest.path] === digest, `${manifest.path}: practical/fresh source manifest seals disagree`);
      manifestHashes[manifest.path] = digest;
    }
  }

  const expectedCases = [];
  for (const recipe of plan.cases) {
    expectedCases.push({
      case_id: recipe.id,
      source_set: 'practical_suite',
      source_group: recipe.group,
      group: recipe.group,
      family: recipe.family,
      base_case_id: recipe.base,
      input_path: PRACTICAL_REL,
      expected_preflight: recipe.expected_preflight ?? null,
    });
  }
  for (const recipe of fresh.cases) {
    expectedCases.push({
      case_id: recipe.id,
      source_set: 'fresh_terrain_inputs',
      source_group: recipe.group,
      group: recipe.group === 'terrain' ? 'additional_terrain' : 'clear',
      family: recipe.family,
      base_case_id: recipe.base_case_id,
      input_path: FRESH_REL,
      expected_preflight: null,
    });
  }
  const uniqueIds = new Set(expectedCases.map(item => item.case_id));
  check(uniqueIds.size === 44, 'source fixtures do not define 44 unique stable case IDs');
  return {
    pack,
    pack_sha256: sha256(packBytes),
    plan,
    fresh,
    expectedCases,
    source_fixture_hashes: Object.fromEntries(EXPECTED_SOURCES.map(source => [source.source_id, source.sha256])),
    source_manifest_hashes: manifestHashes,
    frozen_paths: [...new Set(frozenPaths)],
  };
}

function archiveResult(record, label) {
  const result = record?.result;
  check(isRecord(result), `${label}: accepted suite record is missing typed result fields`);
  return result;
}

function expectedCounts(cases) {
  const summary = Object.fromEntries(SUMMARY_COUNT_KEYS.map(key => [key, 0]));
  const planningStops = {};
  const physicalOutcomes = {};
  const missionOutcomes = {};
  for (const item of cases) {
    summary[`${item.group}_count`] += 1;
    const result = item.result;
    if (result.planning_stop) planningStops[result.planning_stop] = (planningStops[result.planning_stop] ?? 0) + 1;
    if (result.physical_outcome) physicalOutcomes[result.physical_outcome] = (physicalOutcomes[result.physical_outcome] ?? 0) + 1;
    if (result.mission_outcome) missionOutcomes[result.mission_outcome] = (missionOutcomes[result.mission_outcome] ?? 0) + 1;
    const diagnostic = item.group === 'diagnostic';
    const validLanding = result.physical_outcome === 'landed_on_target'
      && result.mission_outcome === 'success'
      && result.planning_stop === 'landed'
      && result.integrity_passed === true
      && result.final_source_replay_passed === true;
    if (validLanding && diagnostic) summary.diagnostic_landing_count += 1;
    else if (validLanding) {
      summary.valid_landing_count += 1;
      if (result.correction_count === 0) summary.direct_landing_count += 1;
      else summary.corrected_landing_count += 1;
    } else if (item.supported && diagnostic) summary.diagnostic_non_landing_count += 1;
    else if (item.supported) summary.non_landing_count += 1;
    if (result.planning_stop === 'unsupported' || result.physical_outcome === null || result.physical_outcome === undefined) summary.unsupported_count += 1;
    if (result.physical_outcome === 'crashed') summary.crash_count += 1;
    if (result.integrity_passed === true) summary.integrity_passed_count += 1;
    if (result.integrity_passed === false) summary.integrity_failed_count += 1;
    if (item.supported && result.final_source_replay_passed === true) summary.final_source_replay_passed_count += 1;
    if (item.supported && result.final_source_replay_passed === false) summary.final_source_replay_failed_count += 1;
    if (result.initial_nominal_terrain_blocked === true) summary.initially_blocked_count += 1;
  }
  return {summary, planningStops, physicalOutcomes, missionOutcomes};
}

function expectedArchiveCases(inputs, practicalArchive, freshArchive) {
  const inventory = [];
  for (const item of inputs.expectedCases) {
    const archive = item.source_set === 'practical_suite' ? practicalArchive : freshArchive;
    const record = archive.cases.get(item.case_id);
    check(record, `${item.case_id}: accepted archive case is missing`);
    const result = archiveResult(record, item.case_id);
    const archivedGroup = record.group;
    assertTypedEqual(archivedGroup, item.source_group, `${item.case_id} archived source group`);
    assertTypedEqual(record.family, item.family, `${item.case_id} archived family`);
    const scenarioPath = join(archive.root, 'runs', item.case_id, 'scenario.json');
    // The writer's typed ScenarioSpec snapshot exists for preflight rejections
    // too. Use it consistently: raw recipe inputs omit optional null fields.
    check(statSync(scenarioPath, {throwIfNoEntry: false})?.isFile(), `${item.case_id}: accepted archive typed scenario is missing`);
    const scenario = readJson(scenarioPath);
    const flightPath = join(archive.root, 'runs', item.case_id, 'flight.json');
    const reportPath = join(archive.root, 'runs', item.case_id, 'report.html');
    const supported = result.physical_outcome !== null && result.physical_outcome !== undefined;
    check(statSync(flightPath, {throwIfNoEntry: false})?.isFile(), `${item.case_id}: accepted typed flight.json is missing`);
    if (supported) {
      check(statSync(reportPath, {throwIfNoEntry: false})?.isFile(), `${item.case_id}: accepted rich report is missing`);
    } else {
      check(result.planning_stop === 'unsupported' && record.cli_exit_code === 1,
        `${item.case_id}: only the two accepted preflight rejections may lack simulated artifacts`);
      check(!statSync(reportPath, {throwIfNoEntry: false}), `${item.case_id}: accepted unsupported archive unexpectedly has a raw report`);
    }
    inventory.push({
      ...item,
      archive_root: archive.root,
      archive_record: record,
      result,
      scenario,
      flight_path: flightPath,
      report_path: supported ? reportPath : null,
      supported,
    });
  }
  check(inventory.filter(item => item.supported).length === 42, 'accepted evidence must contain 42 simulated cases');
  check(inventory.filter(item => !item.supported).length === 2, 'accepted evidence must contain exactly two unsupported preflight cases');
  return inventory;
}

function removePointer(object, pointer, label, removed) {
  const parts = pointer.split('/').slice(1);
  let parent = object;
  for (const part of parts.slice(0, -1)) {
    if (!isRecord(parent) || !Object.hasOwn(parent, part)) return;
    parent = parent[part];
  }
  const leaf = parts.at(-1);
  if (isRecord(parent) && Object.hasOwn(parent, leaf)) {
    removed.push({pointer, value: parent[leaf]});
    delete parent[leaf];
  }
}

export function comparableFlight(flight, counterpart, label) {
  check(isRecord(flight) && isRecord(counterpart), `${label}: both flight payloads must be objects`);
  const actual = structuredClone(flight);
  const expected = structuredClone(counterpart);
  const ignored = [];
  for (const pointer of TIMING_POINTERS) {
    const actualHas = pointerExists(actual, pointer);
    const expectedHas = pointerExists(expected, pointer);
    check(actualHas === expectedHas, `${label}: timing field presence differs at ${pointer}`);
    if (actualHas) {
      check(Number.isFinite(pointerValue(actual, pointer)) && Number.isFinite(pointerValue(expected, pointer)), `${label}: nonnumeric timing exclusion at ${pointer}`);
      removePointer(actual, pointer, label, ignored);
      removePointer(expected, pointer, label, []);
    }
  }
  for (const pointer of PATH_POINTERS) {
    const actualHas = pointerExists(actual, pointer);
    const expectedHas = pointerExists(expected, pointer);
    check(actualHas === expectedHas, `${label}: path field presence differs at ${pointer}`);
    if (actualHas) {
      const actualPath = pointerValue(actual, pointer);
      const expectedPath = pointerValue(expected, pointer);
      check(typeof actualPath === 'string' && typeof expectedPath === 'string', `${label}: observed path field is not a string at ${pointer}`);
      removePointer(actual, pointer, label, ignored);
      removePointer(expected, pointer, label, []);
    }
  }
  assertTypedEqual(actual, expected, label);
  return ignored.map(item => item.pointer);
}

function pointerExists(object, pointer) {
  let value = object;
  for (const part of pointer.split('/').slice(1)) {
    if (!isRecord(value) || !Object.hasOwn(value, part)) return false;
    value = value[part];
  }
  return true;
}

function pointerValue(object, pointer) {
  let value = object;
  for (const part of pointer.split('/').slice(1)) value = value[part];
  return value;
}

export function extractReportData(html, label = 'report') {
  const marker = 'const reportData = ';
  const start = html.indexOf(marker);
  check(start >= 0, `${label}: missing const reportData payload`);
  const payloadStart = start + marker.length;
  const payloadEnd = html.indexOf(';\n', payloadStart);
  check(payloadEnd > payloadStart, `${label}: unterminated reportData payload`);
  try {
    const value = JSON.parse(html.slice(payloadStart, payloadEnd));
    check(isRecord(value), `${label}: reportData must be a JSON object`);
    return value;
  } catch (error) {
    throw new Error(`${label}: invalid reportData JSON: ${error.message}`);
  }
}

function boundarySnapshot(state) {
  check(isRecord(state), 'missing local-correction segment boundary state');
  return {
    physicsStep: state.physics_step,
    simTimeS: state.sim_time_s,
    positionM: state.position_m,
    velocityMps: state.velocity_mps,
    attitudeRad: state.attitude_rad,
    fuelKg: state.fuel_kg,
  };
}

export function verifyCorrectionAnnotations(flight, reportData, label) {
  const annotations = reportData.flightAnnotations;
  check(isRecord(annotations) && Array.isArray(annotations.corrections), `${label}: annotated reportData.flightAnnotations is missing`);
  const corrections = annotations.corrections;
  const segments = flight.segments.filter(segment => segment.kind === 'local_correction');
  check(corrections.length === flight.correction_count, `${label}: annotation count differs from flight correction_count`);
  check(segments.length === flight.correction_count, `${label}: raw LocalCorrection segment count differs from correction_count`);
  for (let index = 0; index < corrections.length; index += 1) {
    const annotation = corrections[index];
    const segment = segments[index];
    check(annotation.number === index + 1, `${label}: handoff numbers are not consecutive`);
    assertTypedEqual(annotation.entry, boundarySnapshot(segment.entry_state), `${label} H${index + 1} exact entry snapshot`);
    assertTypedEqual(annotation.handoff, boundarySnapshot(segment.end_state), `${label} H${index + 1} exact handoff snapshot`);
    check(typeof annotation.reason === 'string' && annotation.reason.trim().length > 0, `${label} H${index + 1}: missing correction reason`);
    check(typeof annotation.afterHandoff === 'string' && annotation.afterHandoff.trim().length > 0, `${label} H${index + 1}: missing post-handoff result`);
  }
  return corrections.length;
}

function validateRichReport(rawHtml, annotatedHtml, flight, id) {
  const raw = extractReportData(rawHtml, `${id} raw rich report`);
  const annotated = extractReportData(annotatedHtml, `${id} annotated detail`);
  check(!Object.hasOwn(raw, 'flightAnnotations'), `${id}: raw report was already annotated`);
  check(Object.hasOwn(annotated, 'flightAnnotations'), `${id}: annotated reportData.flightAnnotations is missing`);
  const annotations = structuredClone(annotated.flightAnnotations);
  delete annotated.flightAnnotations;
  assertTypedEqual(annotated, raw, `${id} rich report parity after removing only flightAnnotations`);
  check(annotated.scenarioId === id, `${id}: rich report scenario identity changed`);
  const count = verifyCorrectionAnnotations(flight, {flightAnnotations: annotations}, id);
  check(raw.samples.length > 0, `${id}: original rich chart data has no samples`);
  for (const marker of ['id="chart-spatial"', 'id="chart-metrics"', 'data-mode="mission"', 'data-mode="guidance"', 'data-mode="speed"', 'data-mode="throttle"', 'data-mode="vectors"']) {
    check(rawHtml.includes(marker), `${id}: original rich chart view is missing ${marker}`);
    check(annotatedHtml.includes(marker), `${id}: annotated detail lost rich chart view ${marker}`);
  }
  return {report_sample_count: raw.samples.length, correction_annotation_count: count};
}

function resultProjection(value) {
  return Object.fromEntries([
    'planning_stop',
    'correction_count',
    'initial_nominal_terrain_blocked',
    'integrity_passed',
    'final_source_replay_passed',
    'physical_outcome',
    'mission_outcome',
  ].map(key => [key, value[key]]));
}

function compareCaseResult(actual, expected, id, source) {
  assertTypedEqual(resultProjection(actual), resultProjection(expected), `${id} ${source} accepted result`);
  check(actual.correction_count === expected.correction_count, `${id}: correction_count changed`);
  check(actual.integrity_passed === true, `${id}: integrity_passed is not true`);
  if (expected.physical_outcome !== null && expected.physical_outcome !== undefined) {
    check(actual.final_source_replay_passed === true, `${id}: final source replay did not pass`);
  }
}

function validateUnsupportedFlight(flight, id) {
  check(flight.planning_stop === 'unsupported', `${id}: unsupported case is not recorded as unsupported`);
  check(flight.manifest === null, `${id}: unsupported result fabricated a manifest`);
  check(flight.ordinary_flight === null, `${id}: unsupported result fabricated ordinary flight data`);
  check(Array.isArray(flight.segments) && flight.segments.length === 0, `${id}: unsupported result has flown segments`);
  check(Array.isArray(flight.cycles) && flight.cycles.length === 0, `${id}: unsupported result has planner cycles`);
  check(flight.physical_outcome === null && flight.mission_outcome === null, `${id}: unsupported result invented a physical or mission outcome`);
  check(flight.final_source_replay_passed === false, `${id}: unsupported result claims a source replay`);
}

export function verifyOrdinaryExecution(flight, id) {
  const ordinary = flight.ordinary_flight;
  check(Array.isArray(ordinary?.actions), `${id}: simulated result has no raw action array`);
  check(Array.isArray(ordinary?.samples) && ordinary.samples.length > 0, `${id}: simulated result has no raw samples`);
  if (ordinary.actions.length === 0) {
    check(flight.planning_stop === 'no_clearing' && flight.physical_outcome === 'flying'
      && flight.mission_outcome === 'in_progress', `${id}: empty commands are not a landing`);
    check(flight.manifest.physics_steps === 0 && flight.manifest.controller_updates === 0
      && flight.manifest.sim_time_s === 0, `${id}: empty commands have nonzero execution coverage`);
    check(ordinary.samples.length === 1 && ordinary.samples[0].physics_step === 0,
      `${id}: zero-command stop must retain only its initial sample`);
  }
}

function validateDiagnosticPage(html, id, record) {
  const lower = html.toLowerCase();
  check(html.includes(id), `${id}: diagnostic page omits stable case ID`);
  check(lower.includes('unsupported'), `${id}: diagnostic page does not identify unsupported setup`);
  check(lower.includes('not simulated') || lower.includes('no flight') || lower.includes('no simulator trajectory'), `${id}: diagnostic page does not say the case was not simulated`);
  check(!html.includes('id="chart-spatial"') && !html.includes('id="chart-metrics"'), `${id}: diagnostic page fabricated rich flight charts`);
  check(typeof record.reason === 'string' && record.reason.trim().length > 0, `${id}: unsupported suite record lacks its reason`);
  const visibleReason = visibleText(record.reason).toLowerCase();
  check(visibleText(html).toLowerCase().includes(visibleReason.slice(0, Math.min(24, visibleReason.length))), `${id}: diagnostic page omits the recorded unsupported reason`);
  const maybePayload = html.includes('const reportData = ') ? extractReportData(html, `${id} diagnostic page`) : null;
  if (maybePayload) {
    check(!Object.hasOwn(maybePayload, 'manifest') && !Object.hasOwn(maybePayload, 'samples'), `${id}: diagnostic page carries simulated report data`);
    check(!Object.hasOwn(maybePayload, 'flightAnnotations'), `${id}: diagnostic page carries flight annotations`);
  }
}

function validateBatchRecord(record, expected, root, accepted, artifactInventory) {
  const id = expected.case_id;
  check(record.case_id === id, `${id}: batch case ID mismatch`);
  for (const field of ['source_set', 'source_group', 'group', 'family', 'base_case_id']) {
    assertTypedEqual(record[field], expected[field], `${id} batch ${field}`);
  }
  assertTypedEqual(record.input_path, expected.input_path, `${id} source fixture path`);
  assertTypedEqual(record.expected_preflight ?? null, expected.expected_preflight, `${id} expected preflight`);

  const expectedResult = accepted.result;
  compareCaseResult(record, expectedResult, id, 'batch');
  const supported = accepted.supported;
  check(record.status === (supported ? 'simulated' : 'preflight_rejected'), `${id}: unexpected batch case status ${record.status}`);
  assertTypedEqual(record.outcome, expectedResult.physical_outcome ?? expectedResult.planning_stop, `${id} typed outcome label`);

  const scenarioPath = resolveCaptureFile(root, record.scenario_path, `${id} scenario_path`);
  const scenario = readJson(scenarioPath);
  assertTypedEqual(scenario, accepted.scenario, `${id} complete typed scenario vs accepted archive`);
  check(scenario.id === id, `${id}: scenario ID mismatch`);
  const rawFlightPath = resolveCaptureFile(root, record.flight_path, `${id} flight_path`);
  const flight = readJson(rawFlightPath);
  compareCaseResult(flight, expectedResult, id, 'raw flight.json');
  if (supported) {
    check(flight.manifest && flight.ordinary_flight, `${id}: supported flight has no manifest or ordinary flight`);
    verifyOrdinaryExecution(flight, id);
    const archiveFlight = readJson(accepted.flight_path);
    const exclusions = comparableFlight(flight, archiveFlight, `${id} complete flight.json vs accepted archive`);
    artifactInventory[id] = {scenario_sha256: sha256(readFileSync(scenarioPath)), flight_sha256: sha256(readFileSync(rawFlightPath)), archive_flight_sha256: sha256(readFileSync(accepted.flight_path)), flight_exclusions: exclusions};

    const summaryPath = resolveCaptureFile(root, record.summary_path, `${id} summary_path`);
    const summary = readJson(summaryPath);
    compareCaseResult(summary.result, expectedResult, id, 'per-run summary.json');
    check(summary.run_summary !== null && isRecord(summary.run_summary), `${id}: supported summary has no run_summary manifest projection`);
    const richPath = resolveCaptureFile(root, record.rich_report_path, `${id} rich_report_path`);
    const annotatedPath = resolveCaptureFile(root, record.annotated_report_path, `${id} annotated_report_path`);
    const reportMetadata = validateRichReport(readFileSync(richPath, 'utf8'), readFileSync(annotatedPath, 'utf8'), flight, id);
    artifactInventory[id].raw_report_sha256 = sha256(readFileSync(richPath));
    artifactInventory[id].annotated_report_sha256 = sha256(readFileSync(annotatedPath));
    artifactInventory[id].report_sample_count = reportMetadata.report_sample_count;
    artifactInventory[id].correction_annotation_count = reportMetadata.correction_annotation_count;
  } else {
    validateUnsupportedFlight(flight, id);
    const archiveFlight = readJson(accepted.flight_path);
    const exclusions = comparableFlight(flight, archiveFlight, `${id} complete unsupported flight.json vs accepted archive`);
    const summaryPath = resolveCaptureFile(root, record.summary_path, `${id} summary_path`);
    const summary = readJson(summaryPath);
    compareCaseResult(summary.result, expectedResult, id, 'preflight summary.json');
    check(summary.run_summary === null, `${id}: unsupported case has a fabricated run summary/manifest`);
    check(record.rich_report_path === null, `${id}: unsupported case declares a raw rich report`);
    const richPath = resolveCaptureFile(root, join(dirname(record.annotated_report_path), 'report.html'), `${id} raw report`, {required: false});
    check(richPath === null || !lstatSync(richPath, {throwIfNoEntry: false}), `${id}: unsupported case has a raw report.html`);
    const annotatedPath = resolveCaptureFile(root, record.annotated_report_path, `${id} diagnostic detail`);
    validateDiagnosticPage(readFileSync(annotatedPath, 'utf8'), id, record);
    artifactInventory[id] = {
      scenario_sha256: sha256(readFileSync(scenarioPath)),
      flight_sha256: sha256(readFileSync(rawFlightPath)),
      archive_flight_sha256: sha256(readFileSync(accepted.flight_path)),
      flight_exclusions: exclusions,
      unsupported_reason: record.reason,
      raw_report_absent: true,
    };
  }

  check(isRecord(record.artifact_sha256), `${id}: artifact_sha256 map is missing`);
  const requiredHashes = [record.scenario_path, record.flight_path, record.summary_path, ...(supported ? [record.rich_report_path] : [])];
  for (const pathValue of requiredHashes) {
    const file = resolveCaptureFile(root, pathValue, `${id} artifact hash path`);
    const expectedHash = record.artifact_sha256[pathValue];
    check(isSha256(expectedHash), `${id}: artifact_sha256 has no digest for ${pathValue}`);
    const actualHash = sha256(readFileSync(file));
    check(actualHash === expectedHash, `${id}: raw artifact digest mismatch at ${pathValue}`);
  }
  for (const [pathValue, digest] of Object.entries(record.artifact_sha256)) {
    check(isSha256(digest), `${id}: malformed artifact SHA-256 for ${pathValue}`);
    const file = resolveCaptureFile(root, pathValue, `${id} artifact digest`);
    check(sha256(readFileSync(file)) === digest, `${id}: recorded artifact digest mismatch at ${pathValue}`);
  }
}

function validateExpandedInputSnapshot(batchRoot, batch, inputs, accepted) {
  check(batch.pack_snapshot_path === 'pack.json', 'batch pack snapshot path changed');
  check(batch.expanded_inputs_snapshot_path === 'expanded-inputs.json', 'batch expanded-input snapshot path changed');
  const packPath = resolveCaptureFile(batchRoot, batch.pack_snapshot_path, 'pack snapshot');
  const packBytes = readFileSync(packPath);
  check(sha256(packBytes) === batch.pack_snapshot_sha256 && batch.pack_snapshot_sha256 === inputs.pack_sha256,
    'raw pack snapshot digest does not match the registered frozen pack');
  assertTypedEqual(JSON.parse(packBytes), inputs.pack, 'capture pack snapshot content');
  const expandedPath = resolveCaptureFile(batchRoot, batch.expanded_inputs_snapshot_path, 'expanded input snapshot');
  const expandedBytes = readFileSync(expandedPath);
  check(isSha256(batch.expanded_inputs_snapshot_sha256) && sha256(expandedBytes) === batch.expanded_inputs_snapshot_sha256,
    'expanded-input snapshot SHA-256 mismatch');
  const expanded = JSON.parse(expandedBytes);
  check(Array.isArray(expanded) && expanded.length === 44, 'expanded-input snapshot must contain all 44 typed inputs');
  assertTypedEqual(expanded.map(item => item.case_id), inputs.expectedCases.map(item => item.case_id), 'expanded-input stable case order');
  for (let index = 0; index < expanded.length; index += 1) {
    const item = expanded[index];
    const expected = inputs.expectedCases[index];
    const caseRecord = batch.cases[index];
    const acceptedCase = accepted[index];
    for (const key of ['source_set', 'source_group', 'group', 'family', 'base_case_id', 'expected_preflight']) {
      assertTypedEqual(item[key] ?? null, expected[key] ?? null, `${item.case_id} expanded input ${key}`);
      assertTypedEqual(caseRecord[key] ?? null, item[key] ?? null, `${item.case_id} batch row/input ${key}`);
    }
    for (const key of ['source_pad_id', 'target_pad_id']) {
      check(typeof item[key] === 'string' && item[key].length > 0, `${item.case_id}: expanded input is missing ${key}`);
      assertTypedEqual(caseRecord[key], item[key], `${item.case_id} batch ${key}`);
    }
    assertTypedEqual(item.scenario, acceptedCase.scenario, `${item.case_id} expanded typed scenario vs accepted archive`);
  }
  return {expanded_input_sha256: sha256(expandedBytes), count: expanded.length};
}

function validateBatchSummary(batch, acceptedCases) {
  const counts = expectedCounts(acceptedCases);
  check(isRecord(batch.summary), 'batch summary object is missing');
  for (const key of SUMMARY_COUNT_KEYS) {
    check(batch.summary[key] === counts.summary[key], `batch summary ${key}=${batch.summary[key]} disagrees with accepted per-case records (${counts.summary[key]})`);
  }
  assertTypedEqual(batch.summary.planning_stops, counts.planningStops, 'batch planning-stop histogram');
  assertTypedEqual(batch.summary.physical_outcomes, counts.physicalOutcomes, 'batch physical-outcome histogram');
  assertTypedEqual(batch.summary.mission_outcomes, counts.missionOutcomes, 'batch mission-outcome histogram');
  assertTypedEqual(
    Object.fromEntries(Object.keys(EXPECTED_GROUP_COUNTS).map(key => [key, batch.summary[key]])),
    EXPECTED_GROUP_COUNTS,
    'expected practical/fresh group denominators',
  );
  check(counts.summary.valid_landing_count === 36, 'accepted records no longer show 36 core landings');
  check(counts.summary.diagnostic_landing_count === 2, 'accepted records no longer show two diagnostic landings');
  check(counts.summary.non_landing_count === 0, 'accepted records no longer show zero core non-landings');
  check(counts.summary.diagnostic_non_landing_count === 4, 'accepted records no longer show four diagnostic finite stops');
  check(counts.summary.simulation_unverified_count === 0, 'accepted records unexpectedly include unverified simulation cases');
  check(counts.summary.unsupported_count === 2, 'accepted records no longer show two unsupported preflight cases');
}

function parseAttributes(source) {
  const attributes = {};
  const pattern = /([^\s=]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/g;
  let match;
  while ((match = pattern.exec(source))) attributes[match[1].toLowerCase()] = decodeHtml(match[2] ?? match[3] ?? match[4] ?? '');
  return attributes;
}

function decodeHtml(value) {
  return value.replaceAll('&amp;', '&').replaceAll('&quot;', '"').replaceAll('&#39;', "'")
    .replaceAll('&lt;', '<').replaceAll('&gt;', '>');
}

function visibleText(html) {
  return decodeHtml(html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, ' ')
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, ' ')
    .replace(/<[^>]+>/g, ' ')).replace(/\s+/g, ' ').trim();
}

export function parseAnchors(html, pageUrl) {
  const baseMatch = /<base\b([^>]*)>/i.exec(html);
  const baseHref = baseMatch ? parseAttributes(baseMatch[1]).href : null;
  const baseUrl = baseHref ? new URL(baseHref, pageUrl) : new URL(pageUrl);
  const anchors = [];
  const pattern = /<a\b([^>]*)>([\s\S]*?)<\/a\s*>/gi;
  let match;
  while ((match = pattern.exec(html))) {
    const attributes = parseAttributes(match[1]);
    if (!attributes.href) continue;
    let url;
    try { url = new URL(attributes.href, baseUrl); } catch { continue; }
    anchors.push({href: attributes.href, url: url.href, pathname: url.pathname, text: visibleText(match[2]), attributes});
  }
  return {baseHref, baseUrl: baseUrl.href, anchors};
}

function localHttpUrl(value, label) {
  const url = new URL(value);
  check(url.protocol === 'http:' || url.protocol === 'https:', `${label}: expected HTTP URL`);
  check(['127.0.0.1', 'localhost', '::1', '[::1]'].includes(url.hostname), `${label}: local loopback host is required`);
  return url;
}

export function verifyHttpContent(contentType, body, label, kind = 'html') {
  if (kind === 'json') {
    check(/application\/json/i.test(contentType), `${label}: expected JSON, got ${contentType || 'no content type'}`);
    JSON.parse(body);
  } else {
    check(/text\/html|application\/xhtml\+xml/i.test(contentType), `${label}: expected HTML, got ${contentType || 'no content type'}`);
  }
}

async function fetchPage(url, origin, label, kind = 'html') {
  const parsed = new URL(url);
  check(parsed.origin === origin, `${label}: link escapes the requested local report server (${parsed.href})`);
  const response = await fetch(parsed, {redirect: 'follow', signal: AbortSignal.timeout(20000)});
  check(response.ok, `${label}: HTTP ${response.status} ${response.statusText} at ${response.url}`);
  const contentType = response.headers.get('content-type') ?? '';
  const body = await response.text();
  verifyHttpContent(contentType, body, label, kind);
  return {url: response.url, html: body, status: response.status};
}

function findBatchCaseAnchor(anchors, expectedUrl, id) {
  const target = new URL(expectedUrl);
  return anchors.find(anchor => new URL(anchor.url).pathname === target.pathname);
}

export function isEvidenceAnchor(anchor) {
  return new URL(anchor.url).pathname.endsWith('.json')
    || /source|original|scenario\.json|flight\.json|summary\.json|input/i.test(anchor.text);
}

async function crawlReportSite(batchRoot, batch, rootUrl) {
  const root = localHttpUrl(rootUrl, '--root-url');
  const origin = root.origin;
  const rootPage = await fetchPage(root.href, origin, 'reports root');
  const rootLinks = parseAnchors(rootPage.html, rootPage.url).anchors;
  const topicLink = rootLinks.find(link => link.pathname.endsWith('/reports/topics/waypoint-planning/index.html'))
    ?? rootLinks.find(link => /waypoint planning/i.test(link.text) && /topics\/waypoint-planning/.test(link.pathname));
  check(topicLink, 'reports root has no waypoint-planning topic link');
  const topicPage = await fetchPage(topicLink.url, origin, 'waypoint-planning topic');
  const topicLinks = parseAnchors(topicPage.html, topicPage.url).anchors;
  const stableLink = topicLinks.find(link => link.pathname.includes('/eval/planner_v2_lab_suite'));
  check(stableLink, 'waypoint-planning topic has no registered planner_v2_lab_suite batch link');
  const stablePage = await fetchPage(stableLink.url, origin, 'stable Planner V2 batch page');
  const stableLinks = parseAnchors(stablePage.html, stablePage.url);
  check(stableLinks.baseHref, 'stable Planner V2 batch page has no <base href>; detail links cannot be resolved safely');

  const outputsRoot = realpathSync(join(ROOT, 'outputs'));
  const batchReal = realpathSync(batchRoot);
  check(inside(outputsRoot, batchReal), '--root-url can only audit captures served beneath repository outputs/');
  const captureRelative = relative(outputsRoot, batchReal).split(sep).join('/');
  const expectedBasePath = `/${captureRelative.split('/').map(encodeURIComponent).join('/')}/`;
  const actualBaseUrl = new URL(stableLinks.baseUrl);
  check(actualBaseUrl.origin === origin, 'stable batch <base href> points to a different report server');
  check(actualBaseUrl.pathname === expectedBasePath, `stable batch <base href> resolves to ${actualBaseUrl.pathname}, expected this capture ${expectedBasePath}`);

  const detailUrls = new Map();
  for (const record of batch.cases) {
    const expected = new URL(record.annotated_report_path, actualBaseUrl);
    const anchor = findBatchCaseAnchor(stableLinks.anchors, expected.href, record.case_id);
    check(anchor, `stable batch page has no detail link for ${record.case_id} resolved against its <base href>`);
    detailUrls.set(record.case_id, expected.href);
  }

  const sourceLinks = [];
  const detailRecords = [];
  const caseIndexes = new Map(batch.cases.map((record, index) => [record.case_id, index]));
  const detailPathToId = new Map([...detailUrls].map(([id, url]) => [new URL(url).pathname, id]));
  const captureIndexUrl = new URL('index.html', actualBaseUrl);
  const caseIds = new Set(batch.cases.map(record => record.case_id));
  const caseIdFromPath = pathname => {
    const match = /\/(?:runs|cases)\/([^/]+)\/index[.]html$/.exec(pathname);
    if (!match) return detailPathToId.get(pathname) ?? null;
    let id;
    try { id = decodeURIComponent(match[1]); } catch { return null; }
    return caseIds.has(id) ? id : null;
  };
  for (const record of batch.cases) {
    const id = record.case_id;
    const detailUrl = detailUrls.get(id);
    const page = await fetchPage(detailUrl, origin, `${id} detail page`);
    const parsed = parseAnchors(page.html, page.url);
    const index = caseIndexes.get(id);
    const actualNeighborIds = parsed.anchors.map(link => caseIdFromPath(link.pathname)).filter(Boolean);
    const expectedNeighbors = [batch.cases[index - 1]?.case_id, batch.cases[index + 1]?.case_id].filter(Boolean);
    for (const neighbor of expectedNeighbors) {
      check(actualNeighborIds.includes(neighbor), `${id}: missing adjacent previous/next detail link to ${neighbor}`);
    }
    check(actualNeighborIds.every(neighbor => expectedNeighbors.includes(neighbor)), `${id}: detail navigation links to a nonadjacent case`);

    const collectionPathnames = new Set([new URL(stablePage.url).pathname, captureIndexUrl.pathname]);
    const collectionLink = parsed.anchors.find(link => collectionPathnames.has(new URL(link.url).pathname));
    check(collectionLink, `${id}: missing return link to the stable Planner V2 batch or capture index`);
    const collectionPage = await fetchPage(collectionLink.url, origin, `${id} batch return link`);
    check(batch.cases.every(record => collectionPage.html.includes(record.case_id)), `${id}: batch return page omits one or more stable case IDs`);

    const homeLink = parsed.anchors.find(link => new URL(link.url).pathname === '/reports/');
    const topicPath = '/reports/topics/waypoint-planning/index.html';
    const topicAnchor = parsed.anchors.find(link => new URL(link.url).pathname === topicPath);
    check(homeLink, `${id}: detail is missing its Report home link`);
    check(topicAnchor, `${id}: detail is missing its Waypoint planning topic link`);
    sourceLinks.push({case_id: id, label: 'Report home', url: homeLink.url});
    sourceLinks.push({case_id: id, label: 'Waypoint planning topic', url: topicAnchor.url});

    let reportData = null;
    if (page.html.includes('const reportData = ')) reportData = extractReportData(page.html, `${id} HTTP detail page`);
    const declared = reportData?.flightAnnotations?.navigation?.sourceLinks;
    if (Array.isArray(declared)) {
      for (const source of declared) {
        check(typeof source.href === 'string' && source.href.length > 0, `${id}: declared source link has no href`);
        const url = new URL(source.href, parsed.baseUrl);
        check(url.origin === origin, `${id}: declared source link escapes local report server`);
        sourceLinks.push({case_id: id, label: source.label ?? '', url: url.href});
      }
    }
    for (const anchor of parsed.anchors) {
      if (isEvidenceAnchor(anchor)) {
        const url = new URL(anchor.url);
        check(url.origin === origin, `${id}: source evidence link escapes local report server`);
        sourceLinks.push({case_id: id, label: anchor.text, url: url.href});
      }
    }
    detailRecords.push({case_id: id, url: page.url, status: page.status, links: parsed.anchors.length, neighbor_ids: actualNeighborIds});
  }

  const uniqueSources = [...new Map(sourceLinks.map(link => [link.url, link])).values()];
  for (const source of uniqueSources) {
    const kind = new URL(source.url).pathname.endsWith('.json') ? 'json' : 'html';
    await fetchPage(source.url, origin, `${source.case_id} source link ${source.label || source.url}`, kind);
  }
  return {
    origin,
    root_url: rootPage.url,
    waypoint_topic_url: topicPage.url,
    stable_batch_url: stablePage.url,
    stable_base_href: stableLinks.baseHref,
    capture_base_url: stableLinks.baseUrl,
    case_detail_count: detailRecords.length,
    source_link_count: uniqueSources.length,
    details: detailRecords,
    source_links: uniqueSources,
  };
}

export function cdpClient(ws) {
  let sequence = 0;
  const pending = new Map();
  const errors = [];
  const optional = [];
  ws.addEventListener('message', event => {
    let message;
    try { message = JSON.parse(event.data); } catch (error) { errors.push({kind: 'malformed-cdp-message', message: String(error)}); return; }
    if (message.id) {
      const item = pending.get(message.id);
      if (!item) return;
      clearTimeout(item.timer);
      pending.delete(message.id);
      if (message.error) item.reject(new Error(`${item.method}: ${message.error.message}`));
      else item.resolve(message.result ?? {});
    } else if (message.method === 'Runtime.exceptionThrown') {
      errors.push({kind: 'javascript-exception', detail: message.params.exceptionDetails});
    } else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
      const entry = message.params.entry;
      if (entry.source === 'network' && /\/favicon\.ico(?:\?|$)/.test(entry.url ?? '') && /\b404\b/.test(entry.text ?? '')) optional.push(entry);
      else errors.push({kind: 'console-error', entry});
    } else if (message.method === 'Network.responseReceived' && message.params.response.status >= 400) {
      const response = message.params.response;
      if (/\/favicon\.ico(?:\?|$)/.test(response.url)) optional.push({source: 'network', url: response.url, status: response.status});
      else errors.push({kind: 'http-resource-error', url: response.url, status: response.status, resourceType: message.params.type});
    } else if (message.method === 'Network.loadingFailed') {
      const request = message.params;
      if (request.requestId) errors.push({kind: 'network-loading-failed', request});
    }
  });
  function send(method, params = {}) {
    const id = ++sequence;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 20000);
      pending.set(id, {resolve, reject, timer, method});
      ws.send(JSON.stringify({id, method, params}));
    });
  }
  async function evaluate(expression) {
    const response = await send('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
    check(!response.exceptionDetails, `Browser evaluation failed: ${JSON.stringify(response.exceptionDetails)}`);
    return response.result?.value;
  }
  return {send, evaluate, errors, optional};
}

export async function waitForBrowserPage(client, url, rich = false) {
  for (let attempt = 0; attempt < 300; attempt += 1) {
    const ready = await client.evaluate(`location.href === ${JSON.stringify(url)} && document.readyState === 'complete'${rich ? ' && typeof Plotly !== "undefined" && Boolean(document.getElementById("chart-spatial")?._fullLayout) && Boolean(document.getElementById("chart-metrics")?._fullLayout)' : ''}`);
    if (ready) {
      await client.evaluate('new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
      return;
    }
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  fail(`Browser page did not become ready: ${url}`);
}

export async function captureScreenshot(client, outputDir, name) {
  const {cssContentSize} = await client.send('Page.getLayoutMetrics');
  const {data} = await client.send('Page.captureScreenshot', {
    format: 'png',
    captureBeyondViewport: true,
    clip: {x: 0, y: 0, width: cssContentSize.width, height: cssContentSize.height, scale: 1},
  });
  const path = join(outputDir, name);
  writeFileSync(path, Buffer.from(data, 'base64'), {flag: 'wx'});
  return {file: name, sha256: sha256(readFileSync(path)), bytes: statSync(path).size};
}

export async function checkFlightInteractions(client, record, label) {
  const info = await client.evaluate(`(()=>({
    corrections: reportData.flightAnnotations?.corrections || [],
    modes: [...document.querySelectorAll('[data-mode]')].map(button => button.dataset.mode),
    spatial: document.getElementById('chart-spatial').data.map(trace => ({name: trace.name, x: trace.x, y: trace.y, visible: trace.visible, marker: trace.marker})),
    metricShapes: document.getElementById('chart-metrics').layout.shapes || [],
    metricAnnotations: document.getElementById('chart-metrics').layout.annotations || [],
    selectorCount: document.querySelectorAll('[data-select-correction]').length,
    toggle: Boolean(document.getElementById('flight-handoffs-visible')),
  }))()`).then(value => value);
  assertTypedEqual(info.modes, ['mission', 'guidance', 'speed', 'throttle', 'vectors'], `${label} rich mode toolbar`);
  check(info.corrections.length === record.correction_count, `${label}: visible correction count differs from accepted raw flight`);
  if (record.correction_count === 0) {
    check(!info.toggle && info.selectorCount === 0, `${label}: direct clear detail unexpectedly has handoff controls`);
    return {corrections: 0, plotly_ready: true};
  }
  check(info.toggle && info.selectorCount === record.correction_count, `${label}: correction toggle/selector controls are missing`);
  const traceIndex = info.spatial.findIndex(trace => trace.name === 'Waypoint handoffs');
  check(traceIndex >= 0, `${label}: handoff marker trace is missing from spatial plot`);
  const trace = info.spatial[traceIndex];
  check(trace.x.length === record.correction_count && trace.y.length === record.correction_count, `${label}: spatial plot is missing an H marker`);
  assertTypedEqual(trace.x, info.corrections.map(correction => correction.handoff.positionM.x), `${label} H marker spatial X coordinates`);
  assertTypedEqual(trace.y, info.corrections.map(correction => correction.handoff.positionM.y), `${label} H marker spatial Y coordinates`);
  const expectedTimes = info.corrections.map(correction => correction.handoff.simTimeS);
  for (const time of expectedTimes) {
    check(info.metricShapes.some(shape => shape.x0 === time && shape.x1 === time), `${label}: H marker ${time} is missing from metrics plot`);
  }
  for (let index = 0; index < record.correction_count; index += 1) {
    check(info.metricAnnotations.some(annotation => annotation.text === `H${index + 1}`), `${label}: H${index + 1} label is missing from metrics plot`);
  }
  const shapesBefore = await client.evaluate(`document.getElementById('chart-metrics').layout.shapes.length`);
  await client.evaluate(`document.getElementById('flight-handoffs-visible').click()`);
  check(await client.evaluate(`document.getElementById('chart-spatial').data[${traceIndex}].visible === false`), `${label}: handoff toggle did not hide spatial markers`);
  check(await client.evaluate(`document.getElementById('chart-metrics').layout.shapes.length < ${shapesBefore}`), `${label}: handoff toggle did not hide metric guides`);
  await client.evaluate(`document.getElementById('flight-handoffs-visible').click()`);
  check(await client.evaluate(`document.getElementById('chart-spatial').data[${traceIndex}].visible === true`), `${label}: handoff toggle did not restore spatial markers`);
  check(await client.evaluate(`document.getElementById('chart-metrics').layout.shapes.length === ${shapesBefore}`), `${label}: handoff toggle did not restore metric guides`);
  const selected = record.correction_count > 1 ? record.correction_count - 1 : 0;
  await client.evaluate(`document.querySelector('[data-select-correction="${selected}"]').click()`);
  check(await client.evaluate(`document.querySelector('[data-select-correction="${selected}"]').getAttribute('aria-pressed') === 'true'`), `${label}: selected correction is not announced`);
  if (record.correction_count > 1) {
    const sizes = await client.evaluate(`document.getElementById('chart-spatial').data[${traceIndex}].marker.size`);
    check(sizes[selected] > sizes[0], `${label}: selected H marker is not emphasized in spatial plot`);
    check(await client.evaluate(`document.getElementById('chart-metrics').layout.annotations.some(a => a.text === 'H${selected + 1}')`), `${label}: selected H marker is not emphasized in metrics plot`);
  }
  return {corrections: record.correction_count, plotly_ready: true, spatial_handoffs: trace.x.length, metric_handoff_guides: expectedTimes.length, toggle_and_select_passed: true};
}

async function runBrowserChecks(batch, crawl, outputDir, cdpUrl) {
  const cdp = localHttpUrl(cdpUrl, '--cdp-url');
  const tabsResponse = await fetch(new URL('/json/list', cdp), {signal: AbortSignal.timeout(10000)});
  check(tabsResponse.ok, `CDP target list returned HTTP ${tabsResponse.status}`);
  const tabs = await tabsResponse.json();
  const tab = tabs.find(item => item.type === 'page');
  check(tab?.webSocketDebuggerUrl, 'no existing page target in caller-owned CDP browser');
  const ws = new WebSocket(tab.webSocketDebuggerUrl);
  await new Promise((resolvePromise, reject) => {
    ws.addEventListener('open', resolvePromise, {once: true});
    ws.addEventListener('error', reject, {once: true});
  });
  const client = cdpClient(ws);
  const screenshots = [];
  const checks = [];
  const caseById = new Map(batch.cases.map(record => [record.case_id, record]));
  const firstWhere = predicate => batch.cases.find(predicate);
  const examples = [
    {kind: 'clear', record: firstWhere(record => record.group === 'clear' && record.status === 'simulated' && record.correction_count === 0)},
    {kind: 'one-handoff', record: firstWhere(record => record.status === 'simulated' && record.correction_count === 1)},
    {kind: 'multi-handoff', record: firstWhere(record => record.status === 'simulated' && record.correction_count >= 2)},
    {kind: 'unsupported', record: firstWhere(record => record.status === 'preflight_rejected')},
  ];
  check(examples.every(item => item.record), 'browser audit could not select clear/one/multiple/unsupported page examples');
  const pages = [
    {kind: 'reports-home', url: crawl.root_url, rich: false},
    {kind: 'waypoint-topic', url: crawl.waypoint_topic_url, rich: false},
    {kind: 'stable-batch', url: crawl.stable_batch_url, rich: false},
    ...examples.map(item => ({kind: item.kind, case_id: item.record.case_id, url: new URL(item.record.annotated_report_path, crawl.capture_base_url).href, rich: item.record.status === 'simulated', record: item.record})),
  ];
  try {
    await client.send('Runtime.enable');
    await client.send('Log.enable');
    await client.send('Network.enable');
    await client.send('Page.enable');
    for (const width of [{name: 'desktop', width: 1440, height: 1000, mobile: false}, {name: 'mobile', width: 390, height: 844, mobile: true}]) {
      await client.send('Emulation.setDeviceMetricsOverride', {width: width.width, height: width.height, deviceScaleFactor: 1, mobile: width.mobile});
      for (const page of pages) {
        await client.send('Page.navigate', {url: page.url});
        await waitForBrowserPage(client, page.url, page.rich);
        const dimensions = await client.evaluate(`({title: document.title, heading: document.querySelector('h1')?.textContent?.trim() ?? '', clientWidth: document.documentElement.clientWidth, scrollWidth: Math.max(document.documentElement.scrollWidth, document.body?.scrollWidth || 0), plotly: typeof Plotly !== 'undefined', spatialReady: Boolean(document.getElementById('chart-spatial')?._fullLayout), metricsReady: Boolean(document.getElementById('chart-metrics')?._fullLayout)})`);
        check(dimensions.heading.length > 0, `${page.kind}/${width.name}: missing page heading`);
        check(dimensions.scrollWidth <= dimensions.clientWidth + 1, `${page.kind}/${width.name}: horizontal overflow ${dimensions.scrollWidth}px > ${dimensions.clientWidth}px`);
        if (page.rich) check(dimensions.plotly && dimensions.spatialReady && dimensions.metricsReady, `${page.kind}/${width.name}: Plotly charts are not ready`);
        let interaction = null;
        if (page.kind === 'clear' || page.kind === 'one-handoff' || page.kind === 'multi-handoff') {
          interaction = await checkFlightInteractions(client, page.record, `${page.case_id}/${width.name}`);
        } else if (page.kind === 'unsupported') {
          const honest = await client.evaluate(`(()=>{const text=document.body.innerText.toLowerCase();const hasData=typeof reportData!=='undefined';return {text, charts:Boolean(document.getElementById('chart-spatial')||document.getElementById('chart-metrics')), annotations:hasData&&Boolean(reportData.flightAnnotations)}})()`);
          check(honest.text.includes('unsupported'), `${page.case_id}: browser diagnostic omits unsupported status`);
          check(honest.text.includes('not simulated') || honest.text.includes('no flight') || honest.text.includes('no simulator trajectory'), `${page.case_id}: browser diagnostic implies a flight`);
          check(!honest.charts && !honest.annotations, `${page.case_id}: browser diagnostic fabricates charts or flight annotations`);
        }
        const shot = `${width.name}-${page.kind}${page.case_id ? `-${page.case_id}` : ''}.png`;
        screenshots.push(await captureScreenshot(client, outputDir, shot));
        checks.push({kind: page.kind, case_id: page.case_id ?? null, width: width.name, url: page.url, dimensions, interaction});
      }
    }
    check(client.errors.length === 0, `browser page/asset errors: ${JSON.stringify(client.errors)}`);
    return {
      schema_id: 'planner_v2_batch_browser_checks_v1',
      user_agent: await client.evaluate('navigator.userAgent'),
      target_url: tab.url,
      pages_per_width: pages.length,
      screenshots,
      checks,
      browser_errors: client.errors,
      optional_favicon_diagnostics: client.optional,
      scope: 'Local desktop/mobile rendering and existing Plotly handoff interaction checks; no flight or product acceptance evidence.',
    };
  } finally {
    ws.close();
  }
}

function parseArgs(argv) {
  const values = new Map();
  const allowed = new Set(['--batch-root', '--output-dir', '--root-url', '--cdp-url', '--practical-archive-root', '--fresh-archive-root']);
  for (let index = 2; index < argv.length; index += 1) {
    const key = argv[index];
    check(allowed.has(key), `unknown option ${key}`);
    check(index + 1 < argv.length && !argv[index + 1].startsWith('--'), `${key} requires a value`);
    check(!values.has(key), `duplicate option ${key}`);
    values.set(key, argv[++index]);
  }
  check(values.has('--batch-root') && values.has('--output-dir'), 'Usage: node scripts/check-planner-v2-batch.mjs --batch-root CAPTURE --output-dir NEW_CHECK_DIR [--root-url http://127.0.0.1:8000/] [--cdp-url http://127.0.0.1:9225] [--practical-archive-root PATH] [--fresh-archive-root PATH]');
  check(!values.has('--cdp-url') || values.has('--root-url'), '--cdp-url requires --root-url so browser routes bind to this capture');
  return {
    batchRoot: resolve(values.get('--batch-root')),
    outputDir: resolve(values.get('--output-dir')),
    rootUrl: values.get('--root-url') ?? null,
    cdpUrl: values.get('--cdp-url') ?? null,
    practicalArchive: resolve(ROOT, values.get('--practical-archive-root') ?? DEFAULT_PRACTICAL_ARCHIVE),
    freshArchive: resolve(ROOT, values.get('--fresh-archive-root') ?? DEFAULT_FRESH_ARCHIVE),
  };
}

function createOutputDir(outputDir, protectedRoots) {
  check(!lstatSync(outputDir, {throwIfNoEntry: false}), `output directory already exists: ${outputDir}`);
  check(protectedRoots.every(root => !inside(root, outputDir) && !inside(outputDir, root)), 'output directory overlaps the batch, accepted archives, or frozen input tree');
  mkdirSync(outputDir, {recursive: true});
}

async function run(options) {
  const batchRoot = realpathSync(options.batchRoot);
  const receipt = {
    schema_id: 'planner_v2_batch_acceptance_v1',
    status: 'running',
    batch_root: batchRoot,
    output_dir: options.outputDir,
    source_root: ROOT,
    started_at: new Date().toISOString(),
    checks: {},
    errors: [],
  };
  let sourceFingerprints;
  let archiveFingerprints;
  let captureFingerprint;
  try {
    const inputs = readPackInputs();
    const frozenAbsolutePaths = inputs.frozen_paths.map(path => resolve(ROOT, path));
    sourceFingerprints = Object.fromEntries(frozenAbsolutePaths.map(path => [relative(ROOT, path), sha256(readFileSync(path))]));
    archiveFingerprints = {
      practical: hashTree(options.practicalArchive),
      fresh: hashTree(options.freshArchive),
    };
    captureFingerprint = hashTree(batchRoot, {includeInventory: true});
    receipt.source_fixture_sha256 = inputs.source_fixture_hashes;
    receipt.source_manifest_sha256 = inputs.source_manifest_hashes;
    receipt.registered_pack_sha256 = inputs.pack_sha256;
    receipt.accepted_archive_roots = {
      practical: {path: options.practicalArchive, ...archiveFingerprints.practical},
      fresh: {path: options.freshArchive, ...archiveFingerprints.fresh},
    };
    receipt.capture_inventory = captureFingerprint.inventory;

    const practical = readArchive(options.practicalArchive, 'waypoint_v2_practical_suite_run_v1', 'practical accepted archive');
    practical.root = options.practicalArchive;
    const fresh = readArchive(options.freshArchive, 'waypoint_v2_fresh_terrain_readiness_run_v1', 'fresh accepted archive');
    fresh.root = options.freshArchive;
    check(practical.summary.suite_sha256 === inputs.source_fixture_hashes.practical_suite, 'practical accepted archive does not bind the frozen practical fixture');
    check(fresh.summary.fixture_sha256 === inputs.source_fixture_hashes.fresh_terrain_inputs, 'fresh accepted archive does not bind the frozen fresh fixture');
    const accepted = expectedArchiveCases(inputs, practical, fresh);
    const expectedIds = inputs.expectedCases.map(item => item.case_id);
    assertTypedEqual([...practical.cases.keys()], inputs.plan.cases.map(item => item.id), 'practical accepted stable case order');
    assertTypedEqual([...fresh.cases.keys()], inputs.fresh.cases.map(item => item.id), 'fresh accepted stable case order');

    const batchSummaryPath = join(batchRoot, 'summary.json');
    const batch = readJson(batchSummaryPath);
    check(batch.schema_id === 'planner_v2_eval_batch_v1', `unexpected native batch schema ${batch.schema_id}`);
    check(batch.pack_id === 'planner_v2_lab_suite' && batch.status === 'completed', 'batch identity/status is not completed planner_v2_lab_suite');
    check(batch.policy_version === 3 && batch.case_count === 44 && Array.isArray(batch.cases) && batch.cases.length === 44, 'native batch must contain exactly 44 policy-3 cases');
    assertTypedEqual(batch.cases.map(record => record.case_id), expectedIds, '44 stable case IDs in practical-then-fresh order');
    check(batch.input_identity.rust_typed_expanded_input_count === 44, 'Rust typed-input provenance count is not 44');
    check(isSha256(batch.input_identity.pack_file_sha256) && batch.input_identity.pack_file_sha256 === inputs.pack_sha256, 'batch pack file digest does not match the frozen pack');
    assertTypedEqual(batch.input_identity.source_fixture_sha256, inputs.source_fixture_hashes, 'batch frozen fixture provenance');
    assertTypedEqual(batch.input_identity.source_manifest_sha256, inputs.source_manifest_hashes, 'batch frozen source-manifest provenance');
    check(isSha256(batch.input_identity.rust_typed_expanded_inputs_sha256), 'batch Rust typed-input digest is malformed');
    check(batch.input_identity.digest_scheme === 'sha256(serde_json::to_vec(ordered ExpandedDigestItem[]))', 'batch Rust typed-input digest scheme changed or is undocumented');
    check(batch.provenance?.unchanged_during_capture === true, 'native batch source provenance does not prove unchanged capture inputs');
    assertTypedEqual(batch.provenance.source_before, batch.provenance.source_after, 'native batch source before/after provenance');
    check(isSha256(batch.provenance.source_before?.rust_source_tree_sha256), 'native batch source tree digest is missing');
    check(isSha256(batch.provenance.source_before?.executable_sha256), 'native batch executable digest is missing');

    const expandedInputEvidence = validateExpandedInputSnapshot(batchRoot, batch, inputs, accepted);
    const artifactInventory = {};
    for (let index = 0; index < batch.cases.length; index += 1) {
      validateBatchRecord(batch.cases[index], inputs.expectedCases[index], batchRoot, accepted[index], artifactInventory);
    }
    validateBatchSummary(batch, accepted);
    check(batch.cases.filter(record => record.status === 'simulated').length === 42, 'expected 42 simulated cases');
    check(batch.cases.filter(record => record.status === 'preflight_rejected').length === 2, 'expected two unsupported preflight diagnostic pages');
    check(lstatSync(join(batchRoot, 'index.html'), {throwIfNoEntry: false})?.isFile(), 'native batch root index.html is missing');
    check(lstatSync(join(batchRoot, 'report.html'), {throwIfNoEntry: false})?.isFile(), 'native batch root report.html is missing');
    const indexHtml = readFileSync(join(batchRoot, 'index.html'), 'utf8');
    for (const id of expectedIds) check(indexHtml.includes(id), `batch index.html omits case ID ${id}`);
    for (const item of accepted.filter(value => !value.supported)) {
      const record = batch.cases[expectedIds.indexOf(item.case_id)];
      const pagePath = resolveCaptureFile(batchRoot, record.annotated_report_path, `${item.case_id} diagnostic page`);
      validateDiagnosticPage(readFileSync(pagePath, 'utf8'), item.case_id, record);
    }

    receipt.checks = {
      expected_ids: expectedIds.length,
      practical_ids: inputs.plan.cases.length,
      fresh_ids: inputs.fresh.cases.length,
      simulated: 42,
      unsupported_diagnostics: 2,
      typed_scenarios_compared: 44,
      exact_raw_flights_compared: 44,
      unsupported_flight_records_checked_for_no_manifest_or_ordinary_data: 2,
      rich_reports_compared_beyond_only_flightAnnotations: 42,
      integrity_passed: batch.summary.integrity_passed_count,
      final_source_replay_passed: batch.summary.final_source_replay_passed_count,
      expanded_inputs_snapshot_sha256: expandedInputEvidence.expanded_input_sha256,
      group_counts: Object.fromEntries(Object.keys(EXPECTED_GROUP_COUNTS).map(key => [key, batch.summary[key]])),
      core_landing_count: batch.summary.valid_landing_count,
      diagnostic_landing_count: batch.summary.diagnostic_landing_count,
      total_target_landings: batch.summary.valid_landing_count + batch.summary.diagnostic_landing_count,
      core_non_landing_count: batch.summary.non_landing_count,
      diagnostic_finite_no_clearing_count: batch.summary.diagnostic_non_landing_count,
      finite_no_clearing_count: batch.summary.planning_stops.no_clearing ?? 0,
      batch_result: batch.summary,
      case_artifact_sha256: artifactInventory,
    };
    if (options.rootUrl) receipt.http_crawl = await crawlReportSite(batchRoot, batch, options.rootUrl);
    if (options.cdpUrl) receipt.browser = await runBrowserChecks(batch, receipt.http_crawl, options.outputDir, options.cdpUrl);

    const currentSources = Object.fromEntries(frozenAbsolutePaths.map(path => [relative(ROOT, path), sha256(readFileSync(path))]));
    const currentPractical = hashTree(options.practicalArchive);
    const currentFresh = hashTree(options.freshArchive);
    const currentCapture = hashTree(batchRoot);
    receipt.preservation = {
      frozen_inputs_unchanged: isDeepStrictEqual(sourceFingerprints, currentSources),
      practical_archive_unchanged: currentPractical.sha256 === archiveFingerprints.practical.sha256,
      fresh_archive_unchanged: currentFresh.sha256 === archiveFingerprints.fresh.sha256,
      capture_unchanged: currentCapture.sha256 === captureFingerprint.sha256,
      frozen_inputs_sha256: sourceFingerprints,
      archive_tree_sha256_before_after: {
        practical: [archiveFingerprints.practical.sha256, currentPractical.sha256],
        fresh: [archiveFingerprints.fresh.sha256, currentFresh.sha256],
      },
      capture_tree_sha256_before_after: [captureFingerprint.sha256, currentCapture.sha256],
    };
    check(receipt.preservation.frozen_inputs_unchanged && receipt.preservation.practical_archive_unchanged
      && receipt.preservation.fresh_archive_unchanged && receipt.preservation.capture_unchanged,
    'frozen inputs, accepted archives, or source capture changed during read-only acceptance');
    receipt.status = 'passed';
  } catch (error) {
    receipt.status = 'failed';
    receipt.errors.push({message: error.message, stack: error.stack});
    try {
      if (sourceFingerprints && archiveFingerprints && captureFingerprint) {
        const inputs = readPackInputs();
        const frozenAbsolutePaths = inputs.frozen_paths.map(path => resolve(ROOT, path));
        const currentSources = Object.fromEntries(frozenAbsolutePaths.map(path => [relative(ROOT, path), sha256(readFileSync(path))]));
        const currentPractical = hashTree(options.practicalArchive);
        const currentFresh = hashTree(options.freshArchive);
        const currentCapture = hashTree(batchRoot);
        receipt.preservation = {
          frozen_inputs_unchanged: isDeepStrictEqual(sourceFingerprints, currentSources),
          practical_archive_unchanged: currentPractical.sha256 === archiveFingerprints.practical.sha256,
          fresh_archive_unchanged: currentFresh.sha256 === archiveFingerprints.fresh.sha256,
          capture_unchanged: currentCapture.sha256 === captureFingerprint.sha256,
          frozen_inputs_sha256: sourceFingerprints,
          archive_tree_sha256_before_after: {practical: [archiveFingerprints.practical.sha256, currentPractical.sha256], fresh: [archiveFingerprints.fresh.sha256, currentFresh.sha256]},
          capture_tree_sha256_before_after: [captureFingerprint.sha256, currentCapture.sha256],
        };
      }
    } catch (preservationError) {
      receipt.preservation_error = preservationError.message;
    }
  }
  receipt.finished_at = new Date().toISOString();
  const receiptPath = join(options.outputDir, 'acceptance.json');
  writeFileSync(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`, {flag: 'wx'});
  return {receipt, receiptPath};
}

async function main() {
  let options;
  try {
    options = parseArgs(process.argv);
    const outputDir = options.outputDir;
    const protectedRoots = [options.batchRoot, options.practicalArchive, options.freshArchive, join(ROOT, 'fixtures/research'), join(ROOT, 'fixtures/packs')].map(path => resolve(path));
    createOutputDir(outputDir, protectedRoots);
    const outcome = await run(options);
    console.log(JSON.stringify({status: outcome.receipt.status, output_dir: outputDir, receipt: outcome.receiptPath,
      simulated: outcome.receipt.checks?.simulated ?? null, failures: outcome.receipt.errors.length}));
    if (outcome.receipt.status !== 'passed') process.exitCode = 1;
  } catch (error) {
    console.error(error.stack ?? error.message);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
