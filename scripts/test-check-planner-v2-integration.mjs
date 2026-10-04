import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join, relative} from 'node:path';
import {checkCaseOutcome, checkCliMatrix, checkSavedReplays, compactJsonLexical, compareCliRepeat, compareNativePhysical, expectedReplayDigests, landed, latencyStats, rawObjectFields, requestIdentity, validateOutputSeparation, verifyBundle, verifyProgress, verifyReplayReceipt} from './check-planner-v2-integration.mjs';

// These small files test checker logic only. No binary, simulator or retained
// capture is needed; they are deliberately not physical acceptance evidence.
const POLICY = {policy_id: 'piecewise_local_clearing_v2_policy_3', maximum_corrections: 6};
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const write = (p, o) => writeFileSync(p, JSON.stringify(o));
const read = p => JSON.parse(readFileSync(p));
const source = {git_commit: 'frozen-test-source', git_dirty: false, rust_source_tree_sha256: 'native-seal', executable_sha256: 'evaluator-exe'};
function inventory(dir) {
  const values = {};
  function visit(path) {for (const item of readdirSync(path, {withFileTypes: true}).sort((a, b) => a.name.localeCompare(b.name))) {
    const p = join(path, item.name); if (item.isDirectory()) visit(p); else values[relative(dir, p)] = hash(p);
  }}
  visit(dir); return values;
}
const state = step => ({physics_step: step, sim_time_s: step / 120, position_m: {x: 1, y: 2}, velocity_mps: {x: 3, y: -4}, attitude_rad: 0.1, fuel_kg: 500, held_command: {throttle: 0.5}, incoming_contact: null});
const annotation = s => ({physicsStep: s.physics_step, simTimeS: s.sim_time_s, positionM: s.position_m, velocityMps: s.velocity_mps, attitudeRad: s.attitude_rad, fuelKg: s.fuel_kg});
const requestFor = input => ({probe_id: input.case_id, scenario: input.scenario, source_pad_id: 'source', target_pad_id: 'target', policy: {version: 'generation-seal'}});
function flight(input) {
  const unsupported = !!input.expected_preflight;
  const finite = input.case_id.startsWith('finite');
  const corrected = input.group === 'ordinary';
  return {policy: POLICY, input_identity: requestIdentity(JSON.stringify({request: requestFor(input), policy: POLICY})), planning_stop: unsupported ? 'unsupported' : finite ? 'no_clearing' : 'landed',
    reason: unsupported ? 'unsupported test input' : null, correction_count: corrected ? 1 : 0, initial_nominal_terrain_blocked: corrected,
    integrity_passed: true, physical_outcome: unsupported ? null : finite ? 'flying' : 'landed_on_target',
    mission_outcome: unsupported ? null : finite ? 'in_progress' : 'success', final_source_replay_passed: !unsupported,
    absolute_deadline_physics_step: unsupported ? null : 10800,
    manifest: unsupported ? null : {physics_steps: finite ? 0 : 2, controller_updates: finite ? 0 : 1, sim_time_s: finite ? 0 : 2 / 120},
    ordinary_flight: unsupported ? null : {actions: finite ? [] : [{command: {throttle: 0.5}, physics_step: 0}], events: [], samples: [{physics_step: 0}], final_state: state(finite ? 0 : 2), incoming_contact: null},
    segments: corrected ? [{kind: 'local_correction', entry_state: state(0), end_state: state(2), updates: [{command: {throttle: 0.5}}]}] : [],
    cycles: unsupported ? [] : [{selected_program: 'one', current_state: state(0)}], timings: {planning_s: 1, execution_s: 2, replay_s: 3}};
}
function progress(f) {
  const hasLive = f.ordinary_flight !== null;
  const entries = f.segments.map((s, i) => ({elapsed_s: 1, progress: {status: 'handoff', piece_index: i, correction_count: i + 1, entry_physics_step: 0, handoff_physics_step: s.end_state.physics_step}}));
  entries.push({elapsed_s: 1, progress: {status: 'terminal', planning_stop: f.planning_stop, correction_count: f.correction_count,
    piece_index: hasLive ? f.correction_count : null, entry_physics_step: hasLive ? (f.correction_count ? 2 : 0) : null,
    physics_step: hasLive ? f.ordinary_flight.final_state.physics_step : null}});
  return {schema_id: 'planner_v2_cli_progress_v1', entries, finalization_elapsed_s: 0.1};
}
function synthetic(t, suffix = '') {
  const root = mkdtempSync(join(tmpdir(), 'pd-v2-integration-check-'));
  t.after(() => rmSync(root, {recursive: true}));
  const native = join(root, 'native'), cli = join(root, 'cli' + suffix);
  mkdirSync(native); mkdirSync(cli); mkdirSync(join(native, 'runs')); mkdirSync(join(cli, 'runs')); mkdirSync(join(cli, 'logs'));
  const inputs = [...Array(11)].map((_, i) => ({case_id: `clear${i}`, group: 'clear'}))
    .concat([...Array(25)].map((_, i) => ({case_id: `terrain${i}`, group: 'ordinary'})))
    .concat(['land0', 'land1', 'finite0', 'finite1', 'finite2', 'finite3', 'unsupported0', 'unsupported1'].map(case_id => ({case_id, group: 'diagnostic'})))
    .map(x => ({...x, family: x.group, expected_preflight: x.case_id.startsWith('unsupported') ? 'unsupported' : null,
      source_pad_id: 'source', target_pad_id: 'target', scenario: {id: x.case_id, initial_state: {position_m: {x: 0, y: 5}}, world: {landing_pads: [{id: 'source'}, {id: 'target'}]}}}));
  const rows = [], cases = [];
  for (const input of inputs) {
    const f = flight(input), paths = {case_id: input.case_id, scenario_path: `runs/${input.case_id}/scenario.json`, flight_path: `runs/${input.case_id}/flight.json`, summary_path: `runs/${input.case_id}/summary.json`, rich_report_path: `runs/${input.case_id}/report.html`};
    for (const base of [native, cli]) {
      const dir = join(base, 'runs', input.case_id); mkdirSync(dir);
      write(join(dir, 'scenario.json'), input.scenario); write(join(dir, 'flight.json'), f);
      write(join(dir, 'summary.json'), {input_identity: f.input_identity, policy: POLICY, result: {planning_stop: f.planning_stop, timings: f.timings}, timings: {output_s: 0.01, total_s: 6.01}});
      if (f.ordinary_flight) {
        const data = {allTelemetry: [{speed: 5}], flightAnnotations: {corrections: f.segments.map((s, i) => ({number: i + 1, entry: annotation(s.entry_state), handoff: annotation(s.end_state), reason: 'blocked', afterHandoff: 'replan'}))}};
        writeFileSync(join(dir, 'report.html'), `const reportData = ${JSON.stringify(data)};\n`);
      }
    }
    const dir = join(cli, 'runs', input.case_id);
    write(join(dir, 'progress.json'), progress(f));
    const names = ['scenario.json', 'flight.json', 'summary.json', 'progress.json']; if (f.ordinary_flight) names.push('report.html');
    const request = requestFor(input);
    write(join(dir, 'bundle.json'), {schema_id: 'planner_v2_cli_bundle_v1', request, policy: POLICY, input_identity: f.input_identity, artifact_sha256: Object.fromEntries(names.map(name => [name, hash(join(dir, name))]))});
    const stdout = {...Object.fromEntries(['planning_stop', 'reason', 'physical_outcome', 'mission_outcome', 'integrity_passed', 'final_source_replay_passed', 'correction_count', 'input_identity'].map(key => [key, f[key]])), output_dir: dir,
      schema_id: 'planner_v2_cli_flight_v1', status: 'completed', supported: !input.expected_preflight, policy: POLICY};
    write(join(cli, 'logs', `${input.case_id}.stdout.json`), stdout); writeFileSync(join(cli, 'logs', `${input.case_id}.stderr.txt`), '');
    rows.push({...paths, status: f.ordinary_flight ? 'simulated' : 'preflight_rejected', planning_s: 1, execution_s: 2, replay_s: 3, artifact_sha256: {}});
    cases.push({case_id: input.case_id, exit_code: landed(f) ? 0 : 1, stdout, paths});
  }
  const batch = {schema_id: 'planner_v2_eval_batch_v1', pack_id: 'planner_v2_lab_suite', status: 'completed', policy_version: 3, case_count: 44, cases: rows, summary: {physical_test_fixture: true}, input_identity: {test: 'same-inputs'}, provenance: {source_before: source, source_after: source, unchanged_during_capture: true}};
  write(join(native, 'summary.json'), batch); write(join(native, 'pack.json'), {id: 'planner_v2_lab_suite'}); write(join(native, 'expanded-inputs.json'), inputs); write(join(cli, 'expanded-inputs.json'), inputs);
  const cliSource = {...source, evaluator_sha256: source.executable_sha256, cli_sha256: 'distinct-cli-exe', integration_source_sha256: 'cli-inclusive-seal'}; delete cliSource.executable_sha256;
  const matrix = {schema_id: 'planner_v2_cli_validation_matrix_v1', pack_id: 'planner_v2_lab_suite', status: 'completed', passed: true, attempted_case_count: 44, cases,
    source_before: cliSource, source_after: cliSource, native_summary_sha256: hash(join(native, 'summary.json')), input_identity: batch.input_identity,
    totals: {mandatory_landings: 36, direct_clear_landings: 11, corrected_terrain_landings: 25, diagnostic_landings: 2, diagnostic_finite_stops: 4, unsupported: 2, integrity: 44, supported_source_replay: 42}, artifact_sha256: inventory(cli)};
  write(join(cli, 'matrix.json'), matrix);
  return {root, native, cli, inputs, batch, matrix};
}

