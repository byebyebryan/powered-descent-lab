#!/usr/bin/env node
// Read-only source retirement checks; never imports Python or opens saved runs.
import {createHash} from 'node:crypto';
import {lstatSync, readFileSync, readdirSync, readlinkSync} from 'node:fs';
import {dirname, isAbsolute, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const ARCHIVE = 'studies/rl_procedural_ppo';
const ORIGINAL = 'experiments/rl';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function inside(root, path) {
  if (typeof path !== 'string' || !path || isAbsolute(path)
    || path.includes('\\') || path.split('/').some(part => !part || part === '.' || part === '..')) {
    throw new Error(`invalid relative source path: ${path}`);
  }
  let current = root;
  for (const part of path.split('/').slice(0, -1)) {
    current = join(current, part);
    const stat = lstatSync(current);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error(`source parent is not a regular directory: ${path}`);
  }
  return join(root, path);
}

function regularBytes(root, path, limit = Infinity) {
  const target = inside(root, path);
  const stat = lstatSync(target);
  if (!stat.isFile() || stat.size > limit) throw new Error(`requires a bounded regular file: ${path}`);
  return readFileSync(target);
}

function verifyEntry(root, entry, field, issues) {
  try {
    if (!/^[0-9a-f]{64}$/.test(entry.sha256) || !Number.isSafeInteger(entry.bytes) || entry.bytes < 0) {
      throw new Error('invalid source digest or byte count');
    }
    const path = inside(root, entry[field]);
    const stat = lstatSync(path);
    const kind = entry.kind ?? 'file';
    let bytes;
    if (kind === 'symlink' && stat.isSymbolicLink()) bytes = Buffer.from(readlinkSync(path));
    else if (kind === 'file' && stat.isFile()) bytes = readFileSync(path);
    else throw new Error('source type differs');
    if (bytes.length !== entry.bytes || sha256(bytes) !== entry.sha256) throw new Error('source bytes differ');
    if (kind === 'file' && Number.isInteger(entry.mode) && (stat.mode & 0o111) !== (entry.mode & 0o111)) {
      throw new Error('source executable mode differs');
    }
  } catch (error) { issues.push(`${entry[field]}: ${error.message}`); }
}

export function checkRlRetirement({root = ROOT, researchWorktree} = {}) {
  root = resolve(root);
  const result = {archivedFiles: 0, researchFiles: 0, researchFilesVerified: 0, issues: []};
  const {issues} = result;
  try {
    const manifest = JSON.parse(regularBytes(root, `${ARCHIVE}/retirement.json`, 2 * 1024 * 1024));
    if (manifest.schema_version !== 1 || manifest.status !== 'retired'
      || manifest.original_package !== ORIGINAL || manifest.archive_package !== ARCHIVE
      || !Array.isArray(manifest.entries) || !manifest.entries.length) throw new Error('invalid retirement manifest');
    const sources = new Set();
    const archives = new Set();
    for (const entry of manifest.entries) {
      if (typeof entry.source_path !== 'string' || !entry.source_path.startsWith(`${ORIGINAL}/`)
        || entry.source_path.slice(ORIGINAL.length + 1).includes('/')
        || typeof entry.archive_path !== 'string' || !entry.archive_path.startsWith(`${ARCHIVE}/`)
        || sources.has(entry.source_path) || archives.has(entry.archive_path)) throw new Error('invalid or duplicate archived source mapping');
      sources.add(entry.source_path); archives.add(entry.archive_path);
      verifyEntry(root, entry, 'archive_path', issues);
      result.archivedFiles += 1;
    }
    // Reserve the retired namespace. Existing installed runtimes remain local;
    // no new Python entrypoint or dependency manifest may appear here.
    for (const entry of readdirSync(join(root, ORIGINAL), {withFileTypes: true})) {
      if (!['README.md', '.gitignore', '.venv'].includes(entry.name)) issues.push(`retired namespace contains ${entry.name}`);
      if (entry.name === '.venv' && !entry.isDirectory()) issues.push('retained .venv must be a directory');
    }
    regularBytes(root, `${ORIGINAL}/README.md`, 64 * 1024);
    const inventoryBytes = regularBytes(root, `${ARCHIVE}/research-source-inventory.json`, 2 * 1024 * 1024);
    if (sha256(inventoryBytes) !== manifest.research_inventory_sha256) throw new Error('research inventory byte identity differs');
    const inventory = JSON.parse(inventoryBytes);
    if (inventory.schema_version !== 1 || inventory.status !== 'historical_source_retained_in_original_worktree'
      || !Array.isArray(inventory.files)) throw new Error('invalid research source inventory');
    const paths = new Set();
    for (const entry of inventory.files) {
      // Validate paths even when the optional research checkout is unavailable.
      if (typeof entry.path !== 'string' || !entry.path || isAbsolute(entry.path)
        || entry.path.includes('\\') || entry.path.split('/').some(p => !p || p === '.' || p === '..')
        || paths.has(entry.path)) throw new Error('invalid or duplicate research source path');
      paths.add(entry.path);
      if (researchWorktree !== undefined) verifyEntry(resolve(researchWorktree), entry, 'path', issues);
    }
    result.researchFiles = inventory.files.length;
    result.researchFilesVerified = researchWorktree === undefined ? 0 : inventory.files.length;
  } catch (error) { issues.push(error.message); }
  return result;
}

export function main(args = process.argv.slice(2)) {
  const {values} = parseArgs({args, options: {help: {type: 'boolean'}, 'research-worktree': {type: 'string'}}, strict: true, allowPositionals: false});
  if (values.help) {
    process.stdout.write('Usage: node scripts/check-rl-retirement.mjs [--research-worktree PATH]\nRead-only archive hashes and retired entrypoint checks; local research checkout is optional.\n');
    return 0;
  }
  if (values['research-worktree'] !== undefined && !values['research-worktree'].trim()) throw new Error('research worktree path must be nonempty');
  const result = checkRlRetirement({researchWorktree: values['research-worktree']});
  for (const issue of result.issues) process.stderr.write(`${issue}\n`);
  process.stdout.write(`RL retirement: ${result.archivedFiles} archived files; ${result.researchFilesVerified}/${result.researchFiles} optional research files checked; ${result.issues.length} issues.\n`);
  return result.issues.length ? 1 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = main(); }
  catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }
}
