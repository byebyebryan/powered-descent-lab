import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {checkRlRetirement} from './check-rl-retirement.mjs';

const ARCHIVE = 'studies/rl_procedural_ppo';
const sha = value => createHash('sha256').update(value).digest('hex');

function fixture(run) {
  const root = mkdtempSync(join(tmpdir(), 'pd-rl-retirement-'));
  try {
    mkdirSync(join(root, ARCHIVE), {recursive: true});
    mkdirSync(join(root, 'experiments/rl'), {recursive: true});
    writeFileSync(join(root, 'experiments/rl/README.md'), '# Retired\n');
    const bytes = 'print("historical source; never executed")\n';
    writeFileSync(join(root, ARCHIVE, 'train.py'), bytes);
    const research = {schema_version: 1, status: 'historical_source_retained_in_original_worktree', files: [
      {path: 'studies/rl_example/reader.py', kind: 'file', bytes: 7, sha256: sha('reader\n')},
    ]};
    const inventory = JSON.stringify(research);
    writeFileSync(join(root, ARCHIVE, 'research-source-inventory.json'), inventory);
    const manifest = {
      schema_version: 1, status: 'retired', original_package: 'experiments/rl', archive_package: ARCHIVE,
      research_inventory_sha256: sha(inventory), entries: [
        {source_path: 'experiments/rl/train.py', archive_path: `${ARCHIVE}/train.py`, bytes: Buffer.byteLength(bytes), sha256: sha(bytes)},
      ],
    };
    const save = () => writeFileSync(join(root, ARCHIVE, 'retirement.json'), JSON.stringify(manifest));
    save();
    run({root, manifest, research, save});
  } finally { rmSync(root, {recursive: true, force: true}); }
}

test('archive validation needs no local environment, source checkout or retained outputs', () => {
  fixture(({root}) => {
    mkdirSync(join(root, 'experiments/rl/.venv'));
    assert.deepEqual(checkRlRetirement({root}), {
      archivedFiles: 1, researchFiles: 1, researchFilesVerified: 0, issues: [],
    });
  });
});

test('changed and missing historical source fail without repairing the archive', () => {
  fixture(({root}) => {
    const path = join(root, ARCHIVE, 'train.py');
    writeFileSync(path, 'changed');
    assert.match(checkRlRetirement({root}).issues.join('\n'), /source bytes differ/);
    assert.equal(readFileSync(path, 'utf8'), 'changed');
    rmSync(path);
    assert.match(checkRlRetirement({root}).issues.join('\n'), /ENOENT/);
  });
});

test('old entrypoints and dependency manifests cannot reappear in the retired namespace', () => {
  fixture(({root}) => {
    for (const name of ['train.py', 'pyproject.toml', 'uv.lock', 'new_launch.py']) {
      const path = join(root, 'experiments/rl', name);
      writeFileSync(path, 'unexpected');
      assert(checkRlRetirement({root}).issues.some(issue => issue.includes(name)));
      rmSync(path);
    }
  });
});

test('symlink source and parent substitution cannot satisfy a source mapping', () => {
  fixture(({root}) => {
    const path = join(root, ARCHIVE, 'train.py');
    const moved = join(root, 'copied.py');
    writeFileSync(moved, readFileSync(path));
    rmSync(path); symlinkSync(moved, path);
    assert.match(checkRlRetirement({root}).issues.join('\n'), /source type differs/);
    rmSync(path); rmSync(moved);
    rmSync(join(root, ARCHIVE), {recursive: true});
    mkdirSync(join(root, 'replacement')); symlinkSync(join(root, 'replacement'), join(root, ARCHIVE));
    assert.match(checkRlRetirement({root}).issues.join('\n'), /parent is not a regular directory/);
  });
});

test('unknown versions, duplicate mappings and paths escaping the archive fail', () => {
  fixture(({root, manifest, save}) => {
    manifest.schema_version = 2; save();
    assert.match(checkRlRetirement({root}).issues.join('\n'), /invalid retirement manifest/);
    manifest.schema_version = 1;
    manifest.entries.push({...manifest.entries[0]}); save();
    assert.match(checkRlRetirement({root}).issues.join('\n'), /duplicate archived source/);
    manifest.entries.pop();
    manifest.entries[0].archive_path = `${ARCHIVE}/../../../outside.py`; save();
    assert.match(checkRlRetirement({root}).issues.join('\n'), /invalid relative source path/);
  });
});

test('the optional worktree inventory detects source drift and missing files', () => {
  fixture(({root}) => {
    const researchWorktree = join(root, 'research');
    mkdirSync(join(researchWorktree, 'studies/rl_example'), {recursive: true});
    const path = join(researchWorktree, 'studies/rl_example/reader.py');
    writeFileSync(path, 'reader\n');
    const valid = checkRlRetirement({root, researchWorktree});
    assert.deepEqual(valid.issues, []);
    assert.equal(valid.researchFilesVerified, 1);
    writeFileSync(path, 'changed');
    assert.match(checkRlRetirement({root, researchWorktree}).issues.join('\n'), /source bytes differ/);
    rmSync(path);
    assert.match(checkRlRetirement({root, researchWorktree}).issues.join('\n'), /ENOENT/);
  });
});

test('a changed research inventory fails its independent byte binding', () => {
  fixture(({root}) => {
    writeFileSync(join(root, ARCHIVE, 'research-source-inventory.json'), '{}');
    assert.match(checkRlRetirement({root}).issues.join('\n'), /inventory byte identity differs/);
  });
});

test('the actual retired package retains all original files and has no active old entrypoints', () => {
  const result = checkRlRetirement();
  assert.deepEqual(result.issues, []);
  assert.equal(result.archivedFiles, 21);
  assert.equal(result.researchFiles, 747);
  assert.equal(result.researchFilesVerified, 0);
});
