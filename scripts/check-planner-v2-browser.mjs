#!/usr/bin/env node
// Optional browser acceptance against an already running report server and
// an explicitly supplied disposable local browser. Does not run missions,
// refresh reports, or select evidence. Screenshots are create-only.
import assert from 'node:assert/strict';
import {mkdirSync, writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
import {cdpClient, waitForBrowserPage, captureScreenshot, checkFlightInteractions} from './check-planner-v2-batch.mjs';

const BASE = '/reports/eval/planner_v2_lab_suite/';
const TOPIC = '/reports/topics/waypoint-planning/index.html';

export function validateBrowserEndpoints(rootUrl, cdpUrl) {
  const root = new URL(rootUrl);
  const cdp = new URL(cdpUrl);
  assert.equal(root.protocol, 'http:');
  assert(['127.0.0.1', '192.168.1.110', 'localhost'].includes(root.hostname), 'report server must be the declared local/LAN host');
  assert.equal(cdp.protocol, 'http:');
  assert(['127.0.0.1', 'localhost'].includes(cdp.hostname), 'browser debugging must be local');
  assert(!root.username && !root.password && !cdp.username && !cdp.password);
  return {root, cdp};
}

export function resolveReportPath(root, path) {
  assert(typeof path === 'string' && path.startsWith('/') && !path.startsWith('//'), 'report path must be single-slash relative to the server origin');
  assert(!/[\\?#]/.test(path), 'unsafe report path');
  for (const [index, piece] of path.split('/').entries()) {
    if (!piece && (index === 0 || index === path.split('/').length - 1)) continue;
    const decoded = decodeURIComponent(piece);
    assert(decoded && decoded !== '.' && decoded !== '..' && !/[\\/]/.test(decoded), 'unsafe encoded report path');
  }
  const url = new URL(path, root);
  assert.equal(url.origin, root.origin, 'report URL escaped the declared server');
  return url;
}

async function readJson(url) {
  const response = await fetch(url, {signal: AbortSignal.timeout(20000), redirect: 'error'});
  assert(response.ok, `HTTP ${response.status}: ${url}`);
  assert(response.headers.get('content-type')?.includes('json'), `not JSON: ${url}`);
  return response.json();
}

export async function checkBrowser({rootUrl, cdpUrl, outputDir}) {
  const {root, cdp} = validateBrowserEndpoints(rootUrl, cdpUrl);
  const receipt = await readJson(resolveReportPath(root, BASE + 'render.json'));
  resolveReportPath(root, receipt.source_base_href);
  assert(receipt.source_base_href.endsWith('/'), 'source base must end in a slash');
  const batch = await readJson(resolveReportPath(root, receipt.source_base_href + 'summary.json'));
  assert.equal(batch.pack_id, 'planner_v2_lab_suite');
  assert.equal(batch.policy_version, 3);
  assert.equal(batch.case_count, 44);
  const direct = batch.cases.find(c => c.group === 'clear' && c.correction_count === 0);
  const corrected = batch.cases.filter(c => c.status === 'simulated' && c.correction_count >= 1)
    .sort((a, b) => a.correction_count - b.correction_count);
  const one = corrected[0];
  const multiple = corrected.at(-1);
  const stopped = batch.cases.find(c => c.group === 'diagnostic' && c.status === 'simulated' && c.planning_stop !== 'landed');
  const unsupported = batch.cases.find(c => c.planning_stop === 'unsupported');
  assert(direct && one && unsupported, 'required direct/corrected/unsupported inspection examples missing');
  const examples = [['direct', direct], ['corrected', one]];
  if (multiple.case_id !== one.case_id) examples.push(['most-handoffs', multiple]);
  if (stopped) examples.push(['finite-stop', stopped]); // Recovery is not a regression.
  examples.push(['unsupported', unsupported]);
  const tabResponse = await fetch(new URL('/json/new?about:blank', cdp), {method: 'PUT', signal: AbortSignal.timeout(20000)});
  assert(tabResponse.ok, 'cannot create a dedicated browser-check tab');
  const tab = await tabResponse.json();
  assert(tab.id && tab.webSocketDebuggerUrl);
  let ws;
  let client;
  const checks = [];
  const screenshots = [];
  const navigate = async (path, rich = false) => {
    const url = resolveReportPath(root, path).href;
    await client.send('Page.navigate', {url});
    await waitForBrowserPage(client, url, rich);
  };
  const clickLink = async path => {
    const url = resolveReportPath(root, path).href;
    assert(await client.evaluate(`(()=>{const a=[...document.querySelectorAll('a')].find(a=>a.href===${JSON.stringify(url)});if(!a)return false;a.click();return true;})()`), `missing reader link: ${path}`);
    await waitForBrowserPage(client, url);
  };
  const width = () => client.evaluate('({width:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth})');
  try {
    const socketUrl = new URL(tab.webSocketDebuggerUrl);
    assert.equal(socketUrl.protocol, 'ws:');
    assert(['127.0.0.1', 'localhost'].includes(socketUrl.hostname), 'browser socket escaped local debugging');
    assert.equal(socketUrl.port, cdp.port, 'browser socket escaped the declared debugging port');
    ws = new WebSocket(socketUrl);
    await new Promise((ok, bad) => {
      const timer = setTimeout(() => bad(new Error('browser socket handshake timed out')), 20000);
      ws.addEventListener('open', () => { clearTimeout(timer); ok(); }, {once: true});
      ws.addEventListener('error', error => { clearTimeout(timer); bad(error); }, {once: true});
    });
    client = cdpClient(ws);
    for (const domain of ['Runtime', 'Page', 'Log', 'Network']) await client.send(`${domain}.enable`);
    for (const size of [
      {name: 'desktop', width: 1440, height: 1000, mobile: false},
      {name: 'mobile', width: 390, height: 844, mobile: true},
    ]) {
      await client.send('Emulation.setDeviceMetricsOverride', {
        width: size.width, height: size.height, deviceScaleFactor: 1, mobile: size.mobile,
      });
      for (const path of ['/', '/reports/']) {
        await navigate(path);
        await clickLink(BASE + 'index.html');
        assert(await client.evaluate('!!document.querySelector("[data-batch-template=common-v1]")'));
      }
      await navigate('/');
      await clickLink(TOPIC);
      await clickLink(BASE + 'index.html');
      const verdict = await client.evaluate('JSON.parse(document.getElementById("planner-v2-acceptance").textContent)');
      assert.equal(verdict.passed, true);
      assert(await client.evaluate('document.body.innerText.includes("PASSED")'));
      const initial = await width();
      assert(initial.scroll <= initial.width + 1, `${size.name}: batch document overflow`);
      await client.evaluate('document.querySelector(".header-context").open=true;document.querySelector("#provenance").open=true');
      const expanded = await width();
      assert(expanded.scroll <= expanded.width + 1, `${size.name}: expanded context overflow`);
      await client.evaluate('document.querySelector(".header-context").open=false;document.querySelector("#provenance").open=false');
      await client.evaluate('document.querySelector("[data-tree-action=expand-seeds]").click()');
      assert.equal(await client.evaluate('[...document.querySelectorAll("tr.mission-row")].filter(r=>!r.hidden).length'), 44);
      await client.evaluate('document.querySelector("[data-tree-action=collapse-seeds]").click()');
      if (outputDir) screenshots.push(await captureScreenshot(client, outputDir, `${size.name}-batch.png`));
      for (const [label, c] of examples) {
        await navigate(BASE + c.annotated_report_path, c.status === 'simulated');
        const pageWidth = await width();
        assert(pageWidth.scroll <= pageWidth.width + 1, `${size.name}/${label}: detail document overflow`);
        if (c.status === 'simulated') await checkFlightInteractions(client, c, `${size.name}/${c.case_id}`);
        else {
          assert(await client.evaluate('!document.getElementById("chart-spatial") && document.body.innerText.includes("No simulator trajectory")'));
        }
        checks.push({viewport: size.name, kind: label, case_id: c.case_id, corrections: c.correction_count});
        if (outputDir && label === 'most-handoffs') screenshots.push(await captureScreenshot(client, outputDir, `${size.name}-multi-handoff.png`));
        await clickLink(BASE + 'index.html');
      }
    }
    assert.deepEqual(client.errors, [], 'browser report/chart/resource errors');
    return {schema_id: 'planner_v2_workflow_browser_check_v1', passed: true,
      root_url: root.href, source_capture: receipt.source_capture,
      viewports: [1440, 390], home_topic_batch_navigation: true,
      checks, screenshots, browser_errors: client.errors, optional_favicon_diagnostics: client.optional};
  } finally {
    ws?.close();
    await fetch(new URL(`/json/close/${encodeURIComponent(tab.id)}`, cdp), {signal: AbortSignal.timeout(20000)});
  }
}

async function main() {
  const {values} = parseArgs({options: {
    'root-url': {type: 'string', default: 'http://127.0.0.1:8000/'},
    'cdp-url': {type: 'string'}, 'output-dir': {type: 'string'}, help: {type: 'boolean'},
  }});
  if (values.help) {
    process.stdout.write('node scripts/check-planner-v2-browser.mjs --cdp-url http://127.0.0.1:PORT/ [--root-url http://127.0.0.1:8000/] [--output-dir NEW_DIRECTORY]\n'
      + 'Requires an existing report server and a disposable browser; opens and closes its own tab. No report writes or simulation.\n');
    return;
  }
  assert(values['cdp-url'], '--cdp-url must name an explicitly supplied disposable browser');
  const outputDir = values['output-dir'] && resolve(values['output-dir']);
  validateBrowserEndpoints(values['root-url'], values['cdp-url']);
  if (outputDir) mkdirSync(outputDir); // Fail rather than overwrite a prior receipt.
  const result = await checkBrowser({rootUrl: values['root-url'], cdpUrl: values['cdp-url'], outputDir});
  if (outputDir) writeFileSync(resolve(outputDir, 'browser-check.json'), JSON.stringify(result, null, 2) + '\n', {flag: 'wx'});
  process.stdout.write(JSON.stringify(result, null, 2) + '\n');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { await main(); } catch (error) { process.stderr.write(`${error.stack ?? error}\n`); process.exitCode = 1; }
}
