// Browser-side checks shared by maintained Planner V2 integration scripts.
import {createHash} from 'node:crypto';
import {readFileSync, statSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {assertTypedEqual} from './planner-v2-report-checks.mjs';

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function fail(message) {
  throw new Error(message);
}

function check(condition, message) {
  if (!condition) fail(message);
}

export function cdpClient(ws) {
  let sequence = 0;
  const pending = new Map();
  const errors = [];
  const optional = [];
  ws.addEventListener('message', event => {
    let message;
    try { message = JSON.parse(event.data); } catch (error) { errors.push({kind: 'malformed-cdp-message', message: String(error)}); return; }
    if (message.id) {
      const item = pending.get(message.id);
      if (!item) return;
      clearTimeout(item.timer);
      pending.delete(message.id);
      if (message.error) item.reject(new Error(`${item.method}: ${message.error.message}`));
      else item.resolve(message.result ?? {});
    } else if (message.method === 'Runtime.exceptionThrown') {
      errors.push({kind: 'javascript-exception', detail: message.params.exceptionDetails});
    } else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
      const entry = message.params.entry;
      if (entry.source === 'network' && /\/favicon\.ico(?:\?|$)/.test(entry.url ?? '') && /\b404\b/.test(entry.text ?? '')) optional.push(entry);
      else errors.push({kind: 'console-error', entry});
    } else if (message.method === 'Network.responseReceived' && message.params.response.status >= 400) {
      const response = message.params.response;
      if (/\/favicon\.ico(?:\?|$)/.test(response.url)) optional.push({source: 'network', url: response.url, status: response.status});
      else errors.push({kind: 'http-resource-error', url: response.url, status: response.status, resourceType: message.params.type});
    } else if (message.method === 'Network.loadingFailed') {
      const request = message.params;
      if (request.requestId) errors.push({kind: 'network-loading-failed', request});
    }
  });
  function send(method, params = {}) {
    const id = ++sequence;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 20000);
      pending.set(id, {resolve, reject, timer, method});
      ws.send(JSON.stringify({id, method, params}));
    });
  }
  async function evaluate(expression) {
    const response = await send('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
    check(!response.exceptionDetails, `Browser evaluation failed: ${JSON.stringify(response.exceptionDetails)}`);
    return response.result?.value;
  }
  return {send, evaluate, errors, optional};
}

