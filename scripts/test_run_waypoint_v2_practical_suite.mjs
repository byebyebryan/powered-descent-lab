// Focused harness contract test using a deterministic fake waypoint-v2 CLI.
import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const HARNESS = join(ROOT, 'scripts/run_waypoint_v2_practical_suite.mjs');
const TEMP_ROOT = mkdtempSync(join(tmpdir(), 'pd-v2-suite-harness-test-'));
const FAKE_CLI = join(TEMP_ROOT, 'fake-waypoint-v2-cli.mjs');

const fakeCli = `#!/usr/bin/env node
import {mkdirSync, readFileSync, writeFileSync} from 'node:fs';
const args = process.argv.slice(2);
const value = name => args[args.indexOf(name) + 1];
if (args[0] !== 'waypoint-v2-flight') throw new Error('unexpected command');
const scenarioPath = value('--scenario');
const outputDir = value('--output-dir');
const policyVersion = Number(value('--policy-version'));
if (![1, 2, 3].includes(policyVersion)) throw new Error('missing or invalid policy version');
const scenario = JSON.parse(readFileSync(scenarioPath, 'utf8'));
const id = scenario.id;
const clear = id.startsWith('v2_clear_');
const diagnostic = id.startsWith('v2_diag_');
const ordinary = !clear && !diagnostic;
const blocked = new Set([
  'v2_ridge_early', 'v2_ridge_mid', 'v2_ridge_late', 'v2_ridge_wide',
  'v2_plateau_reference_900', 'v2_plateau_mid', 'v2_plateau_wide', 'v2_plateau_late',
  'v2_successive_equal', 'v2_successive_rising', 'v2_successive_falling', 'v2_successive_plateaus',
]);
const rejected = id === 'v2_diag_lunar_gravity' || id === 'v2_diag_other_vehicle';
const injectedError = process.env.PD_FAKE_IMPLEMENTATION_ERROR_ID === id;
const injectedIntegrityFailure = process.env.PD_FAKE_INTEGRITY_FAILURE_ID === id;
const landed = !diagnostic && id !== 'v2_ridge_early' && !injectedError;
const planningStop = injectedError ? 'implementation_error'
  : rejected ? 'unsupported' : landed ? 'landed' : 'no_clearing';
const integrityPassed = !injectedIntegrityFailure;
const correctionCount = id === 'v2_plateau_reference_900' ? 3 : clear ? 0 : ordinary ? 1 : 0;
const planning = 0.2 + (process.env.PD_FAKE_TIMER_VARIANT === 'b' ? 0.01 : 0);
const timings = {planning_s: planning, execution_s: 0.1, replay_s: 0.05};
const result = {
  planning_stop: planningStop,
  reason: null,
  correction_count: correctionCount,
  initial_nominal_terrain_blocked: ordinary && blocked.has(id),
  integrity_passed: integrityPassed,
  physical_outcome: rejected ? null : landed ? 'landed_on_target' : 'flying',
  mission_outcome: rejected ? null : landed ? 'success' : 'in_progress',
  final_source_replay_passed: !rejected,
  timings,
};
const updates = [{physics_step: 0, command: {throttle_frac: 0.5, target_attitude_rad: 0}}];
const cycles = clear ? [{
  decision: 'landed', nominal_updates: updates,
  audit: {commands_match: true, passed: true}, local_search: null,
}] : [];
const segments = clear ? [{updates}] : [];
const runSummary = rejected ? null : {
  minimum_clearance: {touchdown_m: 5.5, hull_m: 6.5, landing: null},
  fuel: {remaining_kg: 100, used_kg: 140},
  endpoint: {
    physics_step: 100, sim_time_s: 100 / 120,
    physical_outcome: landed ? 'landed_on_target' : 'flying',
    mission_outcome: landed ? 'success' : 'in_progress', end_reason: 'running',
  },
};
const summary = {
  schema_id: 'waypoint_v2_flight_summary_v1',
  input_identity: id,
  policy: {policy_id: 'piecewise_local_clearing_v2_policy_' + policyVersion},
  result,
  run_summary: runSummary,
  timings: {output_s: 0.02, total_s: 0.4},
  scenario_path: scenarioPath,
  output_dir: outputDir,
};
const flight = {
  ...result,
  policy: {policy_id: 'piecewise_local_clearing_v2_policy_' + policyVersion},
  cycles, segments,
  state_marker: process.env.PD_FAKE_STATE_VARIANT || 'stable',
  scenario_path: scenarioPath,
  output_dir: outputDir,
};
mkdirSync(outputDir);
writeFileSync(outputDir + '/summary.json', JSON.stringify(summary, null, 2));
writeFileSync(outputDir + '/flight.json', JSON.stringify(flight, null, 2));
`;

function harness(args, env = {}) {
  return spawnSync(process.execPath, [HARNESS, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {...process.env, ...env},
  });
}

