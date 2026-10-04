#!/usr/bin/env node
// Maintained read-only checks for the selected native V2 common report and a
// same-source repeat. Historical checkpoint scripts remain unchanged.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {lstatSync, readFileSync, realpathSync} from 'node:fs';
import {dirname, isAbsolute, join, relative, resolve, sep} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
import {assertSharedBatch, assertSharedSummary} from './check-planner-v2-common.mjs';
import {expectedProjection, extractProjection} from './check-planner-v2-tree.mjs';
import {
  comparableFlight, extractReportData, verifyCorrectionAnnotations,
} from './check-planner-v2-batch.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PACK = 'planner_v2_lab_suite';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const json = path => JSON.parse(readFileSync(path, 'utf8'));

export function safeFile(root, value) {
  assert(typeof value === 'string' && value.length > 0, 'missing relative file path');
  assert(!isAbsolute(value) && !value.includes('\\'), 'absolute or backslash file path');
  const pieces = value.split('/');
  assert(pieces.every(p => p && p !== '.' && p !== '..'), 'unsafe relative file path');
  let current = root;
  for (const piece of pieces) {
    current = join(current, piece);
    assert(!lstatSync(current).isSymbolicLink(), `symlink in artifact path: ${value}`);
  }
  assert(lstatSync(current).isFile(), `not a regular artifact: ${value}`);
  return current;
}

export function extractAcceptance(html) {
  const match = html.match(/<script\b(?=[^>]*\bid="planner-v2-acceptance")(?=[^>]*\btype="application\/json")[^>]*>(.*?)<\/script>/s);
  assert(match, 'missing structured native acceptance verdict');
  const verdict = JSON.parse(match[1]);
  assert.equal(verdict.schema_id, 'planner_v2_acceptance_v1');
  assert.equal(typeof verdict.passed, 'boolean');
  assert(Array.isArray(verdict.issues), 'missing acceptance issues');
  const status = html.match(/<strong\b(?=[^>]*\bid="planner-v2-acceptance-status")[^>]*>\s*([^<]*)\s*<\/strong>/);
  assert(status, 'missing visible acceptance status element');
  assert.equal(status[1].trim(), verdict.passed ? 'PASSED' : 'FAILED', 'visible acceptance disagrees with structured verdict');
  return verdict;
}

