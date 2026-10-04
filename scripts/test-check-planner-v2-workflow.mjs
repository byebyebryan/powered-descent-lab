import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, symlinkSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {extractAcceptance, safeFile, comparableCompactSummary, compareCaptures, verifySavedExecution, verifyVisibleRow} from './check-planner-v2-workflow.mjs';
import {validateBrowserEndpoints, resolveReportPath} from './check-planner-v2-browser.mjs';

test('browser checks require local debugging and the declared LAN report host', () => {
  assert.equal(validateBrowserEndpoints('http://192.168.1.110:8000/', 'http://127.0.0.1:9229/').root.hostname, '192.168.1.110');
  for (const [root, cdp] of [
    ['https://127.0.0.1:8000/', 'http://localhost:9229/'],
    ['http://example.com/', 'http://localhost:9229/'],
    ['http://localhost:8000/', 'http://example.com:9229/'],
    ['http://user:secret@localhost:8000/', 'http://localhost:9229/'],
  ]) assert.throws(() => validateBrowserEndpoints(root, cdp));
});

test('served receipt paths cannot cause off-host requests or escape via encoded traversal', () => {
  const root = new URL('http://127.0.0.1:8000/');
  assert.equal(resolveReportPath(root, '/eval/planner_v2_lab_suite/capture-123/summary.json').origin, root.origin);
  for (const path of ['https://example.com/', '//example.com/path', '/../outside', '/%2e%2e/outside',
    '/eval/%2Foutside', '/eval//data', '/eval/data?query=1', '/eval/data#fragment', '/eval\\outside']) {
    assert.throws(() => resolveReportPath(root, path), path);
  }
});

test('zero-command reports allow honest finite stops, never an invented flight or landing', () => {
  const result = {planning_stop: 'no_clearing', physical_outcome: 'flying', mission_outcome: 'in_progress',
    ordinary_flight: {actions: [], samples: [{physics_step: 0}]},
    manifest: {physics_steps: 0, controller_updates: 0, sim_time_s: 0}};
  for (const stop of ['no_nominal', 'nominal_rejected', 'no_clearing', 'correction_limit', 'deadline', 'no_progress']) {
    verifySavedExecution({...result, planning_stop: stop}, stop);
  }
  assert.throws(() => verifySavedExecution({...result, planning_stop: 'landed'}, 'fabricated landing'));
  assert.throws(() => verifySavedExecution({...result, manifest: {...result.manifest, physics_steps: 1}}, 'fabricated flight'));
});

test('acceptance payload requires an explicit visible verdict and stable schema', () => {
  const verdict = {schema_id: 'planner_v2_acceptance_v1', passed: true, issues: []};
  const page = `<strong id="planner-v2-acceptance-status">PASSED</strong><script id="planner-v2-acceptance" type="application/json">${JSON.stringify(verdict)}</script>`;
  assert.deepEqual(extractAcceptance(page), verdict);
  assert.throws(() => extractAcceptance(page.replace('PASSED', 'completed')));
  assert.throws(() => extractAcceptance(page.replace('id="planner-v2-acceptance-status"', 'id="other"') + '<!-- PASSED -->'));
  assert.throws(() => extractAcceptance(page.replace('planner_v2_acceptance_v1', 'foreign')));
  assert.throws(() => extractAcceptance(page.replace('application/json', 'text/javascript')));
  const failed = page.replace('PASSED', 'FAILED').replace('"passed":true', '"passed":false');
  assert.equal(extractAcceptance(failed).passed, false);
});

test('visible report metrics and outcome must agree with raw projected evidence', () => {
  const expected = {case_id: 'synthetic', display_outcome: 'Landed on target',
    fuel_used_pct: 51.25, flight_s: 20.005, landing_offset_m: 0.0021, planning_s: 0.0012};
  const row = '<td>synthetic</td><td>Landed on target<span>Integrity / replay: true · planning 0.001s</span></td>'
    + '<td>51.3%</td><td>20.00s</td><td>0.002m</td><td>—</td><td>preview</td><td>2 H</td>';
  verifyVisibleRow(row, expected);
  for (const [before, after] of [['Landed on target', 'Stopped'], ['51.3%', '51.4%'],
    ['20.00s', '20.01s'], ['0.002m', '0.003m'], ['0.001s', '0.002s']]) {
    assert.throws(() => verifyVisibleRow(row.replace(before, after), expected));
  }
});

test('artifact paths reject traversal, absolute paths, symlinks and non-files', () => {
  const root = mkdtempSync(join(tmpdir(), 'pd-v2-paths-'));
  try {
    mkdirSync(join(root, 'runs'));
    writeFileSync(join(root, 'runs', 'data.json'), '{}');
    assert.equal(safeFile(root, 'runs/data.json'), join(root, 'runs', 'data.json'));
    for (const path of ['', '../outside', '/tmp/outside', 'runs/./data.json', 'runs//data.json', 'runs\\data.json', 'runs']) {
      assert.throws(() => safeFile(root, path), path);
    }
    symlinkSync(join(root, 'runs'), join(root, 'linked'));
    assert.throws(() => safeFile(root, 'linked/data.json'));
  } finally { rmSync(root, {recursive: true}); }
});

