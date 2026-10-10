import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const script = readFileSync(new URL('../pd-report/src/planning_cycles.js', import.meta.url), 'utf8');
const state = (t, x, vx) => ({simTimeS: t, positionM: {x, y: 100}, velocityMps: {x: vx, y: 20}, attitudeRad: 0, fuelKg: 100});
const cycle = (index, decision, x) => ({index, decision, current: state(index * 10, x, index * 20),
  nominal: [{xM: x, yM: 100, timeS: index * 10}], nominalExtension: [],
  ballistic: [{xM: x + 10, yM: 120, timeS: index * 10 + 1}], executed: [],
  conflict: index ? state(40, 1150, 10) : null, summary: `${decision} summary`,
  boundaryCounts: {}, nominalRejections: {}, nominalIdentity: `nominal-${index}`, queries: []});

function harness({plotReady = true, feedback = false, recovery = false, ridge = false, early = false, coast = false, landing = false} = {}) {
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {value: '', checked: true, textContent: '', children: [], listeners: {},
      append(v) { this.children.push(v); }, replaceChildren() { this.children = []; },
      querySelector(selector) { return element(`${id}:${selector}`); },
      addEventListener(type, fn) { this.listeners[type] = fn; }});
    return elements.get(id);
  };
  const payload = {cycles: [cycle(0, 'local_cleared', 0), cycle(1, 'no_clearing', 323)], provenance: 'source-bound'};
  if (feedback) {
    payload.provenance = 'Ballistic feedback candidate · source replay';
    payload.cycles.forEach((c, i) => {
      c.activeGoal = {x: 700 + i, y: 300}; c.goalLabel = i ? 'Waypoint 1' : 'Destination';
      c.decision = i ? 'waypoint_replaced' : 'ballistic_obstruction';
      if (i) c.previousGoal = {x: 900, y: 350};
      if (i) c.cutoffCoast = [{xM: 500, yM: 250, timeS: 12}, {xM: 701, yM: 300, timeS: 16}];
    });
  }
  if (recovery) {
    payload.cycles[0].decision = 'terrain_recovery_started';
    payload.cycles[1].decision = 'terrain_recovery_resumed';
  }
  if (ridge) payload.cycles[1].clearingCrest = {x: 180, y: 508};
  if (coast) {
    payload.cycles[0].decision = 'coast_through_selected';
    payload.cycles[1].decision = 'coast_terminal_entry';
    payload.cycles.forEach(c => { c.goalLabel = 'Destination'; });
  }
  if (landing) {
    payload.cycles[0].decision = 'maintained_landing_entry';
    payload.cycles[1].decision = 'short_command_obstruction';
    payload.cycles.forEach(c => { c.goalLabel = 'Destination'; });
  }
  if (early) {
    payload.cycles[1].decision = 'destination_reacquired_before_waypoint';
    payload.cycles[1].goalLabel = 'Destination';
    payload.cycles[1].summary = 'Previous waypoint superseded, not an actual H';
  }
  element('planning-cycle-data').textContent = JSON.stringify(payload);
  element('planning-cycle-query-visible').checked = false;
  const original = [{name: 'terrain', x: [-160,1360]}, {name: 'executed', x: [0,323]}];
  const plot = element('chart-spatial');
  if (plotReady) { plot.data = structuredClone(original); plot._fullLayout = {}; }
  const timeouts = [];
  const Plotly = {
    async addTraces(p, traces) { p.data.push(...traces); },
    async restyle(p, values, indices) {
      indices.forEach((index, i) => Object.keys(values).forEach(key => { p.data[index][key] = values[key][i]; }));
    },
  };
  vm.runInNewContext(script, {document: {getElementById: element, createElement: () => ({})}, Plotly,
    setTimeout: fn => { timeouts.push(fn); }});
  return {element, plot, payload, original, timeouts};
}

test('coast selection and terminal entry are navigable without inventing waypoint handoffs', async () => {
  const h = harness({feedback: true, coast: true});
  await new Promise(resolve => setImmediate(resolve));
  const nav = h.element('planning-decision-select');
  assert(nav.children.some(o => /coast_through_selected/.test(o.textContent)));
  assert(nav.children.some(o => /coast_terminal_entry/.test(o.textContent)));
  nav.value = '0';
  nav.listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.element('planning-cycle-select').value, '0');
  assert.match(h.element('planning-cycle-select').children[0].textContent, /Refresh 0/);
  assert.equal(h.plot.data[8].name, 'Destination');
  assert.deepEqual(h.plot.data.slice(0,2), h.original);
});

test('ordinary ballistic terminal ownership is reachable through decision navigation', async () => {
  const h = harness({feedback: true, landing: true});
  await new Promise(resolve => setImmediate(resolve));
  const nav = h.element('planning-decision-select');
  const entry = nav.children.find(o => /maintained_landing_entry/.test(o.textContent));
  assert(entry, 'landing entry must appear alongside obstruction decisions');
  nav.value = String(entry.value);
  nav.listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.element('planning-cycle-select').value, '0');
  assert.match(h.element('planning-cycle-select').children[0].textContent, /Refresh 0/);
  assert.equal(h.plot.data[8].name, 'Destination');
  assert.deepEqual(h.plot.data.slice(0, 2), h.original);
});