export function nativeAcceptance(evaluator, capture) {
  const result = spawnSync(evaluator, ['check-planner-v2', '--dir', capture], {
    encoding: 'utf8', maxBuffer: 8 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, `native acceptance failed: ${result.stderr}\n${result.stdout}`);
  const verdict = JSON.parse(result.stdout);
  assert.equal(verdict.schema_id, 'planner_v2_acceptance_v1');
  assert.equal(verdict.passed, true, 'capture was not accepted');
  return verdict;
}

export function verifySavedExecution(flight, label) {
  const ordinary = flight.ordinary_flight;
  assert(flight.manifest && Array.isArray(ordinary?.actions), `${label}: missing simulated execution`);
  assert(Array.isArray(ordinary.samples) && ordinary.samples.length > 0, `${label}: missing samples`);
  if (ordinary.actions.length === 0) {
    assert(['no_nominal', 'nominal_rejected', 'no_clearing', 'correction_limit', 'deadline', 'no_progress'].includes(flight.planning_stop), `${label}: zero-command outcome is not a finite planning stop`);
    assert.equal(flight.physical_outcome, 'flying');
    assert.equal(flight.mission_outcome, 'in_progress');
    assert.equal(flight.manifest.physics_steps, 0);
    assert.equal(flight.manifest.controller_updates, 0);
    assert.equal(flight.manifest.sim_time_s, 0);
    assert.equal(ordinary.samples.length, 1);
    assert.equal(ordinary.samples[0].physics_step, 0);
  }
}

export function verifyVisibleRow(rowHtml, expected) {
  const cells = [...rowHtml.matchAll(/<td\b[^>]*>(.*?)<\/td>/gs)]
    .map(m => m[1].replace(/<[^>]*>/g, ''));
  assert.equal(cells.length, 8, `${expected.case_id}: common tree column count`);
  assert(cells[1].includes(expected.display_outcome), `${expected.case_id}: visible outcome`);
  const metric = (value, unit, digits) => value === null ? '—' : `${value.toFixed(digits)}${unit}`;
  for (const [column, value, unit, digits] of [
    [2, expected.fuel_used_pct, '%', 1], [3, expected.flight_s, 's', 2],
    [4, expected.landing_offset_m, 'm', 3],
  ]) assert.equal(cells[column], metric(value, unit, digits), `${expected.case_id}: visible metric column ${column}`);
  assert(cells[1].includes('planning ' + metric(expected.planning_s, 's', 3)), `${expected.case_id}: visible planning time`);
}

export function indexMissionRows(html, expectedIds) {
  const rows = [...html.matchAll(/<tr\b[^>]*data-case-id="([^"]+)"[^>]*>(.*?)<\/tr>/gs)];
  // The visible tree groups inputs by report category; source/previous/next
  // order is still checked separately. Bind every visible row by identity.
  assert.deepEqual(rows.map(m => m[1]).sort(), [...expectedIds].sort(),
    'visible mission identities differ from the captured case set');
  return new Map(rows.map(m => [m[1], m[2]]));
}

function readBatch(root) {
  const batch = json(safeFile(root, 'summary.json'));
  assert.equal(batch.schema_id, 'planner_v2_eval_batch_v1');
  assert.equal(batch.pack_id, PACK);
  assert.equal(batch.status, 'completed');
  assert.equal(batch.policy_version, 3);
  assert.equal(batch.case_count, 44);
  assert.equal(batch.cases.length, 44);
  assert.equal(new Set(batch.cases.map(c => c.case_id)).size, 44);
  return batch;
}

export function checkPublishedSite(capture, site, acceptance) {
  const batch = readBatch(capture);
  const html = readFileSync(safeFile(site, 'index.html'), 'utf8');
  assertSharedBatch(html);
  assertSharedSummary(html, batch.summary);
  const verdict = extractAcceptance(html);
  assert.deepEqual(verdict, acceptance, 'rendered acceptance differs from native checker');
  assert.equal(verdict.passed, true, 'current report must be accepted');
  const receipt = json(safeFile(site, 'render.json'));
  assert.equal(receipt.schema_id, 'planner_v2_common_report_site_v1');
  assert.equal(realpathSync(receipt.source_capture), realpathSync(capture));
  assert.equal(receipt.source_summary_sha256, hash(readFileSync(safeFile(capture, 'summary.json'))));
  assert.equal(Object.keys(receipt.page_sha256).length, 46);
  for (const [name, digest] of Object.entries(receipt.page_sha256)) {
    assert.equal(hash(readFileSync(safeFile(site, name))), digest, `site receipt: ${name}`);
  }
  const projection = batch.cases.map(c => expectedProjection(c,
    json(safeFile(capture, c.flight_path)), json(safeFile(capture, c.scenario_path))));
  assert.deepEqual(extractProjection(html), projection, 'tree projection differs from raw flights');
  const rows = indexMissionRows(html, batch.cases.map(c => c.case_id));
  let handoffs = 0;
  let rich = 0;
  for (const [index, c] of batch.cases.entries()) {
    const flight = json(safeFile(capture, c.flight_path));
    const detail = readFileSync(safeFile(site, c.annotated_report_path), 'utf8');
    const expected = projection[index];
    const visibleRow = rows.get(c.case_id);
    verifyVisibleRow(visibleRow, expected);
    assert(visibleRow.includes(`href="${c.annotated_report_path}"`));
    const markers = [...visibleRow.matchAll(/<[^>]*data-handoff="([^"]+)"[^>]*>/g)].map(m => {
      const attrs = Object.fromEntries([...m[0].matchAll(/([\w-]+)="([^"]*)"/g)].map(a => [a[1], a[2]]));
      return {number: Number(attrs['data-handoff']), x: Number(attrs['data-world-x']), y: Number(attrs['data-world-y'])};
    });
    assert.deepEqual(markers, expected.handoffs.map((p, i) => ({number: i + 1, x: p.x, y: p.y})), `${c.case_id}: handoff markers`);
    handoffs += markers.length;
    assert(detail.includes('href="../../index.html"'), `${c.case_id}: batch navigation`);
    if (c.status === 'simulated') {
      verifySavedExecution(flight, c.case_id);
      const actual = extractReportData(detail, c.case_id);
      const original = extractReportData(readFileSync(safeFile(capture, c.rich_report_path), 'utf8'), c.case_id);
      verifyCorrectionAnnotations(flight, actual, c.case_id);
      const nav = actual.flightAnnotations.navigation;
      assert.equal(nav.home.href, '/reports/');
      assert.equal(nav.collection.href, '../../index.html');
      assert.equal(nav.previous?.href ?? null, index ? `../${batch.cases[index - 1].case_id}/index.html` : null);
      assert.equal(nav.next?.href ?? null, index + 1 < batch.cases.length ? `../${batch.cases[index + 1].case_id}/index.html` : null);
      for (const name of [c.scenario_path, c.flight_path, c.summary_path, 'expanded-inputs.json']) {
        assert(nav.sourceLinks.some(link => link.href === receipt.source_base_href + name), `${c.case_id}: raw source ${name}`);
      }
      delete actual.flightAnnotations;
      delete original.flightAnnotations;
      assert.deepEqual(actual, original, `${c.case_id}: rich payload changed beyond annotations`);
      for (const name of ['chart-spatial', 'chart-metrics', 'flightStats', 'botStats', 'missionDetails', 'Speed', 'Throttle', 'Vectors']) {
        assert(detail.includes(name), `${c.case_id}: missing rich feature ${name}`);
      }
      rich += 1;
    } else {
      assert.equal(c.status, 'preflight_rejected');
      assert(detail.includes('No simulator trajectory'));
      assert(!detail.includes('const reportData = ') && !detail.includes('chart-spatial'));
    }
  }
  assert.equal(rich, 42);
  return {schema_id: 'planner_v2_workflow_report_check_v1', passed: true,
    case_count: batch.case_count, rich_payloads_preserved: rich, executed_handoffs: handoffs,
    common_template: true, native_acceptance_matches: true, receipt_pages_verified: 46};
}