test('CLI parity retains distinct executable provenance and all 44 bound cases', t => {
  const s = synthetic(t); const result = checkCliMatrix(s.native, s.cli);
  assert.equal(result.passed, true); assert.equal(result.compared_rich_payloads, 42);
  assert.notEqual(result.native_provenance.source_before.executable_sha256, result.cli_source_before.cli_sha256);
  const p = join(s.cli, 'matrix.json'), m = read(p); m.source_after.cli_sha256 = 'swapped'; write(p, m);
  assert.throws(() => checkCliMatrix(s.native, s.cli), /source|swapped/);
});

test('matrix rejects missing rows, mixed source/input seals, altered exit semantics and unbound files', t => {
  const s = synthetic(t), p = join(s.cli, 'matrix.json');
  for (const mutate of [
    m => m.cases.pop(), m => {m.cases[0].case_id = 'foreign';}, m => {m.cases[0].exit_code = 1;},
    m => {m.source_before.git_commit = 'other'; m.source_after.git_commit = 'other';},
    m => {m.input_identity.test = 'different';}, m => {delete m.artifact_sha256['logs/clear0.stderr.txt'];},
  ]) {
    const m = structuredClone(s.matrix); mutate(m); write(p, m);
    assert.throws(() => checkCliMatrix(s.native, s.cli));
  }
});