test('last planning stop selected; selecting launch changes actual nominal and conflict pixels data', async () => {
  const h = harness();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.element('planning-cycle-select').value, '1');
  assert.equal(h.plot.data[2].x[0], 323);
  assert.equal(h.plot.data[7].x[0], 1150);
  assert.match(h.element('planning-cycle-state').textContent, /velocity \(20.00, 20.00\)/);
  h.element('planning-cycle-select').value = '0';
  h.element('planning-cycle-select').listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.plot.data[2].x[0], 0);
  assert.equal(h.plot.data[7].x.length, 0);
  assert.deepEqual(h.plot.data.slice(0,2), h.original);
  assert.equal(h.payload.cycles[1].nominal[0].xM, 323);
});

test('feedback refreshes show active goal without replacing rich traces or calling refreshes handoffs', async () => {
  const h = harness({feedback: true});
  await new Promise(resolve => setImmediate(resolve));
  assert.match(h.element('planning-cycle-select').children[1].textContent, /Refresh 1/);
  assert.match(h.element('planning-cycle-review:h2').textContent, /correction refreshes/);
  assert.equal(h.plot.data[8].name, 'Waypoint 1');
  assert.deepEqual(Array.from(h.plot.data[8].x), [701]);
  assert.deepEqual(Array.from(h.plot.data[9].x), [900]);
  h.element('planning-cycle-select').value = '0';
  h.element('planning-cycle-select').listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.plot.data[8].name, 'Destination');
  assert.deepEqual(Array.from(h.plot.data[8].x), [700]);
  assert.deepEqual(Array.from(h.plot.data[9].x), []);
  assert.deepEqual(h.plot.data.slice(0,2), h.original);
});

test('feedback decision navigation jumps to selection and replacement without altering rich traces', async () => {
  const h = harness({feedback: true});
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.element('planning-decision-nav').hidden, false);
  assert.equal(h.element('planning-decision-select').children.length, 3);
  assert.match(h.element('planning-decision-select').children[2].textContent, /waypoint_replaced/);
  h.element('planning-decision-select').value = '0';
  h.element('planning-decision-select').listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.element('planning-cycle-select').value, '0');
  assert.deepEqual(h.plot.data.slice(0,2), h.original);
});

test('cutoff coast is distinct from ideal aim and disappears when a refresh has no estimate', async () => {
  const h = harness({feedback: true});
  await new Promise(resolve => setImmediate(resolve));
  const trace = h.plot.data.find(t => t.name === 'Predicted cutoff coast · not flown');
  assert.deepEqual(Array.from(trace.x), [500, 701]);
  assert.deepEqual(Array.from(trace.customdata), [12, 16]);
  h.element('planning-cycle-select').value = '0';
  h.element('planning-cycle-select').listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(Array.from(trace.x), []);
  assert.deepEqual(h.plot.data.slice(0, 2), h.original);
});

test('early destination reacquisition is navigable without relabelling it as H', async () => {
  const h = harness({feedback: true, early: true});
  await new Promise(resolve => setImmediate(resolve));
  assert.match(h.element('planning-decision-select').children[2].textContent, /destination_reacquired_before_waypoint/);
  assert.match(h.element('planning-cycle-select').children[1].textContent, /Refresh 1/);
  assert.equal(h.plot.data[8].name, 'Destination');
  assert.deepEqual(Array.from(h.plot.data[9].x), [900]);
  assert.deepEqual(h.plot.data.slice(0, 2), h.original);
});

test('blocking crest is a separate terrain marker, not a waypoint or actual handoff', async () => {
  const h = harness({feedback: true, ridge: true});
  await new Promise(resolve => setImmediate(resolve));
  assert.match(h.plot.data[10].name, /terrain feature, not a handoff/);
  assert.deepEqual(Array.from(h.plot.data[10].x), [180]);
  assert.deepEqual(Array.from(h.plot.data[10].y), [508]);
  h.element('planning-cycle-select').value = '0';
  h.element('planning-cycle-select').listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(Array.from(h.plot.data[10].x), []);
  assert.deepEqual(h.plot.data.slice(0,2), h.original);
});

test('overlay toggle and common presets leave original traces untouched', async () => {
  const h = harness();
  await new Promise(resolve => setImmediate(resolve));
  h.element('planning-cycle-visible').checked = false;
  h.element('planning-cycle-visible').listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert(h.plot.data.slice(2).every(t => !t.visible));
  h.element('planning-cycle-visible').checked = true;
  h.element('spatial-mode-toolbar').listeners.click();
  await h.timeouts.shift()();
  await new Promise(resolve => setImmediate(resolve));
  assert(h.plot.data.slice(2,8).every(t => t.visible));
  assert(h.plot.data.slice(8).every(t => !t.visible));
  h.element('planning-cycle-query-visible').checked = true;
  h.element('planning-cycle-query-visible').listeners.change();
  await new Promise(resolve => setImmediate(resolve));
  assert(h.plot.data.slice(8).every(t => t.visible));
  assert.deepEqual(h.plot.data.slice(0,2), h.original);
});

test('terrain recovery decisions remain reachable without being called waypoint handoffs', async () => {
  const h = harness({feedback: true, recovery: true});
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.element('planning-decision-select').children.length, 3);
  assert.match(h.element('planning-decision-select').children[1].textContent, /terrain_recovery_started/);
  assert.match(h.element('planning-decision-select').children[2].textContent, /terrain_recovery_resumed/);
  assert.deepEqual(h.plot.data.slice(0,2), h.original);
});

test('missing Plotly readiness has bounded retry and retains explanatory data', async () => {
  const h = harness({plotReady: false});
  while (h.timeouts.length) await h.timeouts.shift()();
  assert.match(h.element('planning-cycle-plot-status').textContent, /require the existing Plotly chart/);
  assert.match(h.element('planning-cycle-summary').textContent, /no_clearing summary/);
});