export function comparableCompactSummary(left, right, label) {
  const a = structuredClone(left);
  const b = structuredClone(right);
  const exclusions = [];
  for (const [container, fields] of [
    ['result', ['planning_s', 'execution_s', 'replay_s']],
    [null, ['output_s', 'total_s']],
  ]) {
    const x = container ? a[container]?.timings : a.timings;
    const y = container ? b[container]?.timings : b.timings;
    for (const field of fields) {
      assert(Number.isFinite(x?.[field]) && Number.isFinite(y?.[field]), `${label}: invalid timing ${field}`);
      delete x[field];
      delete y[field];
      exclusions.push(`/${container ? container + '/' : ''}timings/${field}`);
    }
  }
  assert.deepEqual(a, b, `${label}: compact summary differs beyond timing allowlist`);
  return exclusions;
}

export function compareCaptures(left, right) {
  assert.notEqual(realpathSync(left), realpathSync(right), 'repeat roots must be distinct');
  const a = readBatch(left);
  const b = readBatch(right);
  assert.deepEqual(a.input_identity, b.input_identity, 'repeat input identities differ');
  assert.deepEqual(a.provenance.source_before, b.provenance.source_before, 'repeat sources/executables differ');
  for (const batch of [a, b]) {
    assert.equal(batch.provenance.unchanged_during_capture, true);
    assert.deepEqual(batch.provenance.source_before, batch.provenance.source_after);
  }
  assert.deepEqual(a.summary, b.summary, 'repeat outcome summaries differ');
  for (const name of ['pack.json', 'expanded-inputs.json']) {
    assert.deepEqual(json(safeFile(left, name)), json(safeFile(right, name)), `repeat ${name}`);
  }
  const flightExclusions = new Set();
  const summaryExclusions = new Set();
  let rich = 0;
  for (const [index, c] of a.cases.entries()) {
    const counterpart = b.cases[index];
    assert.equal(c.case_id, counterpart.case_id, 'repeat case order differs');
    const scrubRow = row => {
      const copy = structuredClone(row);
      for (const key of ['planning_s', 'execution_s', 'replay_s', 'artifact_sha256']) delete copy[key];
      return copy;
    };
    assert.deepEqual(scrubRow(c), scrubRow(counterpart), `${c.case_id}: repeat case metadata`);
    assert.deepEqual(json(safeFile(left, c.scenario_path)), json(safeFile(right, counterpart.scenario_path)), `${c.case_id}: repeat scenario`);
    for (const pointer of comparableFlight(json(safeFile(left, c.flight_path)), json(safeFile(right, counterpart.flight_path)), c.case_id)) flightExclusions.add(pointer);
    for (const pointer of comparableCompactSummary(json(safeFile(left, c.summary_path)), json(safeFile(right, counterpart.summary_path)), c.case_id)) summaryExclusions.add(pointer);
    if (c.status === 'simulated') {
      assert.deepEqual(extractReportData(readFileSync(safeFile(left, c.rich_report_path), 'utf8'), c.case_id),
        extractReportData(readFileSync(safeFile(right, counterpart.rich_report_path), 'utf8'), c.case_id), `${c.case_id}: repeat rich payload`);
      rich += 1;
    }
  }
  return {schema_id: 'planner_v2_repeat_check_v1', passed: true, case_count: 44,
    compared_case_json_artifacts: 132, compared_rich_payloads: rich,
    flight_exclusions: [...flightExclusions].sort(), compact_summary_exclusions: [...summaryExclusions].sort(),
    case_row_exclusions: ['planning_s', 'execution_s', 'replay_s', 'artifact_sha256']};
}