test('bundle carries explicit pads and complete scenario and rejects stale or foreign artifacts', t => {
  const s = synthetic(t), input = s.inputs[0], dir = join(s.cli, 'runs', input.case_id), p = join(dir, 'bundle.json'), original = read(p);
  verifyBundle(dir, input, flight(input));
  for (const change of [b => {b.request.source_pad_id = 'target';}, b => {b.request.scenario.initial_state.position_m.x = 10;},
    b => {b.request.policy.version = 'unbound-generation-policy';},
    b => {b.input_identity = 'unbound';}, b => {b.artifact_sha256['flight.json'] = 'fake';}, b => {b.artifact_sha256['../foreign'] = 'fake';}]) {
    const b = structuredClone(original); change(b); write(p, b); assert.throws(() => verifyBundle(dir, input, flight(input)));
  }
});

test('typed identity preserves Rust numeric lexemes, strings and full generation policy', () => {
  const raw = '{ "request": {"n":1.0,"text":"spaces \\" and ] stay", "v":[1, {"a":2.0}]}, "policy": {"p":1} }';
  assert.equal(rawObjectFields(raw).get('request'), '{"n":1.0,"text":"spaces \\" and ] stay","v":[1,{"a":2.0}]}');
  assert.equal(requestIdentity(raw), requestIdentity(compactJsonLexical(raw)));
  assert.notEqual(requestIdentity(raw), requestIdentity(raw.replace('1.0', '1')));
  assert.notEqual(requestIdentity(raw), requestIdentity(raw.replace('2.0', '3.0')));
  assert.throws(() => rawObjectFields('{"request":{},"request":{},"policy":{}}'), /duplicate/);
});

