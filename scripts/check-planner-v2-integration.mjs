#!/usr/bin/env node
// Real optional-CLI validation. This is a lab validation matrix, not a native
// BatchReport or a publication receipt. Read-only checks never plan a flight.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawn, spawnSync} from 'node:child_process';
import {lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync} from 'node:fs';
import {basename, dirname, isAbsolute, join, relative, resolve, sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
import {comparableFlight, extractReportData, verifyCorrectionAnnotations} from './check-planner-v2-batch.mjs';
import {comparableCompactSummary, nativeAcceptance, safeFile, verifySavedExecution} from './check-planner-v2-workflow.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PACK = 'planner_v2_lab_suite';
const SCHEMA = 'planner_v2_cli_validation_matrix_v1';
const sha = value => createHash('sha256').update(value).digest('hex');
const fileSha = path => sha(readFileSync(path));
const json = path => JSON.parse(readFileSync(path, 'utf8'));
const readJson = (root, path) => json(safeFile(root, path));
const write = (path, value) => writeFileSync(path, JSON.stringify(value, null, 2) + '\n', {flag: 'wx'});
const finiteStops = new Set(['no_nominal', 'nominal_rejected', 'no_clearing', 'correction_limit', 'deadline', 'no_progress']);
const POLICY = {policy_id: 'piecewise_local_clearing_v2_policy_3', maximum_corrections: 6};

// Preserve Rust's serialized float spelling (including 1.0), key order and
// escaping. JSON.parse/stringify alone cannot reproduce serde identity bytes.
export function compactJsonLexical(raw) {
  JSON.parse(raw);
  let quoted = false, escaped = false, start = 0;
  const chunks = [];
  for (let i = 0; i < raw.length; i++) {
    const char = raw[i];
    if (quoted) {
      if (escaped) escaped = false;
      else if (char === '\\') escaped = true;
      else if (char === '"') quoted = false;
    } else if (char === '"') quoted = true;
    else if (' \r\n\t'.includes(char)) {
      if (start < i) chunks.push(raw.slice(start, i));
      start = i + 1;
    }
  }
  if (start < raw.length) chunks.push(raw.slice(start));
  return chunks.join('');
}

export function rawObjectFields(raw) {
  const text = compactJsonLexical(raw);
  assert(text.startsWith('{') && text.endsWith('}'), 'expected JSON object');
  const fields = new Map(); let i = 1;
  function stringEnd(start) {
    assert.equal(text[start], '"'); let escaped = false;
    for (let j = start + 1; j < text.length; j++) {
      if (escaped) escaped = false;
      else if (text[j] === '\\') escaped = true;
      else if (text[j] === '"') return j + 1;
    }
    throw new Error('unterminated JSON string');
  }
  while (i < text.length - 1) {
    const end = stringEnd(i), key = JSON.parse(text.slice(i, end));
    assert(!fields.has(key), `duplicate JSON field: ${key}`);
    assert.equal(text[end], ':'); i = end + 1;
    const start = i; let depth = 0;
    for (; i < text.length; i++) {
      const char = text[i];
      if (char === '"') {i = stringEnd(i) - 1; continue;}
      if (char === '[' || char === '{') depth++;
      else if (char === ']' || char === '}') {if (!depth) break; depth--;}
      else if (char === ',' && !depth) break;
    }
    fields.set(key, text.slice(start, i));
    if (text[i] === ',') i++;
    else {assert.equal(text[i], '}'); break;}
  }
  return fields;
}

export function requestIdentity(receiptRaw) {
  const fields = rawObjectFields(receiptRaw);
  assert(fields.has('request') && fields.has('policy'), 'missing typed request/policy');
  const bytes = Buffer.from(`[${fields.get('request')},${fields.get('policy')}]`, 'utf8');
  let value = 0xcbf29ce484222325n;
  for (const byte of bytes) value = ((value ^ BigInt(byte)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return 'fnv1a64:' + value.toString(16).padStart(16, '0');
}

function within(parent, child) {
  const path = relative(parent, child);
  return path === '' || (!isAbsolute(path) && path !== '..' && !path.startsWith('..' + sep));
}

export function validateOutputSeparation(root, output, protectedRoots) {
  const requested = resolve(output);
  try {lstatSync(requested); throw new Error('output already exists (including symlinks)');}
  catch (error) {if (error.code !== 'ENOENT') throw error;}
  const canonical = join(realpathSync(dirname(requested)), basename(requested));
  const repository = realpathSync(root);
  if (within(repository, canonical)) assert(within(join(repository, 'outputs/validation'), canonical), 'matrix output inside repository must be under outputs/validation');
  for (const path of protectedRoots) {
    const protectedPath = realpathSync(path);
    assert(!within(protectedPath, canonical) && !within(canonical, protectedPath), 'output overlaps a protected evidence root');
  }
  return canonical;
}

function git(root, args) {
  const r = spawnSync('git', ['-C', root, ...args], {encoding: 'utf8', maxBuffer: 16 * 1024 * 1024});
  assert.equal(r.status, 0, r.stderr);
  return r.stdout;
}

function appendSource(hash, root, path) {
  hash.update(path).update(Buffer.from([0])).update(readFileSync(safeFile(root, path))).update(Buffer.from([255]));
}

// Deliberately matches the evaluator's existing source seal. The separate
// integration seal also covers CLI, tests and maintained JavaScript sources.
export function nativeSourceSha(root) {
  const hash = createHash('sha256');
  function visit(path) {
    for (const name of readdirSync(join(root, path)).sort()) {
      const file = join(path, name), stat = lstatSync(join(root, file));
      assert(!stat.isSymbolicLink(), `source symlink: ${file}`);
      if (stat.isDirectory()) visit(file);
      else if (stat.isFile() && name.endsWith('.rs')) appendSource(hash, root, file);
    }
  }
  for (const crate of ['pd-eval', 'pd-core', 'pd-plan', 'pd-report', 'pd-control']) visit(`${crate}/src`);
  for (const file of ['Cargo.toml', 'Cargo.lock', 'pd-eval/Cargo.toml', 'pd-core/Cargo.toml', 'pd-plan/Cargo.toml', 'pd-report/Cargo.toml', 'pd-control/Cargo.toml']) appendSource(hash, root, file);
  return hash.digest('hex');
}

export function sourceState(root, evaluator, cli) {
  const hash = createHash('sha256');
  const paths = git(root, ['ls-files', '-z']).split('\0').filter(p => p && (p.endsWith('.rs') || p.endsWith('.mjs') || p.endsWith('Cargo.toml') || p === 'Cargo.lock')).sort();
  assert(paths.some(p => p.startsWith('pd-cli/src/')), 'CLI source missing from integration seal');
  for (const path of paths) appendSource(hash, root, path);
  return {
    git_commit: git(root, ['rev-parse', 'HEAD']).trim(),
    git_dirty: git(root, ['status', '--porcelain']).trim().length > 0,
    rust_source_tree_sha256: nativeSourceSha(root),
    integration_source_sha256: hash.digest('hex'),
    evaluator_sha256: fileSha(evaluator), cli_sha256: fileSha(cli),
  };
}

export function checkInputSeals(root, batch) {
  const identity = batch.input_identity;
  const packPath = safeFile(root, `fixtures/packs/${PACK}.json`);
  assert.equal(fileSha(packPath), identity.pack_file_sha256, 'tracked pack drift');
  const pack = json(packPath);
  assert.equal(pack.id, PACK); assert.equal(pack.policy_version, 3);
  assert.equal(identity.rust_typed_expanded_input_count, 44);
  for (const source of pack.sources) {
    assert.equal(fileSha(safeFile(root, source.path)), source.sha256, `tracked source ${source.source_id}`);
    assert.equal(identity.source_fixture_sha256[source.source_id], source.sha256);
  }
  for (const [path, digest] of Object.entries(identity.source_manifest_sha256)) assert.equal(fileSha(safeFile(root, path)), digest, `tracked input manifest ${path}`);
  return identity;
}

function nativeBatch(capture) {
  const batch = readJson(capture, 'summary.json');
  assert.equal(batch.schema_id, 'planner_v2_eval_batch_v1');
  assert.equal(batch.pack_id, PACK); assert.equal(batch.status, 'completed');
  assert.equal(batch.policy_version, 3); assert.equal(batch.case_count, 44);
  assert.equal(batch.cases.length, 44);
  assert.equal(new Set(batch.cases.map(c => c.case_id)).size, 44);
  assert.equal(batch.provenance.unchanged_during_capture, true);
  assert.deepEqual(batch.provenance.source_before, batch.provenance.source_after);
  assert.equal(batch.provenance.source_before.git_dirty, false);
  return batch;
}

export function flightPair(leftRoot, left, rightRoot, right) {
  const label = left.case_id;
  assert.deepEqual(readJson(leftRoot, left.scenario_path), readJson(rightRoot, right.scenario_path), `${label}: scenario`);
  const a = readJson(leftRoot, left.flight_path), b = readJson(rightRoot, right.flight_path);
  // Output locations have not occurred in this DTO. If they ever appear, do
  // not silently accept the historical checker's generic path exclusions.
  for (const f of [a, b]) assert(!Object.hasOwn(f, 'scenario_path') && !Object.hasOwn(f, 'output_dir'), `${label}: undeclared output-root field`);
  const flightExclusions = comparableFlight(a, b, label);
  const compactExclusions = comparableCompactSummary(readJson(leftRoot, left.summary_path), readJson(rightRoot, right.summary_path), label);
  let rich = false;
  if (a.ordinary_flight) {
    const x = extractReportData(readFileSync(safeFile(leftRoot, left.rich_report_path), 'utf8'), label);
    const y = extractReportData(readFileSync(safeFile(rightRoot, right.rich_report_path), 'utf8'), label);
    verifyCorrectionAnnotations(a, x, label); verifyCorrectionAnnotations(b, y, label);
    assert.deepEqual(x, y, `${label}: complete rich report payload`);
    rich = true;
  } else assert.equal(b.ordinary_flight, null);
  return {flightExclusions, compactExclusions, rich};
}

export function compareNativePhysical(baseline, capture) {
  const a = nativeBatch(baseline), b = nativeBatch(capture);
  assert.notEqual(realpathSync(baseline), realpathSync(capture), 'baseline and capture must differ');
  assert.deepEqual(a.input_identity, b.input_identity, 'baseline input identity');
  assert.deepEqual(a.summary, b.summary, 'baseline outcome summary');
  for (const name of ['pack.json', 'expanded-inputs.json']) assert.deepEqual(readJson(baseline, name), readJson(capture, name), `baseline ${name}`);
  const flights = new Set(), compact = new Set(); let rich = 0;
  for (const [i, left] of a.cases.entries()) {
    const right = b.cases[i];
    const scrub = row => Object.fromEntries(Object.entries(row).filter(([key]) => !['planning_s', 'execution_s', 'replay_s', 'artifact_sha256'].includes(key)));
    assert.deepEqual(scrub(left), scrub(right), `${left.case_id}: full native row`);
    const pair = flightPair(baseline, left, capture, right);
    pair.flightExclusions.forEach(p => flights.add(p)); pair.compactExclusions.forEach(p => compact.add(p)); rich += Number(pair.rich);
  }
  return {schema_id: 'planner_v2_cross_version_physical_parity_v1', passed: true, cases: 44, compared_case_json_artifacts: 132, compared_rich_payloads: rich,
    baseline_provenance: a.provenance, capture_provenance: b.provenance,
    flight_timing_exclusions: [...flights].sort(), compact_timing_exclusions: [...compact].sort(),
    native_row_exclusions: ['planning_s', 'execution_s', 'replay_s', 'artifact_sha256']};
}

export function landed(flight) {
  return flight.planning_stop === 'landed' && flight.physical_outcome === 'landed_on_target' && flight.mission_outcome === 'success' && flight.integrity_passed === true && flight.final_source_replay_passed === true;
}

export function checkCaseOutcome(input, flight, exitCode) {
  assert.equal(flight.integrity_passed, true, `${input.case_id}: integrity`);
  assert.deepEqual(flight.policy, POLICY, `${input.case_id}: explicit policy 3`);
  assert.equal(exitCode, landed(flight) ? 0 : 1, `${input.case_id}: honest single-flight exit`);
  if (input.expected_preflight === 'unsupported') {
    assert.equal(flight.planning_stop, 'unsupported');
    for (const field of ['ordinary_flight', 'manifest', 'physical_outcome', 'mission_outcome', 'absolute_deadline_physics_step']) assert.equal(flight[field], null, `${input.case_id}: unsupported ${field}`);
    assert.equal(flight.final_source_replay_passed, false); assert.equal(flight.correction_count, 0);
    assert.deepEqual(flight.cycles, []); assert.deepEqual(flight.segments, []);
  } else {
    assert.equal(input.expected_preflight, null);
    assert.equal(flight.final_source_replay_passed, true, `${input.case_id}: source replay`);
    verifySavedExecution(flight, input.case_id);
    if (input.group !== 'diagnostic') assert(landed(flight), `${input.case_id}: mandatory landing`);
    else if (!landed(flight)) {
      assert(finiteStops.has(flight.planning_stop), `${input.case_id}: not an honest finite stop`);
      assert.equal(flight.physical_outcome, 'flying'); assert.equal(flight.mission_outcome, 'in_progress');
    }
    if (input.group === 'clear') {
      assert.equal(flight.correction_count, 0); assert.equal(flight.initial_nominal_terrain_blocked, false);
    } else if (input.group !== 'diagnostic') {
      assert(flight.correction_count > 0); assert.equal(flight.initial_nominal_terrain_blocked, true);
    }
  }
}

export function verifyBundle(root, input, nativeFlight) {
  const raw = readFileSync(safeFile(root, 'bundle.json'), 'utf8');
  const receipt = JSON.parse(raw);
  assert.equal(receipt.schema_id, 'planner_v2_cli_bundle_v1');
  assert.deepEqual(receipt.policy, POLICY);
  assert.equal(receipt.request.probe_id, input.scenario.id);
  assert.equal(receipt.request.source_pad_id, input.source_pad_id);
  assert.equal(receipt.request.target_pad_id, input.target_pad_id);
  assert.deepEqual(receipt.request.scenario, input.scenario);
  assert(receipt.request.policy && typeof receipt.request.policy === 'object', 'full generation policy missing');
  assert.equal(receipt.input_identity, nativeFlight.input_identity);
  assert.equal(requestIdentity(raw), receipt.input_identity, 'complete typed request and generation policy identity');
  const flight = readJson(root, 'flight.json');
  assert.equal(flight.input_identity, receipt.input_identity);
  const names = ['scenario.json', 'flight.json', 'summary.json', 'progress.json'];
  if (flight.ordinary_flight) names.push('report.html');
  assert.deepEqual(Object.keys(receipt.artifact_sha256).sort(), names.sort(), 'bundle artifact set');
  for (const [name, digest] of Object.entries(receipt.artifact_sha256)) assert.equal(fileSha(safeFile(root, name)), digest, `bundle binding ${name}`);
  assert.deepEqual(readJson(root, 'scenario.json'), input.scenario);
  return {receipt, flight};
}

export function verifyProgress(trace, flight, label) {
  assert.equal(trace.schema_id, 'planner_v2_cli_progress_v1');
  assert(Array.isArray(trace.entries));
  assert.equal(trace.entries.length, flight.correction_count + 1, `${label}: exactly one advance per handoff plus terminal`);
  assert(Number.isFinite(trace.finalization_elapsed_s) && trace.finalization_elapsed_s >= 0, `${label}: finalization timing`);
  const corrections = flight.segments.filter(s => s.kind === 'local_correction');
  const hasLive = flight.ordinary_flight !== null;
  let previous = hasLive ? 0 : null;
  for (const [i, item] of trace.entries.entries()) {
    assert(Number.isFinite(item.elapsed_s) && item.elapsed_s >= 0, `${label}: piece timing`);
    const p = item.progress;
    assert.equal(p.entry_physics_step, previous, `${label}: piece starts at original source or actual previous H`);
    if (i < corrections.length) {
      assert.equal(p.status, 'handoff'); assert.equal(p.piece_index, i);
      assert.equal(p.correction_count, i + 1);
      assert.equal(p.entry_physics_step, flight.cycles[i].current_state.physics_step);
      assert.equal(p.handoff_physics_step, corrections[i].end_physics_step ?? corrections[i].end_state.physics_step);
      assert(p.handoff_physics_step > previous && p.handoff_physics_step % 2 === 0);
      previous = p.handoff_physics_step;
    } else {
      assert.equal(p.status, 'terminal'); assert.equal(p.planning_stop, flight.planning_stop);
      assert.equal(p.correction_count, flight.correction_count);
      if (hasLive && p.piece_index === null) {
        assert.equal(p.planning_stop, 'deadline', `${label}: only a pre-piece deadline omits its cycle`);
        assert.equal(p.physics_step, previous, `${label}: pre-piece deadline issues no commands`);
      } else assert.equal(p.piece_index, hasLive ? flight.correction_count : null);
      assert.equal(p.physics_step, hasLive ? flight.ordinary_flight.final_state.physics_step : null);
      if (hasLive) assert(p.physics_step >= previous);
    }
  }
  return trace;
}

export function latencyStats(values) {
  assert(values.every(x => Number.isFinite(x) && x >= 0), 'invalid latency sample');
  const sorted = [...values].sort((a, b) => a - b);
  if (!sorted.length) return {count: 0, median_s: null, p95_s: null, maximum_s: null};
  const middle = Math.floor(sorted.length / 2);
  const median = sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
  return {count: sorted.length, median_s: median,
    p95_s: sorted[Math.ceil(sorted.length * 0.95) - 1], maximum_s: sorted.at(-1)};
}

function inventory(root) {
  const out = {};
  function visit(dir) {
    for (const name of readdirSync(dir).sort()) {
      const path = join(dir, name), stat = lstatSync(path);
      assert(!stat.isSymbolicLink(), 'symlink in CLI matrix');
      if (stat.isDirectory()) visit(path);
      else {assert(stat.isFile()); out[relative(root, path)] = fileSha(path);}
    }
  }
  visit(root); return out;
}

function rowFor(input) {
  const base = `runs/${input.case_id}`;
  return {case_id: input.case_id, scenario_path: `${base}/scenario.json`, flight_path: `${base}/flight.json`, summary_path: `${base}/summary.json`, rich_report_path: `${base}/report.html`};
}

export function checkCliMatrix(native, directory) {
  const batch = nativeBatch(native), matrix = readJson(directory, 'matrix.json');
  assert.equal(matrix.schema_id, SCHEMA); assert.equal(matrix.pack_id, PACK);
  assert.equal(matrix.status, 'completed'); assert.equal(matrix.passed, true);
  assert.equal(matrix.attempted_case_count, 44); assert.equal(matrix.cases.length, 44);
  assert.deepEqual(matrix.source_before, matrix.source_after); assert.equal(matrix.source_before.git_dirty, false);
  const source = batch.provenance.source_before;
  for (const key of ['git_commit', 'git_dirty', 'rust_source_tree_sha256']) assert.equal(matrix.source_before[key], source[key], `same-checkout ${key}`);
  assert.equal(matrix.source_before.evaluator_sha256, source.executable_sha256, 'native executable identity');
  assert.equal(matrix.native_summary_sha256, fileSha(safeFile(native, 'summary.json')));
  assert.deepEqual(matrix.input_identity, batch.input_identity);
  const inputs = readJson(native, 'expanded-inputs.json');
  assert.deepEqual(readJson(directory, 'expanded-inputs.json'), inputs);
  assert.equal(inputs.length, 44); assert.equal(new Set(inputs.map(x => x.case_id)).size, 44);
  const flights = new Set(), compact = new Set(); let rich = 0;
  const pieces = [], finalizations = [], stages = {planning_s: [], execution_s: [], replay_s: []};
  const totals = {mandatory_landings: 0, direct_clear_landings: 0, corrected_terrain_landings: 0, diagnostic_landings: 0, diagnostic_finite_stops: 0, unsupported: 0, integrity: 0, supported_source_replay: 0};
  for (const [i, input] of inputs.entries()) {
    const saved = matrix.cases[i], nativeRow = batch.cases[i];
    assert.equal(saved.case_id, input.case_id); assert.equal(nativeRow.case_id, input.case_id);
    assert.deepEqual(saved.paths, rowFor(input));
    const root = join(directory, 'runs', input.case_id);
    const {flight} = verifyBundle(root, input, readJson(native, nativeRow.flight_path));
    checkCaseOutcome(input, flight, saved.exit_code);
    const trace = verifyProgress(readJson(root, 'progress.json'), flight, input.case_id);
    if (!input.expected_preflight) {
      pieces.push(...trace.entries.map(e => e.elapsed_s)); finalizations.push(trace.finalization_elapsed_s);
      for (const field of Object.keys(stages)) stages[field].push(flight.timings[field]);
    }
    const stdout = readJson(directory, `logs/${input.case_id}.stdout.json`);
    assert.deepEqual(stdout, saved.stdout, `${input.case_id}: saved stdout receipt`);
    assert.equal(stdout.schema_id, 'planner_v2_cli_flight_v1'); assert.equal(stdout.status, 'completed');
    assert.equal(stdout.supported, input.expected_preflight === null);
    assert.deepEqual(stdout.policy, flight.policy);
    assert.equal(typeof stdout.output_dir, 'string');
    assert.equal(resolve(stdout.output_dir), resolve(root), `${input.case_id}: stdout output root`);
    for (const field of ['planning_stop', 'reason', 'physical_outcome', 'mission_outcome', 'integrity_passed', 'final_source_replay_passed', 'correction_count', 'input_identity']) assert.deepEqual(stdout[field], flight[field], `${input.case_id}: stdout ${field}`);
    const pair = flightPair(native, nativeRow, directory, saved.paths);
    pair.flightExclusions.forEach(p => flights.add(p)); pair.compactExclusions.forEach(p => compact.add(p)); rich += Number(pair.rich);
    totals.integrity++;
    if (input.expected_preflight) totals.unsupported++;
    else {
      totals.supported_source_replay++;
      if (input.group === 'diagnostic') totals[landed(flight) ? 'diagnostic_landings' : 'diagnostic_finite_stops']++;
      else {totals.mandatory_landings++; totals[input.group === 'clear' ? 'direct_clear_landings' : 'corrected_terrain_landings']++;}
    }
  }
  assert.deepEqual(totals, {mandatory_landings: 36, direct_clear_landings: 11, corrected_terrain_landings: 25, diagnostic_landings: 2, diagnostic_finite_stops: 4, unsupported: 2, integrity: 44, supported_source_replay: 42});
  assert.deepEqual(totals, matrix.totals);
  const actual = inventory(directory); delete actual['matrix.json'];
  assert.deepEqual(actual, matrix.artifact_sha256, 'complete matrix artifact inventory');
  return {schema_id: 'planner_v2_evaluator_cli_parity_v1', passed: true, cases: 44, compared_case_json_artifacts: 132, compared_rich_payloads: rich, totals,
    native_provenance: batch.provenance, cli_source_before: matrix.source_before, cli_source_after: matrix.source_after,
    flight_timing_exclusions: [...flights].sort(), compact_timing_exclusions: [...compact].sort(),
    latency: {scope: 'Supported session advance wall time includes planning, queries, admission/prefix/certificate proofs and execution. Finalization is separately timed final source proof. Existing flight timing buckets are preserved and are not total piece work.',
      advance_piece_wall: latencyStats(pieces), finalization_wall: latencyStats(finalizations),
      existing_flight_buckets: Object.fromEntries(Object.entries(stages).map(([k, values]) => [k, latencyStats(values)])),
      hard_realtime_claim: false, timing_success_threshold: null}};
}

export function compareCliRepeat(first, repeat) {
  const a = readJson(first, 'matrix.json'), b = readJson(repeat, 'matrix.json');
  assert.notEqual(realpathSync(first), realpathSync(repeat));
  assert.equal(a.passed, true); assert.equal(b.passed, true);
  assert.deepEqual(a.source_before, b.source_before, 'repeat source/both binaries');
  assert.deepEqual(a.source_before, a.source_after); assert.deepEqual(b.source_before, b.source_after);
  assert.deepEqual(a.input_identity, b.input_identity); assert.deepEqual(a.totals, b.totals);
  assert.deepEqual(readJson(first, 'expanded-inputs.json'), readJson(repeat, 'expanded-inputs.json'));
  assert.equal(a.cases.length, 44); assert.equal(b.cases.length, 44);
  let rich = 0;
  for (const [i, left] of a.cases.entries()) {
    const right = b.cases[i]; assert.equal(left.case_id, right.case_id); assert.equal(left.exit_code, right.exit_code);
    rich += Number(flightPair(first, left.paths, repeat, right.paths).rich);
    const x = readJson(first, `runs/${left.case_id}/bundle.json`), y = readJson(repeat, `runs/${left.case_id}/bundle.json`);
    // Different timings change artifact digests. Request, explicit policy and
    // input identities still compare in full, not a generic provenance scrub.
    delete x.artifact_sha256; delete y.artifact_sha256;
    assert.deepEqual(x, y, `${left.case_id}: repeat complete request/receipt binding`);
    const traceA = readJson(first, `runs/${left.case_id}/progress.json`), traceB = readJson(repeat, `runs/${left.case_id}/progress.json`);
    for (const trace of [traceA, traceB]) {
      assert(Number.isFinite(trace.finalization_elapsed_s)); delete trace.finalization_elapsed_s;
      for (const item of trace.entries) {assert(Number.isFinite(item.elapsed_s)); delete item.elapsed_s;}
    }
    assert.deepEqual(traceA, traceB, `${left.case_id}: repeat piece/actual handoff progress`);
    const stdoutA = {...left.stdout}, stdoutB = {...right.stdout};
    assert.equal(resolve(stdoutA.output_dir), resolve(first, 'runs', left.case_id));
    assert.equal(resolve(stdoutB.output_dir), resolve(repeat, 'runs', left.case_id));
    delete stdoutA.output_dir; delete stdoutB.output_dir;
    assert.deepEqual(stdoutA, stdoutB, `${left.case_id}: repeat outcome`);
  }
  return {schema_id: 'planner_v2_cli_repeat_parity_v1', passed: true, cases: 44, compared_case_json_artifacts: 132, compared_rich_payloads: rich, source: a.source_before,
    receipt_exclusions: ['artifact_sha256 (derived timing-dependent hashes, independently verified)'], stdout_exclusions: ['output_dir (proven distinct roots)'],
    progress_exclusions: ['/entries/*/elapsed_s', '/finalization_elapsed_s']};
}

function invoke(executable, args, cwd) {
  return new Promise((ok, bad) => {
    const child = spawn(executable, args, {cwd, stdio: ['ignore', 'pipe', 'pipe']});
    let stdout = '', stderr = '', bytes = 0;
    const timer = setTimeout(() => {child.kill('SIGTERM'); bad(new Error('CLI invocation exceeded 120 seconds'));}, 120_000);
    const collect = stream => chunk => {
      bytes += chunk.length;
      if (bytes > 32 * 1024 * 1024) {child.kill('SIGTERM'); bad(new Error('CLI output exceeded bounded allowance'));}
      else if (stream === 'stdout') stdout += chunk; else stderr += chunk;
    };
    child.stdout.on('data', collect('stdout')); child.stderr.on('data', collect('stderr'));
    child.on('error', error => {clearTimeout(timer); bad(error);});
    child.on('close', (code, signal) => {
      clearTimeout(timer);
      if (signal) bad(new Error(`CLI terminated: ${signal}`));
      else ok({code, stdout, stderr});
    });
  });
}

async function captureCli(options) {
  const {root, native, evaluator, cli, first, baseline} = options;
  const directory = validateOutputSeparation(root, options.directory, [native, baseline, ...(first ? [first] : [])]);
  nativeAcceptance(evaluator, native);
  const batch = nativeBatch(native), inputs = readJson(native, 'expanded-inputs.json');
  checkInputSeals(root, batch);
  assert(baseline, '--baseline accepted historical capture required before CLI measurement');
  nativeAcceptance(evaluator, baseline);
  compareNativePhysical(baseline, native);
  if (first) checkCliMatrix(native, first);
  const before = sourceState(root, evaluator, cli);
  assert.equal(before.git_dirty, false, 'freeze a clean source before measured attempts');
  for (const key of ['git_commit', 'git_dirty', 'rust_source_tree_sha256']) assert.equal(before[key], batch.provenance.source_before[key], `native/source freeze ${key}`);
  assert.equal(before.evaluator_sha256, batch.provenance.source_before.executable_sha256);
  if (first) assert.deepEqual(before, readJson(first, 'matrix.json').source_before, 'repeat must use the same frozen source and binaries');
  mkdirSync(directory);
  for (const folder of ['inputs', 'runs', 'logs']) mkdirSync(join(directory, folder));
  write(join(directory, 'expanded-inputs.json'), inputs);
  const matrix = {schema_id: SCHEMA, pack_id: PACK, status: 'failed', passed: false, attempted_case_count: 0,
    native_capture: native, native_summary_sha256: fileSha(safeFile(native, 'summary.json')), input_identity: batch.input_identity, source_before: before, cases: [],
    baseline_capture: baseline, baseline_summary_sha256: fileSha(safeFile(baseline, 'summary.json'))};
  try {
    for (const [i, input] of inputs.entries()) {
      assert(/^[a-zA-Z0-9_-]+$/.test(input.case_id));
      const scenario = join(directory, 'inputs', `${input.case_id}.json`), output = join(directory, 'runs', input.case_id);
      write(scenario, input.scenario);
      matrix.attempted_case_count++;
      const result = await invoke(cli, ['waypoint-v2-flight', scenario, '--source-pad', input.source_pad_id, '--target-pad', input.target_pad_id, '--output-dir', output], directory);
      writeFileSync(join(directory, 'logs', `${input.case_id}.stdout.json`), result.stdout, {flag: 'wx'});
      writeFileSync(join(directory, 'logs', `${input.case_id}.stderr.txt`), result.stderr, {flag: 'wx'});
      const row = {case_id: input.case_id, exit_code: result.code, stdout: JSON.parse(result.stdout), paths: rowFor(input)};
      matrix.cases.push(row);
      const {flight} = verifyBundle(output, input, readJson(native, batch.cases[i].flight_path));
      checkCaseOutcome(input, flight, result.code);
      flightPair(native, batch.cases[i], directory, row.paths);
      checkInputSeals(root, batch); assert.deepEqual(sourceState(root, evaluator, cli), before, 'source/binary changed during CLI matrix');
      process.stderr.write(`${i + 1}/44 ${input.case_id}: ${flight.planning_stop} (${flight.correction_count} handoffs)\n`);
    }
    matrix.totals = {mandatory_landings: 36, direct_clear_landings: 11, corrected_terrain_landings: 25, diagnostic_landings: 2, diagnostic_finite_stops: 4, unsupported: 2, integrity: 44, supported_source_replay: 42};
    matrix.status = 'completed'; matrix.passed = true;
  } catch (error) {matrix.error = error.stack ?? String(error); throw error;}
  finally {
    matrix.source_after = sourceState(root, evaluator, cli);
    matrix.artifact_sha256 = inventory(directory);
    write(join(directory, 'matrix.json'), matrix);
  }
  const parity = checkCliMatrix(native, directory);
  const repeat = first ? compareCliRepeat(first, directory) : null;
  return {matrix: join(directory, 'matrix.json'), parity, repeat};
}

async function savedReplays({root, native, directory, evaluator, cli, output: requestedOutput}) {
  const output = validateOutputSeparation(root, requestedOutput, [native, directory]);
  const parity = checkCliMatrix(native, directory);
  assert.equal(fileSha(cli), parity.cli_source_before.cli_sha256, 'saved replay CLI binary');
  nativeAcceptance(evaluator, native);
  const sourceBefore = sourceState(root, evaluator, cli);
  assert.deepEqual(sourceBefore, parity.cli_source_before, 'saved replay uses frozen matrix source/both binaries');
  const matrix = readJson(directory, 'matrix.json'), inputs = readJson(directory, 'expanded-inputs.json');
  mkdirSync(output);
  const before = inventory(directory), cases = [];
  const receipt = {schema_id: 'planner_v2_cli_saved_replay_matrix_v1', status: 'failed', passed: false,
    cli_sha256: fileSha(cli), matrix_sha256: fileSha(safeFile(directory, 'matrix.json')), source_before: sourceBefore, attempted_replays: 0, cases};
  try {
    for (const input of inputs.filter(x => !x.expected_preflight)) {
      receipt.attempted_replays++;
      const result = await invoke(cli, ['waypoint-v2-replay', '--bundle-dir', join(directory, 'runs', input.case_id)], output);
      writeFileSync(join(output, `${input.case_id}.stdout.json`), result.stdout, {flag: 'wx'});
      writeFileSync(join(output, `${input.case_id}.stderr.txt`), result.stderr, {flag: 'wx'});
      const outcome = JSON.parse(result.stdout);
      cases.push({case_id: input.case_id, exit_code: result.code, outcome});
      assert.equal(result.code, 0, `${input.case_id}: source replay failed: ${result.stderr}`);
      verifyReplayReceipt(outcome, readFileSync(safeFile(directory, `runs/${input.case_id}/flight.json`), 'utf8'), input.case_id);
      assert.equal(fileSha(cli), matrix.source_before.cli_sha256, 'CLI changed during replay');
      process.stderr.write(`${cases.length}/42 source replay: ${input.case_id}\n`);
    }
    assert.equal(cases.length, 42); assert.deepEqual(inventory(directory), before, 'saved replay mutated original evidence');
    checkInputSeals(root, nativeBatch(native));
    assert.deepEqual(sourceState(root, evaluator, cli), sourceBefore, 'source/both binaries changed during saved replay');
    receipt.status = 'completed'; receipt.passed = true;
  } catch (error) {receipt.error = error.stack ?? String(error); throw error;}
  finally {
    receipt.source_after = sourceState(root, evaluator, cli);
    receipt.artifact_sha256 = inventory(output);
    write(join(output, 'replays.json'), receipt);
  }
  checkSavedReplays(native, directory, output);
  return receipt;
}

export function checkSavedReplays(native, directory, output) {
  const parity = checkCliMatrix(native, directory);
  const receipt = readJson(output, 'replays.json');
  assert.equal(receipt.schema_id, 'planner_v2_cli_saved_replay_matrix_v1');
  assert.equal(receipt.status, 'completed'); assert.equal(receipt.passed, true);
  assert.equal(receipt.attempted_replays, 42); assert.equal(receipt.cases.length, 42);
  assert.deepEqual(receipt.source_before, parity.cli_source_before); assert.deepEqual(receipt.source_after, receipt.source_before);
  assert.equal(receipt.cli_sha256, receipt.source_before.cli_sha256);
  assert.equal(receipt.matrix_sha256, fileSha(safeFile(directory, 'matrix.json')));
  const inputs = readJson(directory, 'expanded-inputs.json').filter(x => !x.expected_preflight);
  for (const [i, input] of inputs.entries()) {
    const row = receipt.cases[i]; assert.equal(row.case_id, input.case_id); assert.equal(row.exit_code, 0);
    assert.deepEqual(row.outcome, readJson(output, `${input.case_id}.stdout.json`));
    verifyReplayReceipt(row.outcome, readFileSync(safeFile(directory, `runs/${input.case_id}/flight.json`), 'utf8'), input.case_id);
  }
  const actual = inventory(output); delete actual['replays.json'];
  assert.deepEqual(actual, receipt.artifact_sha256, 'saved replay artifact inventory');
  return {schema_id: 'planner_v2_saved_replay_check_v1', passed: true, supported_cases: 42, actual_component_digests_verified: 42 * 6, source: receipt.source_before};
}

export function expectedReplayDigests(flightRaw) {
  const fields = rawObjectFields(flightRaw);
  assert(fields.get('ordinary_flight') !== 'null', 'no physical flight to replay');
  const ordinary = rawObjectFields(fields.get('ordinary_flight'));
  const values = {manifest: fields.get('manifest')};
  for (const key of ['actions', 'events', 'samples', 'final_state', 'incoming_contact']) values[key] = ordinary.get(key);
  for (const [key, value] of Object.entries(values)) assert(typeof value === 'string', `missing replay component ${key}`);
  return Object.fromEntries(Object.entries(values).map(([key, value]) => [key, sha(value)]));
}

export function verifyReplayReceipt(outcome, flightRaw, label) {
  const flight = JSON.parse(flightRaw);
  assert.equal(outcome.schema_id, 'planner_v2_cli_replay_v1');
  assert.equal(outcome.replay_passed, true, `${label}: replay verdict`);
  for (const key of ['input_identity', 'planning_stop', 'physical_outcome', 'mission_outcome']) assert.deepEqual(outcome[key], flight[key], `${label}: replay ${key}`);
  assert.deepEqual(outcome.comparison_sha256, expectedReplayDigests(flightRaw), `${label}: actual replay actions/events/samples/endpoint/contact/manifest`);
  assert.equal(outcome.physics_step, flight.ordinary_flight.final_state.physics_step, `${label}: original replay endpoint clock`);
  assert.equal(outcome.sim_time_s, flight.ordinary_flight.final_state.sim_time_s, `${label}: original replay time`);
  return true;
}

async function main() {
  const {values} = parseArgs({options: {
    mode: {type: 'string'}, root: {type: 'string', default: ROOT}, native: {type: 'string'},
    baseline: {type: 'string'}, directory: {type: 'string'}, first: {type: 'string'},
    evaluator: {type: 'string'}, cli: {type: 'string'}, output: {type: 'string'}, help: {type: 'boolean'},
  }});
  if (values.help) {
    process.stdout.write('Planner V2 session/CLI validation (explicit create-only capture mode):\n'
      + '  --mode native-parity --baseline OLD_CAPTURE --native NEW_CAPTURE\n'
      + '  --mode capture-cli --baseline OLD_CAPTURE --native CAPTURE --directory NEW_DIR [--first CLI_MATRIX]\n'
      + '  --mode check-cli --native CAPTURE --directory CLI_MATRIX [--first CLI_MATRIX]\n'
      + '  --mode replay-cli --native CAPTURE --directory CLI_MATRIX --output NEW_DIR\n'
      + '  --mode check-replays --native CAPTURE --directory CLI_MATRIX --output REPLAY_DIR\n'
      + 'Optional --root REPO --evaluator PD_EVAL --cli FEATURE_ENABLED_PD_CLI.\n'
      + 'Only capture-cli plans new flights (44 max); replay-cli consumes saved commands (42 max).\n');
    return;
  }
  const root = realpathSync(resolve(values.root));
  const evaluator = resolve(values.evaluator ?? join(root, 'target/release/pd-eval'));
  const cli = resolve(values.cli ?? join(root, 'target/release/pd-cli'));
  assert(values.native, '--native required');
  const native = realpathSync(resolve(values.native));
  const directory = values.directory ? resolve(values.directory) : null;
  const first = values.first ? realpathSync(resolve(values.first)) : null;
  let result;
  switch (values.mode) {
    case 'native-parity':
      assert(values.baseline, '--baseline required'); nativeAcceptance(evaluator, native);
      nativeAcceptance(evaluator, realpathSync(resolve(values.baseline)));
      result = compareNativePhysical(realpathSync(resolve(values.baseline)), native); break;
    case 'capture-cli':
      assert(directory, '--directory required');
      assert(values.baseline, '--baseline required before measured CLI capture');
      result = await captureCli({root, native, directory, evaluator, cli, first, baseline: realpathSync(resolve(values.baseline))}); break;
    case 'check-cli':
      assert(directory, '--directory required'); nativeAcceptance(evaluator, native);
      if (first) checkCliMatrix(native, first);
      result = {parity: checkCliMatrix(native, directory), repeat: first ? compareCliRepeat(first, directory) : null}; break;
    case 'replay-cli':
      assert(directory && values.output, '--directory and --output required');
      result = await savedReplays({root, native, directory, evaluator, cli, output: resolve(values.output)}); break;
    case 'check-replays':
      assert(directory && values.output, '--directory and --output required'); nativeAcceptance(evaluator, native);
      result = checkSavedReplays(native, directory, realpathSync(resolve(values.output))); break;
    default: throw new Error('Choose explicit --mode (or --help)');
  }
  process.stdout.write(JSON.stringify(result, null, 2) + '\n');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {await main();} catch (error) {process.stderr.write(`${error.stack ?? error}\n`); process.exitCode = 1;}
}
