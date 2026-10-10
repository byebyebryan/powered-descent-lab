import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';

const source = readFileSync(new URL('../pd-report/src/batch_tree.rs', import.meta.url), 'utf8');
const script = source.match(/pub const SCRIPT: &str = r#"([\s\S]*?)"#;/)[1];

function fixture() {
  const window = {};
  runInNewContext(script, {window});
  const rows = [];
  const table = {dataset: {}, querySelectorAll: selector => rows.filter(row => row.matches(selector))};
  const add = (group, parent, depth, leaf = false) => {
    const attributes = {'aria-expanded': 'false'};
    const row = {
      dataset: {...(leaf ? {caseId: group} : {group}), ...(parent ? {parent} : {}), depth: String(depth)},
      hidden: Boolean(parent),
      matches: selector => selector.split(',').some(s => s.trim() === (leaf ? 'tr.seed-row' : 'tr.summary-row')),
      closest: selector => selector === 'table' ? table : null,
      setAttribute: (name, value) => { attributes[name] = value; },
      getAttribute: name => attributes[name],
      addEventListener() {},
    };
    rows.push(row);
    return row;
  };
  add('stops', null, 0);
  const launch = add('stops-not-launched', 'stops', 1);
  add('launch-recipe-a', 'stops-not-launched', 2);
  add('327', 'launch-recipe-a', 3, true);
  add('launch-recipe-b', 'stops-not-launched', 2);
  add('791', 'launch-recipe-b', 3, true);
  const clearing = add('stops-clearing', 'stops', 1);
  add('clearing-recipe', 'stops-clearing', 2);
  add('280', 'clearing-recipe', 3, true);
  add('landings', null, 0);
  add('direct', 'landings', 1);
  add('000', 'direct', 2, true);
  add('sentinel', null, 0);
  add('control', 'sentinel', 1, true);
  const root = {
    querySelectorAll: selector => selector === 'table[data-tree-table]' ? [table] : table.querySelectorAll(selector),
    ownerDocument: {querySelectorAll: () => []},
  };
  const api = window.PdBatchTree.init(root, {
    maxDepth: 3, defaultExpansion: 1,
    page: {}, controlScope: {querySelectorAll: () => []},
  });
  const visible = () => rows.filter(row => row.dataset.caseId && !row.hidden).map(row => row.dataset.caseId);
  return {api, rows, launch, clearing, visible};
}

test('failure-group focus opens all its recipes and no successful/control missions', () => {
  const {api, launch, visible} = fixture();
  assert.equal(api.focusGroup('stops-not-launched'), launch);
  assert.deepEqual(visible(), ['327', '791']);
});

test('switching groups or returning to the same group restores only that subset', () => {
  const {api, clearing, visible} = fixture();
  api.focusGroup('stops-not-launched');
  assert.equal(api.focusGroup('stops-clearing'), clearing);
  assert.deepEqual(visible(), ['280']);
  api.collapseGroups(0);
  api.focusGroup('stops-clearing');
  assert.deepEqual(visible(), ['280']);
});

test('all stopped missions remains distinct from all missions; stale targets do not mutate', () => {
  const {api, rows, visible} = fixture();
  api.focusGroup('stops');
  assert.deepEqual(visible(), ['327', '791', '280']);
  const before = rows.map(row => [row.hidden, row.getAttribute('aria-expanded')]);
  assert.equal(api.focusGroup('missing-group'), null);
  assert.deepEqual(rows.map(row => [row.hidden, row.getAttribute('aria-expanded')]), before);
  api.expandLeaves();
  assert.deepEqual(visible(), ['327', '791', '280', '000', 'control']);
});