test('replay receipt binds actual commands, events, samples, full state, contact and original clock', () => {
  const f = flight({case_id: 'land0', group: 'diagnostic', expected_preflight: null}), raw = JSON.stringify(f);
  const receipt = {schema_id: 'planner_v2_cli_replay_v1', replay_passed: true, input_identity: f.input_identity, planning_stop: f.planning_stop,
    physical_outcome: f.physical_outcome, mission_outcome: f.mission_outcome,
    comparison_sha256: expectedReplayDigests(raw), physics_step: f.ordinary_flight.final_state.physics_step, sim_time_s: f.ordinary_flight.final_state.sim_time_s};
  verifyReplayReceipt(receipt, raw, 'synthetic');
  for (const field of Object.keys(receipt.comparison_sha256)) {
    const broken = structuredClone(receipt); broken.comparison_sha256[field] = 'wrong';
    assert.throws(() => verifyReplayReceipt(broken, raw, field));
  }
  assert.throws(() => verifyReplayReceipt({...receipt, physics_step: 0}, raw, 'rewound'));
  assert.throws(() => verifyReplayReceipt({...receipt, planning_stop: 'no_clearing'}, raw, 'wrong stop'));
  assert.throws(() => verifyReplayReceipt({...receipt, comparison_sha256: undefined}, raw, 'boolean-only'));
  const unsupported = flight({case_id: 'unsupported0', group: 'diagnostic', expected_preflight: 'unsupported'});
  assert.throws(() => expectedReplayDigests(JSON.stringify(unsupported)), /no physical/);
});

test('saved replay inventory has exactly 42 supported bound actual-component receipts', t => {
  const s = synthetic(t), out = join(s.root, 'replays'); mkdirSync(out);
  const cases = [];
  for (const input of s.inputs.filter(x => !x.expected_preflight)) {
    const raw = readFileSync(join(s.cli, 'runs', input.case_id, 'flight.json'), 'utf8'), f = JSON.parse(raw);
    const outcome = {schema_id: 'planner_v2_cli_replay_v1', replay_passed: true, input_identity: f.input_identity,
      planning_stop: f.planning_stop, physical_outcome: f.physical_outcome, mission_outcome: f.mission_outcome,
      physics_step: f.ordinary_flight.final_state.physics_step, sim_time_s: f.ordinary_flight.final_state.sim_time_s,
      comparison_sha256: expectedReplayDigests(raw)};
    cases.push({case_id: input.case_id, exit_code: 0, outcome});
    write(join(out, `${input.case_id}.stdout.json`), outcome); writeFileSync(join(out, `${input.case_id}.stderr.txt`), '');
  }
  const receipt = {schema_id: 'planner_v2_cli_saved_replay_matrix_v1', status: 'completed', passed: true, attempted_replays: 42, cases,
    source_before: s.matrix.source_before, source_after: s.matrix.source_after, cli_sha256: s.matrix.source_before.cli_sha256,
    matrix_sha256: hash(join(s.cli, 'matrix.json')), artifact_sha256: inventory(out)};
  const p = join(out, 'replays.json'); write(p, receipt);
  assert.equal(checkSavedReplays(s.native, s.cli, out).actual_component_digests_verified, 252);
  for (const mutate of [r => r.cases.pop(), r => {r.cases[0].exit_code = 1;},
    r => {r.cases[0].outcome.comparison_sha256.events = 'wrong';}, r => {r.source_after.cli_sha256 = 'swapped';},
    r => {r.matrix_sha256 = 'unbound';}, r => {delete r.artifact_sha256['land0.stderr.txt'];}]) {
    const r = structuredClone(receipt); mutate(r); write(p, r); assert.throws(() => checkSavedReplays(s.native, s.cli, out));
  }
});

