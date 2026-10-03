#!/usr/bin/env node
// Presentation-only browser gate. Connect to an isolated, caller-owned Chrome
// debugging session; no browser/package installation or mission execution.
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';

const args = new Map();
for (let i = 2; i < process.argv.length; i += 2) args.set(process.argv[i], process.argv[i + 1]);
const base = args.get('--url');
const out = args.get('--output-dir');
assert(base && out, 'Usage: node scripts/check-waypoint-v2-report.mjs --url SUITE_URL --output-dir FRESH_DIR [--cdp-url http://127.0.0.1:9224] [--cases id,id]');
const cdp = args.get('--cdp-url') || 'http://127.0.0.1:9224';
assert(new URL(cdp).hostname === '127.0.0.1', 'CDP must be a local isolated Chrome session');
const receiptResponse = await fetch(new URL('render-provenance.json', base));
assert(receiptResponse.ok, 'Finish the create-only render command before starting browser checks');
const renderReceipt = await receiptResponse.json();
assert.equal(renderReceipt.schema_id, 'waypoint_v2_report_render_v1', 'Missing completed rendering receipt');
await fs.mkdir(out); // deliberately create-only
const tabs = await (await fetch(`${cdp}/json/list`)).json();
const tab = tabs.find(t => t.type === 'page');
assert(tab, 'No page in caller-owned Chrome session');
const ws = new WebSocket(tab.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
let seq = 0;
const pending = new Map();
const browserErrors = [];
ws.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  if (message.id) {
    const p = pending.get(message.id);
    if (p) { clearTimeout(p.timer); pending.delete(message.id); message.error ? p.reject(message.error) : p.resolve(message.result); }
  } else if (message.method === 'Runtime.exceptionThrown') browserErrors.push(message.params);
  else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') browserErrors.push(message.params.entry);
};
function send(method, params = {}) {
  const id = ++seq;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timed out: ${method}`)); }, 15000);
    pending.set(id, { resolve, reject, timer });
    ws.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  assert(!r.exceptionDetails, JSON.stringify(r.exceptionDetails));
  return r.result.value;
}
async function screenshot(name) {
  const { cssContentSize } = await send('Page.getLayoutMetrics');
  const { data } = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip: { x: 0, y: 0, width: cssContentSize.width, height: cssContentSize.height, scale: 1 } });
  await fs.writeFile(path.join(out, name), Buffer.from(data, 'base64'), { flag: 'wx' });
}
await send('Page.enable');
await send('Runtime.enable');
await send('Log.enable');
const cases = (args.get('--cases') || 'v2_ridge_late').split(',');
const checks = [];
try {
  for (const width of [1280, 390]) {
    await send('Emulation.setDeviceMetricsOverride', { width, height: width === 390 ? 844 : 900, deviceScaleFactor: 1, mobile: false });
    for (const id of ['index', ...cases]) {
      const url = id === 'index' ? new URL('index.html', base).href : new URL(`cases/${id}/index.html`, base).href;
      await send('Page.navigate', { url });
      // Navigation response does not imply document readiness. Poll only this
      // local browser, with a finite bound, then wait two paint frames.
      let ready = false;
      for (let i = 0; i < 100; i++) {
        ready = await evaluate(`location.href === ${JSON.stringify(url)} && document.readyState === 'complete'`);
        if (ready) break;
        await new Promise(r => setTimeout(r, 50));
      }
      assert(ready, `Page failed to load: ${url}`);
      await evaluate('new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))');
      const initial = await evaluate(`({title:document.querySelector('h1')?.innerText, width:innerWidth, scrollWidth:document.documentElement.scrollWidth,
        svgCount:document.querySelectorAll('svg').length, segmentCount:document.querySelectorAll('.flight-path[data-flight-segment]').length,
        handoffCount:document.querySelectorAll('.handoff-marker[data-handoff-segment]').length,
        handoffs:[...document.querySelectorAll('.handoff-marker')].map(g=>({step:Number(g.dataset.step),time:Number(g.dataset.time),x:Number(g.dataset.xM),y:Number(g.dataset.yM)})),
        landingCount:document.querySelectorAll('.landing-marker').length,
        buttons:[...document.querySelectorAll('.piece-button')].map(b=>b.dataset.selectSegment),
        openDetails:document.querySelectorAll('details[open]').length,
        externalAssets:[...document.querySelectorAll('script[src],link[rel="stylesheet"]')].map(e=>e.src||e.href),
        linkCount:document.querySelectorAll('a[href]').length})`);
      assert(initial.title, `Missing title: ${id}`);
      assert(initial.scrollWidth <= width + 1, `Horizontal overflow: ${id}/${width}: ${initial.scrollWidth}`);
      assert.equal(initial.openDetails, 0, `Diagnostics start expanded: ${id}`);
      assert.equal(initial.externalAssets.length, 0, `External asset dependency: ${id}`);
      if (id !== 'index') assert.equal(initial.svgCount, 1, `Missing trajectory diagram: ${id}`);
      const expected = {
        v2_clear_845: [1, [], 1], v2_clear_uphill_845: [1, [], 1], v2_clear_downhill_845: [1, [], 1],
        v2_ridge_late: [3, [2426], 1], v2_successive_rising: [5, [2006, 2774], 1],
        v2_plateau_reference_900: [7, [2820, 3136, 3226], 1], v2_diag_near_target: [0, [], 0],
      }[id];
      if (expected) {
        assert.equal(initial.segmentCount, expected[0], `Executed segment count: ${id}`);
        assert.deepEqual(initial.handoffs.map(h=>h.step), expected[1], `Wrong actual H boundary: ${id}`);
        assert(initial.handoffs.every(h=>Math.abs(h.time-h.step/120) < 1e-8), 'H clock was moved or rounded');
        assert.equal(initial.landingCount, expected[2], `Landing symbol inconsistent: ${id}`);
      }
      await screenshot(`${id}-${width}-initial.png`);
      for (const segment of initial.buttons) {
        await evaluate(`document.querySelector('[data-select-segment="${segment}"]').click()`);
        const selected = await evaluate(`({pressed:document.querySelector('[data-select-segment="${segment}"]').getAttribute('aria-pressed'),
          visible:!document.querySelector('#segment-detail-${segment}').hidden,
          paths:document.querySelectorAll('.flight-path[data-flight-segment]').length})`);
        assert.equal(selected.pressed, 'true', `Selection did not activate: ${id}/${segment}`);
        assert(selected.visible, `Selected details remain hidden: ${id}/${segment}`);
        assert.equal(selected.paths, initial.segmentCount, 'Selection discarded other actual flight pieces');
        const markersCorrect = await evaluate(`[...document.querySelectorAll('[data-selected-segment]')].every(g => (getComputedStyle(g).display !== 'none') === (g.dataset.selectedSegment === '${segment}'))`);
        assert(markersCorrect, `SVG entry/conflict markers did not switch: ${id}/${segment}`);
      }
      if (initial.buttons.length) {
        await evaluate(`document.querySelector('[data-select-segment="${initial.buttons[0]}"]').focus()`);
        await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r', unmodifiedText: '\r' });
        await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
        assert.equal(await evaluate(`document.activeElement.getAttribute('aria-pressed')`), 'true', 'Keyboard selection failed');
      }
      const hasDetails = await evaluate(`Boolean(document.querySelector('details'))`);
      if (hasDetails) {
        await evaluate(`document.querySelector('details summary').click()`);
        assert(await evaluate(`document.querySelector('details').open`), 'Disclosure did not open');
      }
      const links = await evaluate(`[...document.querySelectorAll('a[href]')].map(a=>a.href)`);
      for (const link of new Set(links)) {
        const response = await fetch(link);
        assert(response.ok, `Broken link ${response.status}: ${link}`);
      }
      checks.push({ id, width, url, ...initial, selectionAndDisclosurePassed: true, linksPassed: true });
    }
  }
  assert.equal(browserErrors.length, 0, 'Browser JS exceptions');
  await fs.writeFile(path.join(out, 'browser-checks.json'), JSON.stringify({ schema: 'waypoint_v2_browser_checks_v1', browser: await evaluate('navigator.userAgent'), renderer:renderReceipt.renderer_version, sourceRoot:renderReceipt.source_root, checks, browserErrors, evidenceScope: 'Real local headless browser layout and interaction checks, not a new physical flight or user readability acceptance.' }, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ checks: checks.length, screenshots: checks.length, output: out, status: 'passed' }));
} finally { ws.close(); }
