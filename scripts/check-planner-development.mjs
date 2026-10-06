#!/usr/bin/env node
// Maintained local developer gate for current planner and CLI boundaries.
// Captured numerical parity remains an explicit opt-in read-only check.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {readdirSync} from 'node:fs';
import {dirname, isAbsolute, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';

export const REPOSITORY_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const USAGE = [
  'Usage: node scripts/check-planner-development.mjs [--parity-capture PATH]',
  '',
  'Runs ordinary workspace, CLI, formatting, Clippy, documentation and maintained Node checks.',
  'The exact 44-case retained-capture regression runs only when --parity-capture is given.',
].join('\n');

export function parsePlannerDevelopmentArgs(args) {
  const captureOptions = args.filter(argument =>
    argument === '--parity-capture' || argument.startsWith('--parity-capture='));
  if (captureOptions.length > 1) {
    throw new Error('--parity-capture may be supplied only once');
  }
  const parsed = parseArgs({
    args,
    options: {
      help: {type: 'boolean', short: 'h'},
      'parity-capture': {type: 'string'},
    },
    allowPositionals: false,
    strict: true,
  });
  const capture = parsed.values['parity-capture'];
  if (capture !== undefined && !capture.trim()) {
    throw new Error('--parity-capture requires a nonempty path');
  }
  return {
    help: parsed.values.help ?? false,
    parityCapture: capture,
  };
}

export function discoverNodeTests(repositoryRoot = REPOSITORY_ROOT) {
  const scriptsDirectory = join(repositoryRoot, 'scripts');
  return readdirSync(scriptsDirectory, {withFileTypes: true})
    .filter(entry => entry.isFile() && /^test.*\.mjs$/.test(entry.name))
    .map(entry => `scripts/${entry.name}`)
    .sort();
}

const hasText = (output, pattern, message) => {
  assert.match(output, pattern, message);
};

function step(repositoryRoot, label, commandArgs, options = {}) {
  return {
    label,
    command: 'rtk',
    args: ['proxy', ...commandArgs],
    cwd: repositoryRoot,
    env: options.env ?? {},
    captureOutput: options.captureOutput ?? false,
    validateOutput: options.validateOutput,
  };
}

export function buildPlannerDevelopmentSteps({
  repositoryRoot = REPOSITORY_ROOT,
  parityCapture,
} = {}) {
  const root = resolve(repositoryRoot);
  const nodeTests = discoverNodeTests(root);
  assert(nodeTests.length > 0, 'no scripts/test*.mjs Node tests were found');
  const steps = [
    step(root, 'workspace all-features tests', [
      'cargo', 'test', '--workspace', '--all-features',
    ]),
    step(root, 'planner-v2 CLI tests', [
      'cargo', 'test', '-p', 'pd-cli', '--features', 'planner-v2',
    ]),
    step(root, 'default-off CLI tests', [
      'cargo', 'test', '-p', 'pd-cli', '--no-default-features',
    ]),
    step(root, 'feature-enabled CLI help', [
      'cargo', 'run', '-p', 'pd-cli', '--features', 'planner-v2', '--', '--help',
    ], {
      captureOutput: true,
      validateOutput: output => {
        hasText(output, /waypoint-v2-flight/, 'feature help must expose V2 flight');
        hasText(output, /waypoint-v2-replay/, 'feature help must expose V2 replay');
      },
    }),
    step(root, 'default-off CLI help', [
      'cargo', 'run', '-p', 'pd-cli', '--no-default-features', '--', '--help',
    ], {
      captureOutput: true,
      validateOutput: output => {
        for (const command of ['run', 'replay', 'report']) {
          hasText(output, new RegExp(`\\b${command}\\b`), `default help must expose ${command}`);
        }
        assert.doesNotMatch(output, /waypoint-v2-(?:flight|replay)/,
          'default-off help must not expose optional V2 commands');
      },
    }),
    step(root, 'default-off normal dependency tree', [
      'cargo', 'tree', '-p', 'pd-cli', '--no-default-features', '--edges', 'normal',
    ], {
      captureOutput: true,
      validateOutput: output => assert.doesNotMatch(output, /\bpd-eval v\d/,
        'pd-eval must stay out of the default-off normal dependency tree'),
    }),
    step(root, 'Rust formatting', ['cargo', 'fmt', '--all', '--', '--check']),
    step(root, 'diff whitespace', ['git', 'diff', '--check']),
    step(root, 'strict all-target Clippy', [
      'cargo', 'clippy', '--workspace', '--all-targets', '--all-features', '--',
      '-D', 'warnings',
    ]),
    step(root, 'maintained JavaScript tests', ['node', '--test', ...nodeTests]),
    step(root, 'local documentation links', ['node', 'scripts/check-docs.mjs']),
  ];

  if (parityCapture !== undefined) {
    const capture = isAbsolute(parityCapture)
      ? resolve(parityCapture)
      : resolve(root, parityCapture);
    steps.push(step(root, 'retained 44-case numerical parity', [
      'cargo', 'test', '--release', '-p', 'pd-eval', '--lib',
      'waypoint_v2::tests::retained_capture_numerical_parity', '--', '--ignored', '--nocapture',
    ], {
      env: {PD_V2_PARITY_CAPTURE: capture},
    }));
  }

  return steps;
}

function executeStep(gateStep) {
  const result = spawnSync(gateStep.command, gateStep.args, {
    cwd: gateStep.cwd,
    env: {...process.env, ...gateStep.env},
    encoding: gateStep.captureOutput ? 'utf8' : undefined,
    stdio: gateStep.captureOutput ? ['ignore', 'pipe', 'pipe'] : 'inherit',
    shell: false,
    maxBuffer: 16 * 1024 * 1024,
  });
  return {
    status: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    error: result.error,
  };
}

export function runPlannerDevelopmentSteps(steps, {
  execute = executeStep,
  write = line => process.stdout.write(`${line}\n`),
} = {}) {
  for (const gateStep of steps) {
    write(`==> ${gateStep.label}`);
    const result = execute(gateStep);
    if (result.error || result.status !== 0) {
      const detail = [result.error?.message, result.stderr?.trim()]
        .filter(Boolean)
        .join('\n');
      throw new Error(
        `${gateStep.label} failed with exit status ${result.status}${detail ? `:\n${detail}` : ''}`,
      );
    }
    if (gateStep.validateOutput) gateStep.validateOutput(result.stdout ?? '');
    write(`PASS ${gateStep.label}`);
  }
  return steps.length;
}

export function main(args = process.argv.slice(2), dependencies = {}) {
  const parsed = parsePlannerDevelopmentArgs(args);
  const write = dependencies.write ?? (line => process.stdout.write(`${line}\n`));
  if (parsed.help) {
    write(USAGE);
    return 0;
  }
  const steps = buildPlannerDevelopmentSteps({
    repositoryRoot: dependencies.repositoryRoot ?? REPOSITORY_ROOT,
    parityCapture: parsed.parityCapture,
  });
  const completed = runPlannerDevelopmentSteps(steps, {
    execute: dependencies.execute ?? executeStep,
    write,
  });
  write(`Completed ${completed} planner development checks.`);
  return completed;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error?.stack ?? error}\n${USAGE}\n`);
    process.exitCode = 1;
  }
}