try {
  writeFileSync(FAKE_CLI, fakeCli);
  chmodSync(FAKE_CLI, 0o755);
  const rootA = join(TEMP_ROOT, 'full-a');
  const rootB = join(TEMP_ROOT, 'full-b');
  const rootC = join(TEMP_ROOT, 'state-changed');
  const rootV2 = join(TEMP_ROOT, 'policy-v2');
  const rootV3 = join(TEMP_ROOT, 'policy-v3');
  const rootV3Full = join(TEMP_ROOT, 'policy-v3-full');
  const outputA = harness(['--bin', FAKE_CLI, '--output-dir', rootA]);
  assert.equal(outputA.status, 0, outputA.stderr || outputA.stdout);
  const outputB = harness(['--bin', FAKE_CLI, '--output-dir', rootB], {
    PD_FAKE_TIMER_VARIANT: 'b',
  });
  assert.equal(outputB.status, 0, outputB.stderr || outputB.stdout);
  const comparison = harness(['--compare', rootA, rootB]);
  assert.equal(comparison.status, 0, comparison.stderr || comparison.stdout);
  assert.equal(JSON.parse(comparison.stdout).compared_case_artifacts, 64);

  const changed = harness(['--bin', FAKE_CLI, '--output-dir', rootC], {
    PD_FAKE_STATE_VARIANT: 'changed',
  });
  assert.equal(changed.status, 0, changed.stderr || changed.stdout);
  const stateComparison = harness(['--compare', rootA, rootC]);
  assert.equal(stateComparison.status, 1);
  assert.match(stateComparison.stderr, /flight\.json differs at \/state_marker/);

  const version2 = harness([
    '--bin', FAKE_CLI, '--output-dir', rootV2, '--policy-version', '2',
  ]);
  assert.equal(version2.status, 0, version2.stderr || version2.stdout);
  const version2Summary = JSON.parse(
    readFileSync(join(rootV2, 'suite-summary.json'), 'utf8'),
  );
  assert.equal(version2Summary.policy_version, 2);
  const versionComparison = harness(['--compare', rootA, rootV2]);
  assert.equal(versionComparison.status, 1);
  assert.match(versionComparison.stderr, /different waypoint V2 policy versions/);

  const version3 = harness([
    '--bin', FAKE_CLI, '--output-dir', rootV3, '--policy-version', '3',
    '--case', 'v2_clear_685',
  ]);
  assert.equal(version3.status, 0, version3.stderr || version3.stdout);
  const version3Summary = JSON.parse(
    readFileSync(join(rootV3, 'suite-summary.json'), 'utf8'),
  );
  const version3Flight = JSON.parse(
    readFileSync(join(rootV3, 'runs', 'v2_clear_685', 'flight.json'), 'utf8'),
  );
  assert.equal(version3Summary.schema_id, 'waypoint_v2_practical_suite_run_v1');
  assert.equal(version3Summary.policy_version, 3);
  assert.equal(version3Summary.provenance.unchanged, true);
  assert.deepEqual(version3Summary.acceptance, {
    evaluated: false,
    passed: null,
    reason: 'single-case development run',
  });
  assert.equal(version3Flight.policy.policy_id, 'piecewise_local_clearing_v2_policy_3');

  const version3Full = harness([
    '--bin', FAKE_CLI, '--output-dir', rootV3Full, '--policy-version', '3',
  ]);
  assert.equal(version3Full.status, 0, version3Full.stderr || version3Full.stdout);
  const version3FullSummary = JSON.parse(
    readFileSync(join(rootV3Full, 'suite-summary.json'), 'utf8'),
  );
  assert.equal(version3FullSummary.schema_id, 'waypoint_v2_practical_suite_run_v1');
  assert.equal(version3FullSummary.policy_version, 3);
  assert.equal(version3FullSummary.provenance.unchanged, true);
  assert.equal(version3FullSummary.acceptance.evaluated, true);
  assert.equal(version3FullSummary.acceptance.passed, true);
  const version3Comparison = harness(['--compare', rootA, rootV3Full]);
  assert.equal(version3Comparison.status, 1);
  assert.match(version3Comparison.stderr, /different waypoint V2 policy versions/);

  const failedRoot = join(TEMP_ROOT, 'implementation-error');
  const failed = harness(['--bin', FAKE_CLI, '--output-dir', failedRoot], {
    PD_FAKE_IMPLEMENTATION_ERROR_ID: 'v2_ridge_mid',
  });
  assert.equal(failed.status, 1);
  const partial = JSON.parse(readFileSync(join(failedRoot, 'suite-summary.json'), 'utf8'));
  assert.equal(partial.status, 'stopped');
  assert.equal(partial.cases.length, 10);
  assert.match(partial.stop_reason, /implementation_error/);

  const integrityRoot = join(TEMP_ROOT, 'integrity-error');
  const integrity = harness(['--bin', FAKE_CLI, '--output-dir', integrityRoot], {
    PD_FAKE_INTEGRITY_FAILURE_ID: 'v2_ridge_mid',
  });
  assert.equal(integrity.status, 1);
  const partialIntegrity = JSON.parse(
    readFileSync(join(integrityRoot, 'suite-summary.json'), 'utf8'),
  );
  assert.match(partialIntegrity.stop_reason, /integrity_passed is false/);

  const singleRoot = join(TEMP_ROOT, 'single-case');
  const single = harness([
    '--bin', FAKE_CLI, '--output-dir', singleRoot, '--case', 'v2_clear_685',
    '--policy-version', '2',
  ]);
  assert.equal(single.status, 0, single.stderr || single.stdout);
  const singleSummary = JSON.parse(readFileSync(join(singleRoot, 'suite-summary.json'), 'utf8'));
  assert.equal(singleSummary.cases.length, 1);
  assert.equal(singleSummary.acceptance.evaluated, false);
  assert.equal(singleSummary.policy_version, 2);

  process.stdout.write('fake CLI harness checks passed\n');
} finally {
  rmSync(TEMP_ROOT, {recursive: true, force: true});
}