export async function waitForBrowserPage(client, url, rich = false) {
  for (let attempt = 0; attempt < 300; attempt += 1) {
    const ready = await client.evaluate(`location.href === ${JSON.stringify(url)} && document.readyState === 'complete'${rich ? ' && typeof Plotly !== "undefined" && Boolean(document.getElementById("chart-spatial")?._fullLayout) && Boolean(document.getElementById("chart-metrics")?._fullLayout)' : ''}`);
    if (ready) {
      await client.evaluate('new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
      return;
    }
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  fail(`Browser page did not become ready: ${url}`);
}

export async function captureScreenshot(client, outputDir, name) {
  const {cssContentSize} = await client.send('Page.getLayoutMetrics');
  const {data} = await client.send('Page.captureScreenshot', {
    format: 'png',
    captureBeyondViewport: true,
    clip: {x: 0, y: 0, width: cssContentSize.width, height: cssContentSize.height, scale: 1},
  });
  const path = join(outputDir, name);
  writeFileSync(path, Buffer.from(data, 'base64'), {flag: 'wx'});
  return {file: name, sha256: sha256(readFileSync(path)), bytes: statSync(path).size};
}

export async function checkFlightInteractions(client, record, label) {
  const info = await client.evaluate(`(()=>({
    corrections: reportData.flightAnnotations?.corrections || [],
    modes: [...document.querySelectorAll('[data-mode]')].map(button => button.dataset.mode),
    spatial: document.getElementById('chart-spatial').data.map(trace => ({name: trace.name, x: trace.x, y: trace.y, visible: trace.visible, marker: trace.marker})),
    metricShapes: document.getElementById('chart-metrics').layout.shapes || [],
    metricAnnotations: document.getElementById('chart-metrics').layout.annotations || [],
    selectorCount: document.querySelectorAll('[data-select-correction]').length,
    toggle: Boolean(document.getElementById('flight-handoffs-visible')),
  }))()`).then(value => value);
  assertTypedEqual(info.modes, ['mission', 'guidance', 'speed', 'throttle', 'vectors'], `${label} rich mode toolbar`);
  check(info.corrections.length === record.correction_count, `${label}: visible correction count differs from accepted raw flight`);
  if (record.correction_count === 0) {
    check(!info.toggle && info.selectorCount === 0, `${label}: direct clear detail unexpectedly has handoff controls`);
    return {corrections: 0, plotly_ready: true};
  }
  check(info.toggle && info.selectorCount === record.correction_count, `${label}: correction toggle/selector controls are missing`);
  const traceIndex = info.spatial.findIndex(trace => trace.name === 'Waypoint handoffs');
  check(traceIndex >= 0, `${label}: handoff marker trace is missing from spatial plot`);
  const trace = info.spatial[traceIndex];
  check(trace.x.length === record.correction_count && trace.y.length === record.correction_count, `${label}: spatial plot is missing an H marker`);
  assertTypedEqual(trace.x, info.corrections.map(correction => correction.handoff.positionM.x), `${label} H marker spatial X coordinates`);
  assertTypedEqual(trace.y, info.corrections.map(correction => correction.handoff.positionM.y), `${label} H marker spatial Y coordinates`);
  const expectedTimes = info.corrections.map(correction => correction.handoff.simTimeS);
  for (const time of expectedTimes) {
    check(info.metricShapes.some(shape => shape.x0 === time && shape.x1 === time), `${label}: H marker ${time} is missing from metrics plot`);
  }
  for (let index = 0; index < record.correction_count; index += 1) {
    check(info.metricAnnotations.some(annotation => annotation.text === `H${index + 1}`), `${label}: H${index + 1} label is missing from metrics plot`);
  }
  const shapesBefore = await client.evaluate(`document.getElementById('chart-metrics').layout.shapes.length`);
  await client.evaluate(`document.getElementById('flight-handoffs-visible').click()`);
  check(await client.evaluate(`document.getElementById('chart-spatial').data[${traceIndex}].visible === false`), `${label}: handoff toggle did not hide spatial markers`);
  check(await client.evaluate(`document.getElementById('chart-metrics').layout.shapes.length < ${shapesBefore}`), `${label}: handoff toggle did not hide metric guides`);
  await client.evaluate(`document.getElementById('flight-handoffs-visible').click()`);
  check(await client.evaluate(`document.getElementById('chart-spatial').data[${traceIndex}].visible === true`), `${label}: handoff toggle did not restore spatial markers`);
  check(await client.evaluate(`document.getElementById('chart-metrics').layout.shapes.length === ${shapesBefore}`), `${label}: handoff toggle did not restore metric guides`);
  const selected = record.correction_count > 1 ? record.correction_count - 1 : 0;
  await client.evaluate(`document.querySelector('[data-select-correction="${selected}"]').click()`);
  check(await client.evaluate(`document.querySelector('[data-select-correction="${selected}"]').getAttribute('aria-pressed') === 'true'`), `${label}: selected correction is not announced`);
  if (record.correction_count > 1) {
    const sizes = await client.evaluate(`document.getElementById('chart-spatial').data[${traceIndex}].marker.size`);
    check(sizes[selected] > sizes[0], `${label}: selected H marker is not emphasized in spatial plot`);
    check(await client.evaluate(`document.getElementById('chart-metrics').layout.annotations.some(a => a.text === 'H${selected + 1}')`), `${label}: selected H marker is not emphasized in metrics plot`);
  }
  return {corrections: record.correction_count, plotly_ready: true, spatial_handoffs: trace.x.length, metric_handoff_guides: expectedTimes.length, toggle_and_select_passed: true};
}
