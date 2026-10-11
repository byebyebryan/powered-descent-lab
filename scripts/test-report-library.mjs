import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('../pd-report/src/report_navigation.rs', import.meta.url), 'utf8');
const script = source.match(/const LIBRARY_JS: &str = r#"([\s\S]*?)"#;/)[1];

function harness(query = '') {
  const controls = Object.fromEntries([
    ['report-search', ''], ['topic-filter', 'all'], ['type-filter', 'all'],
    ['availability-filter', 'available'], ['order-filter', 'newest'],
    ['report-count', ''], ['report-empty', ''],
  ].map(([id, value]) => [id, { value, events: {}, addEventListener(name, fn) { this.events[name] = fn; } }]));
  controls['topic-filter'].options = [{ value: 'all' }, { value: 'waypoint-planning' }, { value: 'flight-control' }];
  const rows = [
    ['new', 'Zulu recovery', 'waypoint-planning', 'experimental-report', true, 300],
    ['old', 'Alpha terminal', 'flight-control', 'batch-report', true, 100],
    ['missing', 'Missing pack', 'waypoint-planning', 'batch-report', false, 0],
  ].map(([id, name, topic, type, available, updated]) => ({
    id, hidden: false,
    dataset: { name: name.toLowerCase(), search: name.toLowerCase(), topic, type,
      available: String(available), updated: String(updated) },
  }));
  const order = [...rows];
  const container = { appendChild(row) { order.splice(order.indexOf(row), 1); order.push(row); } };
  vm.runInNewContext(script, {
    URLSearchParams, location: { search: query },
    document: {
      querySelectorAll: () => rows,
      querySelector: (selector) => selector.includes('.report-rows') ? container : controls[selector.slice(1)],
    },
  });
  return {
    rows, order, controls,
    change(id, value, event = 'change') { controls[id].value = value; controls[id].events[event](); },
    visible() { return order.filter((row) => !row.hidden).map((row) => row.id); },
  };
}

test('saved reports are visible by default; missing fixture entries remain optional', () => {
  const h = harness();
  assert.deepEqual(h.visible(), ['new', 'old']);
  assert.equal(h.controls['report-count'].textContent, '2 reports');
  h.change('availability-filter', 'all');
  assert.deepEqual(h.visible(), ['new', 'old', 'missing']);
});

test('topic links, text search, type and empty-state filters compose', () => {
  const h = harness('?topic=waypoint-planning');
  assert.deepEqual(h.visible(), ['new']);
  h.change('report-search', 'unmatched', 'input');
  assert.deepEqual(h.visible(), []);
  assert.equal(h.controls['report-empty'].hidden, false);
  h.change('report-search', 'recovery', 'input');
  assert.deepEqual(h.visible(), ['new']);
  h.change('type-filter', 'batch-report');
  assert.deepEqual(h.visible(), []);
});

test('name order and newest order can be switched without losing active filters', () => {
  const h = harness();
  h.change('order-filter', 'name');
  assert.deepEqual(h.visible(), ['old', 'new']);
  h.change('order-filter', 'newest');
  assert.deepEqual(h.visible(), ['new', 'old']);
  h.change('topic-filter', 'flight-control');
  h.change('order-filter', 'name');
  assert.deepEqual(h.visible(), ['old']);
});