test('single-matrix stdout must bind schema, supported flag, policy, reason and actual output root', t => {
  const s = synthetic(t), matrixPath = join(s.cli, 'matrix.json'), stdoutPath = join(s.cli, 'logs', 'clear0.stdout.json');
  const original = read(stdoutPath);
  for (const mutate of [o => {o.schema_id = 'other';}, o => {o.status = 'preflight_only';},
    o => {o.supported = false;}, o => {o.policy.maximum_corrections = 8;}, o => {o.reason = 'different';},
    o => {o.output_dir = s.native;}]) {
    const o = structuredClone(original); mutate(o); write(stdoutPath, o);
    const m = structuredClone(s.matrix); m.cases[0].stdout = o; m.artifact_sha256['logs/clear0.stdout.json'] = hash(stdoutPath); write(matrixPath, m);
    assert.throws(() => checkCliMatrix(s.native, s.cli));
  }
});

test('capture and replay outputs reject existing, protected, tracked and symlinked overlap before calls', t => {
  const dir = mkdtempSync(join(tmpdir(), 'pd-v2-integration-paths-')); t.after(() => rmSync(dir, {recursive: true}));
  const root = join(dir, 'repo'), protectedPath = join(dir, 'capture');
  mkdirSync(root); mkdirSync(protectedPath); mkdirSync(join(root, 'outputs')); mkdirSync(join(root, 'outputs', 'validation'));
  assert.equal(validateOutputSeparation(root, join(root, 'outputs', 'validation', 'new'), [protectedPath]), join(root, 'outputs', 'validation', 'new'));
  assert.throws(() => validateOutputSeparation(root, join(protectedPath, 'new'), [protectedPath]), /overlap/);
  assert.throws(() => validateOutputSeparation(root, join(root, 'tracked-new'), [protectedPath]), /outputs\/validation/);
  assert.throws(() => validateOutputSeparation(root, protectedPath, [protectedPath]), /exists/);
  const linked = join(dir, 'linked'); symlinkSync(protectedPath, linked);
  assert.throws(() => validateOutputSeparation(root, join(linked, 'new'), [protectedPath]), /overlap/);
  const dangling = join(dir, 'dangling'); symlinkSync(join(dir, 'missing'), dangling);
  assert.throws(() => validateOutputSeparation(root, dangling, [protectedPath]), /exists/);
});

test('full flight parity catches commands, state, contact, clock, fuel, segments and selected programs', t => {
  const s = synthetic(t), input = s.inputs[0], p = join(s.cli, 'runs', input.case_id, 'flight.json');
  for (const mutate of [
    f => {f.ordinary_flight.actions[0].command.throttle = 0.4;}, f => {f.ordinary_flight.final_state.fuel_kg--;},
    f => {f.ordinary_flight.final_state.physics_step++;}, f => {f.ordinary_flight.incoming_contact = {normal: 1};},
    f => {f.ordinary_flight.samples.push({physics_step: 2});}, f => {f.cycles[0].selected_program = 'different';},
    f => {f.segments.push({kind: 'extra'});}, f => {f.ordinary_flight.final_state.held_command.throttle = 0.2;},
  ]) {
    const f = flight(input); mutate(f); write(p, f);
    // Even freshly re-sealing the artifact cannot bypass native physical parity.
    const bundlePath = join(s.cli, 'runs', input.case_id, 'bundle.json'), b = read(bundlePath); b.artifact_sha256['flight.json'] = hash(p); write(bundlePath, b);
    assert.throws(() => checkCliMatrix(s.native, s.cli));
  }
});