function compact() {
  return {schema_id: 'waypoint_v2_flight_summary_v1', result: {
    planning_stop: 'landed', correction_count: 2,
    timings: {planning_s: 1, execution_s: 2, replay_s: 3},
  }, run_summary: {fuel: {remaining_kg: 42}, endpoint: {physics_step: 1200}},
  timings: {output_s: 4, total_s: 10}};
}

test('repeat exclusions are narrow: wall-clock timing may differ but physics may not', () => {
  const a = compact();
  const b = structuredClone(a);
  b.result.timings.planning_s = 99;
  b.timings.total_s = 100;
  assert.equal(comparableCompactSummary(a, b, 'repeat').length, 5);
  assert.equal(a.result.timings.planning_s, 1, 'comparison must not mutate its input');
  for (const mutate of [
    x => { x.result.correction_count += 1; },
    x => { x.run_summary.fuel.remaining_kg -= 1; },
    x => { x.run_summary.endpoint.physics_step += 1; },
    x => { x.result.timings.unrecognized_s = 5; },
    x => { delete x.result.timings.replay_s; },
    x => { x.timings.total_s = '100'; },
  ]) {
    const bad = structuredClone(b);
    mutate(bad);
    assert.throws(() => comparableCompactSummary(a, bad, 'repeat'));
  }
});

// Synthetic JSON exercises the comparison, not native acceptance or flight.
function repeatFixture(root, offset = 0) {
  mkdirSync(root);
  const source = {git_commit: 'synthetic', git_dirty: false,
    rust_source_tree_sha256: 'synthetic-source', executable_sha256: 'synthetic-executable'};
  const cases = Array.from({length: 44}, (_, index) => {
    const id = `synthetic_${index}`;
    const run = join(root, 'runs', id);
    mkdirSync(run, {recursive: true});
    const summary = compact();
    summary.result.timings.planning_s += offset;
    summary.timings.total_s += offset;
    writeFileSync(join(run, 'summary.json'), JSON.stringify(summary));
    writeFileSync(join(run, 'scenario.json'), JSON.stringify({id, initial: {x: index}}));
    writeFileSync(join(run, 'flight.json'), JSON.stringify({
      timings: {planning_s: 1 + offset, execution_s: 2, replay_s: 3},
      segments: [{updates: [{physics_step: 2, throttle: 0.5}]}],
    }));
    return {case_id: id, status: 'preflight_rejected',
      scenario_path: `runs/${id}/scenario.json`, flight_path: `runs/${id}/flight.json`,
      summary_path: `runs/${id}/summary.json`, planning_s: 1 + offset,
      execution_s: 2, replay_s: 3, artifact_sha256: {synthetic: String(offset)}};
  });
  writeFileSync(join(root, 'summary.json'), JSON.stringify({
    schema_id: 'planner_v2_eval_batch_v1', pack_id: 'planner_v2_lab_suite',
    status: 'completed', policy_version: 3, case_count: 44, cases,
    input_identity: {synthetic: true}, summary: {synthetic: 44},
    provenance: {source_before: source, source_after: source, unchanged_during_capture: true},
  }));
  writeFileSync(join(root, 'pack.json'), '{"synthetic":true}');
  writeFileSync(join(root, 'expanded-inputs.json'), '[]');
}

test('repeat comparison reads every case and does not alter either capture', () => {
  const root = mkdtempSync(join(tmpdir(), 'pd-v2-repeat-'));
  const a = join(root, 'a');
  const b = join(root, 'b');
  try {
    repeatFixture(a);
    repeatFixture(b, 100);
    const before = readFileSync(join(b, 'runs/synthetic_43/flight.json'));
    const result = compareCaptures(a, b);
    assert.equal(result.compared_case_json_artifacts, 132);
    assert.equal(result.case_count, 44);
    assert.deepEqual(readFileSync(join(b, 'runs/synthetic_43/flight.json')), before);
    assert.throws(() => compareCaptures(a, a), /distinct/);
    const flight = JSON.parse(before);
    flight.segments[0].updates[0].throttle = 0.6;
    writeFileSync(join(b, 'runs/synthetic_43/flight.json'), JSON.stringify(flight));
    assert.throws(() => compareCaptures(a, b), /synthetic_43/);
  } finally { rmSync(root, {recursive: true}); }
});

test('repeat source drift and input changes are not timing exclusions', () => {
  const root = mkdtempSync(join(tmpdir(), 'pd-v2-repeat-binding-'));
  const a = join(root, 'a');
  const b = join(root, 'b');
  try {
    repeatFixture(a);
    repeatFixture(b);
    const path = join(b, 'summary.json');
    const saved = JSON.parse(readFileSync(path, 'utf8'));
    for (const mutate of [
      x => { x.input_identity.synthetic = false; },
      x => { x.provenance.source_before.executable_sha256 = 'other'; },
      x => { x.provenance.unchanged_during_capture = false; },
      x => { x.cases[43].case_id = x.cases[42].case_id; },
      x => { x.case_count = 43; },
    ]) {
      const bad = structuredClone(saved);
      mutate(bad);
      writeFileSync(path, JSON.stringify(bad));
      assert.throws(() => compareCaptures(a, b));
    }
  } finally { rmSync(root, {recursive: true}); }
});
