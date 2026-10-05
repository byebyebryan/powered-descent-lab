import test from 'node:test';
import assert from 'node:assert/strict';
import {
  assertTypedEqual,
  comparableFlight,
  extractReportData,
  parseAnchors,
  verifyCorrectionAnnotations,
  verifyOrdinaryExecution,
  verifyHttpContent,
  isEvidenceAnchor,
} from './planner-v2-report-checks.mjs';

test('unsupported detail source links are found by their JSON path despite human labels', () => {
  const {anchors} = parseAnchors('<a href="scenario.json">Scenario JSON</a><a href="flight.json">Typed preflight result JSON</a><a href="../next/index.html">Next case</a>', 'http://127.0.0.1:8000/runs/unsupported/index.html');
  assert.deepEqual(anchors.map(isEvidenceAnchor), [true, true, false]);
});

test('HTTP crawl distinguishes report HTML from source JSON and rejects malformed evidence', () => {
  verifyHttpContent('text/html', '<h1>Report</h1>', 'report');
  verifyHttpContent('application/json', '{"id":"case"}', 'source', 'json');
  assert.throws(() => verifyHttpContent('text/html', '{}', 'wrong source', 'json'), /expected JSON/);
  assert.throws(() => verifyHttpContent('application/json', 'not json', 'corrupt source', 'json'), SyntaxError);
});

test('zero-command NoClearing keeps its initial sample and is not a flown landing', () => {
  const stopped = {
    planning_stop: 'no_clearing', physical_outcome: 'flying', mission_outcome: 'in_progress',
    manifest: {physics_steps: 0, controller_updates: 0, sim_time_s: 0},
    ordinary_flight: {actions: [], samples: [{physics_step: 0}]},
  };
  verifyOrdinaryExecution(stopped, 'finite stop');
  assert.throws(() => verifyOrdinaryExecution({...stopped, planning_stop: 'landed'}, 'fake landing'), /not a landing/);
  assert.throws(() => verifyOrdinaryExecution({...stopped, manifest: {...stopped.manifest, physics_steps: 2}}, 'fake steps'), /nonzero execution coverage/);
});

test('typed comparison ignores object key order but preserves exact numbers and array order', () => {
  assertTypedEqual({b: [1, 2], a: 3}, {a: 3, b: [1, 2]}, 'same typed values');
  assert.throws(() => assertTypedEqual({values: [1, 2]}, {values: [2, 1]}, 'array order'), /\$\.values\[0\]/);
  assert.throws(() => assertTypedEqual({value: 1}, {value: 1.0000000001}, 'numeric precision'), /\$\.value/);
});

test('flight comparison removes only the three planner timings and observed root path fields', () => {
  const accepted = {
    timings: {planning_s: 1, execution_s: 2, replay_s: 3, other_s: 4},
    scenario_path: '/accepted/scenario.json',
    state: {ticks: [0, 12], fuel_kg: 8},
  };
  const capture = {
    timings: {planning_s: 10, execution_s: 20, replay_s: 30, other_s: 4},
    scenario_path: '/new/scenario.json',
    state: {ticks: [0, 12], fuel_kg: 8},
  };
  assert.deepEqual(comparableFlight(capture, accepted, 'synthetic flight'), [
    '/timings/planning_s', '/timings/execution_s', '/timings/replay_s', '/scenario_path',
  ]);
  assert.throws(() => comparableFlight({...capture, state: {...capture.state, fuel_kg: 8.000001}}, accepted, 'tampered flight'), /fuel_kg/);
  assert.throws(() => comparableFlight({...capture, timings: {...capture.timings, other_s: 99}}, accepted, 'unlisted timing'), /other_s/);
});

test('reportData extraction and annotation comparison bind every H and entry to exact raw segment snapshots', () => {
  const flight = {
    correction_count: 1,
    segments: [{
      kind: 'local_correction',
      entry_state: {
        physics_step: 120, sim_time_s: 1, position_m: {x: -4, y: 50},
        velocity_mps: {x: 20, y: 3}, attitude_rad: 0.25, fuel_kg: 600,
      },
      end_state: {
        physics_step: 240, sim_time_s: 2, position_m: {x: -2, y: 55},
        velocity_mps: {x: 21, y: 2}, attitude_rad: 0, fuel_kg: 590,
      },
    }],
  };
  const annotation = {
    number: 1,
    entry: {physicsStep: 120, simTimeS: 1, positionM: {x: -4, y: 50}, velocityMps: {x: 20, y: 3}, attitudeRad: 0.25, fuelKg: 600},
    handoff: {physicsStep: 240, simTimeS: 2, positionM: {x: -2, y: 55}, velocityMps: {x: 21, y: 2}, attitudeRad: 0, fuelKg: 590},
    reason: 'The direct route was blocked.',
    afterHandoff: 'A later direct continuation landed.',
  };
  const data = extractReportData(`<!doctype html><script>const reportData = {"samples":[],"flightAnnotations":{"corrections":[${JSON.stringify(annotation)}]}};\n</script>`, 'synthetic report');
  assert.equal(verifyCorrectionAnnotations(flight, data, 'synthetic report'), 1);
  const altered = structuredClone(data);
  altered.flightAnnotations.corrections[0].handoff.positionM.x = -1.999999;
  assert.throws(() => verifyCorrectionAnnotations(flight, altered, 'tampered report'), /exact handoff snapshot/);
});

test('HTML detail links resolve against a declared stable batch base href', () => {
  const html = '<head><base href="/eval/planner_v2_lab_suite/capture-1/"></head><a href="runs/v2_clear_685/index.html">clear</a>';
  const parsed = parseAnchors(html, 'http://127.0.0.1:8000/reports/eval/planner_v2_lab_suite/index.html');
  assert.equal(parsed.baseUrl, 'http://127.0.0.1:8000/eval/planner_v2_lab_suite/capture-1/');
  assert.equal(parsed.anchors[0].url, 'http://127.0.0.1:8000/eval/planner_v2_lab_suite/capture-1/runs/v2_clear_685/index.html');
});