test('honest finite stops and unsupported inputs stay distinct from successful flight exits', () => {
  const finite = {case_id: 'finite0', group: 'diagnostic', expected_preflight: null};
  checkCaseOutcome(finite, flight(finite), 1);
  assert.throws(() => checkCaseOutcome(finite, flight(finite), 0));
  const unsupported = {case_id: 'unsupported0', group: 'diagnostic', expected_preflight: 'unsupported'};
  checkCaseOutcome(unsupported, flight(unsupported), 1);
  assert.throws(() => checkCaseOutcome(unsupported, {...flight(unsupported), ordinary_flight: {}}, 1));
  assert.throws(() => checkCaseOutcome(finite, {...flight(finite), planning_stop: 'implementation_error'}, 1));
});

test('piece progress binds original entry, actual H clock, terminal endpoint and finite timing', () => {
  const f = flight({case_id: 'terrain0', group: 'ordinary', expected_preflight: null});
  verifyProgress(progress(f), f, 'synthetic');
  for (const mutate of [p => {p.entries[0].progress.handoff_physics_step = 4;},
    p => {p.entries[1].progress.entry_physics_step = 0;}, p => {p.entries[1].progress.physics_step++;},
    p => {p.entries.push(p.entries[1]);}, p => {p.entries[0].elapsed_s = -1;},
    p => {p.entries[1].progress.planning_stop = 'no_clearing';}, p => {p.finalization_elapsed_s = Infinity;}]) {
    const trace = progress(f); mutate(trace); assert.throws(() => verifyProgress(trace, f, 'tampered'));
  }
  const unsupported = flight({case_id: 'unsupported0', group: 'diagnostic', expected_preflight: 'unsupported'});
  verifyProgress(progress(unsupported), unsupported, 'unsupported');
  const zero = flight({case_id: 'finite0', group: 'diagnostic', expected_preflight: null});
  verifyProgress(progress(zero), zero, 'zero-command finite stop');
  const deadline = {...zero, planning_stop: 'deadline'}, deadlineTrace = progress(deadline);
  deadlineTrace.entries[0].progress.piece_index = null;
  verifyProgress(deadlineTrace, deadline, 'pre-piece deadline');
  assert.deepEqual(latencyStats([3, 1, 2]), {count: 3, median_s: 2, p95_s: 3, maximum_s: 3});
  assert.equal(latencyStats([4, 1, 3, 2]).median_s, 2.5);
  assert.throws(() => latencyStats([NaN]));
});

test('cross-version native parity retains both source identities and rejects non-timing change', t => {
  const a = synthetic(t), b = synthetic(t);
  const p = join(b.native, 'summary.json'), batch = read(p);
  batch.provenance.source_before.git_commit = 'new-version'; batch.provenance.source_after.git_commit = 'new-version'; write(p, batch);
  const result = compareNativePhysical(a.native, b.native);
  assert.notEqual(result.baseline_provenance.source_before.git_commit, result.capture_provenance.source_before.git_commit);
  const fpath = join(b.native, 'runs', 'clear0', 'flight.json'), f = read(fpath); f.timings.planning_s = 9; write(fpath, f);
  assert.equal(compareNativePhysical(a.native, b.native).passed, true);
  f.ordinary_flight.final_state.fuel_kg++; write(fpath, f);
  assert.throws(() => compareNativePhysical(a.native, b.native));
});

test('CLI repeat requires the same executable and binds its only output-root exclusion', t => {
  const a = synthetic(t), b = synthetic(t);
  checkCliMatrix(a.native, a.cli); checkCliMatrix(b.native, b.cli);
  assert.equal(compareCliRepeat(a.cli, b.cli).passed, true);
  const p = join(b.cli, 'matrix.json'), m = read(p); m.source_before.cli_sha256 = 'different-exe'; m.source_after.cli_sha256 = 'different-exe'; write(p, m);
  assert.throws(() => compareCliRepeat(a.cli, b.cli), /source\/both binaries/);
});