function selectedCapture(root) {
  const selected = json(safeFile(join(root, 'outputs/eval', PACK), 'current.json'));
  assert.equal(selected.schema_id, 'planner_v2_current_batch_v1');
  assert.equal(selected.pack_id, PACK);
  assert.equal(typeof selected.capture_dir, 'string');
  const outputs = realpathSync(join(root, 'outputs'));
  const capture = realpathSync(resolve(outputs, selected.capture_dir));
  const rel = relative(outputs, capture);
  assert(rel && rel !== '..' && !rel.startsWith('..' + sep) && !isAbsolute(rel), 'selection escapes outputs');
  return capture;
}

function main() {
  const {values} = parseArgs({options: {
    root: {type: 'string', default: ROOT}, evaluator: {type: 'string'},
    'capture-dir': {type: 'string'}, 'site-dir': {type: 'string'}, 'compare-dir': {type: 'string'},
    help: {type: 'boolean', default: false},
  }});
  if (values.help) {
    process.stdout.write('Read-only native V2 report check (no archived captures required):\n'
      + '  node scripts/check-planner-v2-workflow.mjs [--evaluator target/release/pd-eval]\n'
      + '  ... --capture-dir CAPTURE --site-dir SITE [--compare-dir SAME_SOURCE_REPEAT]\n'
      + 'No report regeneration, simulation, publication, or evidence writes.\n');
    return;
  }
  const root = realpathSync(resolve(values.root));
  const evaluator = resolve(values.evaluator ?? join(root, 'target/release/pd-eval'));
  const capture = realpathSync(resolve(values['capture-dir'] ?? selectedCapture(root)));
  const site = realpathSync(resolve(values['site-dir'] ?? join(root, 'outputs/reports/eval', PACK)));
  const acceptance = nativeAcceptance(evaluator, capture);
  const result = {report: checkPublishedSite(capture, site, acceptance)};
  if (values['compare-dir']) {
    const other = realpathSync(resolve(values['compare-dir']));
    nativeAcceptance(evaluator, other);
    result.repeat = compareCaptures(capture, other);
  }
  process.stdout.write(JSON.stringify(result, null, 2) + '\n');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); } catch (error) { process.stderr.write(`${error.stack ?? error}\n`); process.exitCode = 1; }
}
