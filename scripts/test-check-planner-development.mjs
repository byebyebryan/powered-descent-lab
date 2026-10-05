import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {
  buildPlannerDevelopmentSteps,
  discoverNodeTests,
  main,
  parsePlannerDevelopmentArgs,
  runPlannerDevelopmentSteps,
} from './check-planner-development.mjs';

function syntheticRoot() {
  const root = mkdtempSync(join(tmpdir(), 'pd-planner-development-gate-'));
  mkdirSync(join(root, 'scripts'));
  writeFileSync(join(root, 'scripts', 'test-z.mjs'), '');
  writeFileSync(join(root, 'scripts', 'check-not-a-test.mjs'), '');
  writeFileSync(join(root, 'scripts', 'test-a.mjs'), '');
  mkdirSync(join(root, 'scripts', 'test-directory.mjs'));
  return root;
}

function outputFor(step) {
  if (step.label === 'feature-enabled CLI help') {
    return 'Commands: run replay report waypoint-v2-flight waypoint-v2-replay';
  }
  if (step.label === 'default-off CLI help') return 'Commands: run replay report';
  if (step.label === 'default-off normal dependency tree') return 'pd-cli v0.1.0';
  return '';
}

test('argument parser accepts only the optional retained parity path', () => {
  assert.deepEqual(parsePlannerDevelopmentArgs([]), {help: false, parityCapture: undefined});
  assert.deepEqual(parsePlannerDevelopmentArgs(['--parity-capture', 'capture/path']), {
    help: false,
    parityCapture: 'capture/path',
  });
  assert.equal(parsePlannerDevelopmentArgs(['--help']).help, true);
  for (const args of [
    ['--unknown'],
    ['unexpected'],
    ['--parity-capture'],
    ['--parity-capture='],
    ['--parity-capture', 'one', '--parity-capture', 'two'],
  ]) assert.throws(() => parsePlannerDevelopmentArgs(args), args.join(' '));
});

test('ordinary and explicit parity plans preserve feature and test discovery boundaries', () => {
  const root = syntheticRoot();
  try {
    assert.deepEqual(discoverNodeTests(root), ['scripts/test-a.mjs', 'scripts/test-z.mjs']);
    const ordinary = buildPlannerDevelopmentSteps({repositoryRoot: root});
    assert(!ordinary.some(step => step.label.includes('parity')));
    const find = label => ordinary.find(step => step.label === label);
    assert.deepEqual(find('workspace all-features tests').args.slice(0, 4), [
      'proxy', 'cargo', 'test', '--workspace',
    ]);
    assert(find('workspace all-features tests').args.includes('--all-features'));
    assert(find('planner-v2 CLI tests').args.includes('--features'));
    assert(find('planner-v2 CLI tests').args.includes('planner-v2'));
    assert(find('default-off CLI tests').args.includes('--no-default-features'));
    assert(find('default-off CLI help').args.includes('--no-default-features'));
    assert(find('feature-enabled CLI help').validateOutput);
    assert(find('default-off normal dependency tree').args.includes('--edges'));
    assert(find('default-off normal dependency tree').validateOutput);
    assert(find('maintained JavaScript tests').args.includes('scripts/test-a.mjs'));
    assert(find('maintained JavaScript tests').args.includes('scripts/test-z.mjs'));
    assert(ordinary.every(step => step.command === 'rtk' && step.args[0] === 'proxy'));

    const parity = buildPlannerDevelopmentSteps({
      repositoryRoot: root,
      parityCapture: 'capture folder/accepted',
    });
    const parityStep = parity.at(-1);
    assert.equal(parityStep.label, 'retained 44-case numerical parity');
    assert(parityStep.args.includes('--release'));
    assert(parityStep.args.includes('waypoint_v2::tests::retained_capture_numerical_parity'));
    assert.equal(parityStep.env.PD_V2_PARITY_CAPTURE, join(root, 'capture folder/accepted'));
    assert(!parityStep.args.some(argument => argument.includes('capture folder')),
      'capture paths travel through environment data, never shell text');
  } finally {
    rmSync(root, {recursive: true, force: true});
  }
});

test('injected execution is fail-fast and ordinary mode creates no capture artifacts', () => {
  const root = syntheticRoot();
  const externalCapture = join(root, 'accepted capture');
  try {
    const steps = buildPlannerDevelopmentSteps({repositoryRoot: root});
    const attempted = [];
    assert.throws(() => runPlannerDevelopmentSteps(steps, {
      write: () => {},
      execute: step => {
        attempted.push(step.label);
        return {
          status: step.label === 'default-off CLI tests' ? 17 : 0,
          stdout: outputFor(step),
          stderr: step.label === 'default-off CLI tests' ? 'synthetic failure' : '',
        };
      },
    }), /default-off CLI tests failed with exit status 17/);
    assert.deepEqual(attempted, [
      'workspace all-features tests',
      'planner-v2 CLI tests',
      'default-off CLI tests',
    ]);

    const completed = [];
    const count = main([], {
      repositoryRoot: root,
      write: () => {},
      execute: step => {
        completed.push(step.label);
        return {status: 0, stdout: outputFor(step), stderr: ''};
      },
    });
    assert.equal(count, steps.length);
    assert.deepEqual(completed, steps.map(step => step.label));
    assert.equal(existsSync(externalCapture), false);
    assert.equal(existsSync(join(root, 'outputs')), false);
  } finally {
    rmSync(root, {recursive: true, force: true});
  }
});

test('default-off leaks and command spawn errors fail before later steps', () => {
  const root = syntheticRoot();
  const steps = buildPlannerDevelopmentSteps({repositoryRoot: root});
  const failures = [
    {
      label: 'default-off CLI help',
      result: {status: 0, stdout: 'Commands: run replay report waypoint-v2-flight', stderr: ''},
      message: /must not expose optional V2 commands/,
    },
    {
      label: 'default-off normal dependency tree',
      result: {status: 0, stdout: 'pd-cli v0.1.0\n└── pd-eval v0.1.0', stderr: ''},
      message: /pd-eval must stay out of the default-off normal dependency tree/,
    },
    {
      label: 'workspace all-features tests',
      result: {status: null, stdout: '', stderr: '', error: new Error('synthetic spawn error')},
      message: /workspace all-features tests failed with exit status null:.*synthetic spawn error/s,
    },
  ];

  try {
    for (const failure of failures) {
      const failingIndex = steps.findIndex(step => step.label === failure.label);
      const attempted = [];
      assert.throws(() => runPlannerDevelopmentSteps(steps, {
        write: () => {},
        execute: step => {
          attempted.push(step.label);
          return step.label === failure.label
            ? failure.result
            : {status: 0, stdout: outputFor(step), stderr: ''};
        },
      }), failure.message);
      assert.deepEqual(
        attempted,
        steps.slice(0, failingIndex + 1).map(step => step.label),
        `${failure.label} must stop the runner before later checks`,
      );
    }
    assert.equal(existsSync(join(root, 'outputs')), false);
  } finally {
    rmSync(root, {recursive: true, force: true});
  }
});

test('main rejects bad arguments before any synthetic command runs', () => {
  const root = syntheticRoot();
  let calls = 0;
  try {
    assert.throws(() => main(['--parity-capture='], {
      repositoryRoot: root,
      write: () => {},
      execute: () => { calls++; return {status: 0}; },
    }), /nonempty path/);
    assert.equal(calls, 0);
    assert.equal(existsSync(join(root, 'outputs')), false);
  } finally {
    rmSync(root, {recursive: true, force: true});
  }
});
