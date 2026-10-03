#!/usr/bin/env node
// Retained-data and real-browser presentation checks. No missions or stable-site writes.
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

const args = new Map();
for (let i = 2; i < process.argv.length; i += 2) args.set(process.argv[i], process.argv[i + 1]);
const base = args.get('--url');
const out = args.get('--output-dir');
const rootUrl = args.get('--root-url');
assert(base && out, 'Usage: node scripts/check-rich-waypoint-preview.mjs --url PREVIEW_ROOT_URL --output-dir FRESH_DIR [--root-url REPORT_SERVER_URL] [--cdp-url http://127.0.0.1:9225]');
if (rootUrl) assert.equal(new URL(rootUrl).origin, new URL(base).origin, 'Root and preview must share the report server');
const cdp = args.get('--cdp-url') || 'http://127.0.0.1:9225';
assert.equal(new URL(cdp).hostname, '127.0.0.1', 'Use a caller-owned isolated local Chrome');
const receiptResponse = await fetch(new URL('render-provenance.json', base));
assert(receiptResponse.ok, 'Missing completed render receipt');
const receipt = await receiptResponse.json();
const navigationEdition = receipt.schema_id === 'waypoint_v2_navigation_edition_v1';
assert(navigationEdition || receipt.schema_id === 'waypoint_v2_rich_preview_v1', 'Unexpected report edition schema');
assert.equal(receipt.case_count, 32);
assert.equal(receipt.flight_page_count, navigationEdition ? 30 : 1);
assert.equal(Object.keys(receipt.input_sha256).length, 127);
assert.equal(Object.keys(receipt.output_sha256).length, navigationEdition ? 32 : 3);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
for (const [relative, expected] of Object.entries(receipt.input_sha256)) {
  assert.equal(hash(await fs.readFile(path.join(receipt.source_root, relative))), expected, `Source drift: ${relative}`);
}
for (const [relative, expected] of Object.entries(receipt.output_sha256)) {
  assert.equal(hash(await fs.readFile(path.join(receipt.output_root, relative))), expected, `Output drift: ${relative}`);
}
const originalFile = path.join(receipt.source_root, 'runs/v2_ridge_late/report.html');
const annotatedFile = path.join(receipt.output_root, 'waypoint-v2/cases/v2_ridge_late/index.html');
const parsePayload = html => JSON.parse(html.slice(html.indexOf('const reportData = ') + 19).split(';\n')[0]);
const originalPayload = parsePayload(await fs.readFile(originalFile, 'utf8'));
const annotatedPayload = parsePayload(await fs.readFile(annotatedFile, 'utf8'));
const exactAnnotation = annotatedPayload.flightAnnotations;
delete annotatedPayload.flightAnnotations;
assert.deepEqual(annotatedPayload, originalPayload, 'Original report data changed beyond optional annotations');
assert.equal(exactAnnotation.corrections.length, 1);
assert.equal(exactAnnotation.corrections[0].handoff.physicsStep, 2426);
assert.equal(exactAnnotation.corrections[0].handoff.simTimeS, 2426 / 120);
if (navigationEdition) {
  for (const relative of Object.keys(receipt.output_sha256).filter(p => p.includes('/cases/'))) {
    const id = relative.split('/')[2];
    const copy = await fs.readFile(path.join(receipt.output_root, relative), 'utf8');
    assert(copy.includes('aria-label="Report hierarchy"'), `Missing return navigation: ${id}`);
    if (id !== 'v2_ridge_late') {
      const original = await fs.readFile(path.join(receipt.source_root, 'runs', id, 'report.html'), 'utf8');
      assert.deepEqual(parsePayload(copy), parsePayload(original), `Navigation changed report data: ${id}`);
      const begin = copy.indexOf('<nav aria-label="Report hierarchy"');
      const end = copy.indexOf('</nav>', begin) + 6;
      assert.equal(copy.slice(0, begin) + copy.slice(end), original, `Report changed beyond navigation banner: ${id}`);
    }
  }
}

await fs.mkdir(out); // create-only validation bundle
const tabs = await (await fetch(`${cdp}/json/list`)).json();
const tab = tabs.find(t => t.type === 'page');
assert(tab, 'No page in isolated Chrome');
const ws = new WebSocket(tab.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
let sequence = 0;
const pending = new Map();
const browserErrors = [];
const optionalAssetDiagnostics = [];
ws.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  if (message.id) {
    const p = pending.get(message.id);
    if (p) { clearTimeout(p.timer); pending.delete(message.id); message.error ? p.reject(message.error) : p.resolve(message.result); }
  } else if (message.method === 'Runtime.exceptionThrown') browserErrors.push(message.params);
  else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
    const entry = message.params.entry;
    // An undeclared optional favicon is not a chart/page failure. Keep this
    // narrow diagnostic visible; never suppress JS or required-asset errors.
    if (entry.source === 'network' && entry.url === new URL('/favicon.ico', base).href && /\b404\b/.test(entry.text)) optionalAssetDiagnostics.push(entry);
    else browserErrors.push(entry);
  }
};
function send(method, params = {}) {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 20000);
    pending.set(id, { resolve, reject, timer }); ws.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  assert(!r.exceptionDetails, JSON.stringify(r.exceptionDetails));
  return r.result.value;
}
async function settle() { await evaluate('new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))'); }
async function navigate(url, rich = false) {
  await send('Page.navigate', { url });
  await waitForPage(url, rich);
}
async function waitForPage(url, rich = false) {
  for (let i = 0; i < 300; i++) {
    const ready = await evaluate(`location.href === ${JSON.stringify(url)} && document.readyState === 'complete'${rich ? ' && typeof Plotly !== "undefined" && Boolean(document.getElementById("chart-spatial")?._fullLayout) && Boolean(document.getElementById("chart-metrics")?._fullLayout)' : ''}`);
    if (ready) { await settle(); return; }
    await new Promise(r => setTimeout(r, 50));
  }
  throw new Error(`Page not ready: ${url}`);
}
async function screenshot(name) {
  const { cssContentSize } = await send('Page.getLayoutMetrics');
  const { data } = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true,
    clip: { x: 0, y: 0, width: cssContentSize.width, height: cssContentSize.height, scale: 1 } });
  await fs.writeFile(path.join(out, name), Buffer.from(data, 'base64'), { flag: 'wx' });
}
async function enter() {
  await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r', unmodifiedText: '\r' });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
  await settle();
}
async function followLabel(label, rich = false) {
  const link = await evaluate(`(()=>{const matches=[...document.querySelectorAll('a[href]')].filter(a=>
    a.textContent.trim()===${JSON.stringify(label)} || [...a.querySelectorAll('h2,h3,strong')].some(h=>h.textContent.trim()===${JSON.stringify(label)}));
    if(!matches.length || new Set(matches.map(a=>a.href)).size!==1)throw new Error('Expected one unique destination for '+${JSON.stringify(label)}+', found '+matches.length);
    matches[0].focus();return {label:matches[0].textContent.trim(),href:matches[0].href}})()`);
  await enter(); await waitForPage(link.href, rich);
  assert(await evaluate('document.documentElement.scrollWidth <= innerWidth + 1'), `Overflow after following ${label}`);
  await checkPageLinks();
  return link;
}
async function checkPageLinks() {
  const links = await evaluate(`[...document.querySelectorAll('a[href]')].map(a=>a.href)`);
  for (const link of links) {
    if (checkedLinks.has(link)) continue;
    assert.equal(new URL(link).origin, new URL(base).origin, `Unexpected report-link origin: ${link}`);
    const response = await fetch(link);
    assert(response.ok, `Broken link ${response.status}: ${link}`);
    await response.body?.cancel();
    checkedLinks.add(link);
  }
}
async function drag(x, y, dx, dy) {
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  for (let i = 1; i <= 6; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x + dx * i / 6, y: y + dy * i / 6, button: 'left', buttons: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: x + dx, y: y + dy, button: 'left', clickCount: 1 });
  await settle();
}
async function hoverSample(chart, curve, sample) {
  await evaluate(`document.getElementById('${chart}').scrollIntoView({block:'center'})`); await settle();
  const xy = await evaluate(`(()=>{const p=document.getElementById('${chart}'),r=p.getBoundingClientRect(),f=p._fullLayout,t=p.data[${curve}];
    return {x:r.left+f._size.l+f.xaxis.l2p(t.x[${sample}]),y:r.top+f._size.t+f.yaxis.l2p(t.y[${sample}])}})()`);
  await send('Input.dispatchMouseEvent', {type:'mouseMoved', ...xy}); await settle();
}
async function checkSampleInspector(expectedStep) {
  const inspected = await evaluate(`(()=>{const caption=document.querySelector('#inspect-caption').textContent;
    const step=Number(caption.match(/Sample (\\d+)/)?.[1]);const s=reportData.samples.find(s=>s.physicsStep===step);
    return {caption,step,time:s?.simTimeS,valid:Boolean(s),position:s ? s.xM.toFixed(2)+' m, '+s.yM.toFixed(2)+' m' : null,
      grid:document.querySelector('#inspect-grid').textContent}})()`);
  assert(inspected.valid, 'Inspector must select a real sample');
  // Dense 10 Hz points share pixels, especially on mobile: nearest pixel may
  // select a neighboring sample. Its displayed time/state must still be exact.
  assert(Math.abs(inspected.step - expectedStep) <= 36, `Native pointer hover selected distant sample: ${inspected.caption}`);
  assert(inspected.caption.includes(`at ${inspected.time.toFixed(2)} s`));
  assert(inspected.grid.includes(inspected.position));
}
const checks = [];
const entrypointChecks = [];
const checkedLinks = new Set();
const richUrl = new URL('waypoint-v2/cases/v2_ridge_late/index.html', base).href;
try {
  await send('Page.enable'); await send('Runtime.enable'); await send('Log.enable');
  for (const width of [1280, 390]) {
    await send('Emulation.setDeviceMetricsOverride', { width, height: width === 390 ? 844 : 900, deviceScaleFactor: 1, mobile: false });
    if (rootUrl && navigationEdition) {
      for (const kind of ['root', 'reports-home']) {
        const entryUrl = new URL(kind === 'root' ? '/' : '/reports/', rootUrl).href;
        await navigate(entryUrl);
        assert.match(await evaluate('document.querySelector("h1").textContent.trim()'), /^(PD Lab )?Reports$/);
        assert(await evaluate('document.documentElement.scrollWidth <= innerWidth + 1'));
        await checkPageLinks();
        await screenshot(`hierarchy-${kind}-${width}.png`);
        const route = [entryUrl];
        route.push((await followLabel('Waypoint planning')).href);
        await screenshot(`hierarchy-planning-${kind}-${width}.png`);
        route.push((await followLabel('Waypoint planner V2')).href);
        route.push((await followLabel('Late ridge', true)).href);
        assert.equal(await evaluate('reportData.flightAnnotations.corrections[0].handoff.physicsStep'), 2426);
        route.push((await followLabel('V2 mission collection')).href);
        const flat = await evaluate(`document.querySelector('#case-v2_clear_845 a[href]').href`);
        await evaluate(`document.querySelector('#case-v2_clear_845 a[href]').focus()`); await enter(); await waitForPage(flat, true);
        assert.equal(await evaluate('Boolean(reportData.flightAnnotations)'), false);
        route.push(flat, (await followLabel('V2 mission collection')).href);
        const plateau = await evaluate(`document.querySelector('#case-v2_plateau_reference_900 a[href]').href`);
        await evaluate(`document.querySelector('#case-v2_plateau_reference_900 a[href]').focus()`); await enter(); await waitForPage(plateau, true);
        assert.equal(await evaluate('Boolean(reportData.flightAnnotations)'), false);
        route.push(plateau, (await followLabel('Waypoint planning')).href);
        route.push((await followLabel('Maintained planner baseline')).href);
        assert.match(await evaluate('document.querySelector("h1").textContent'), /Maintained planner baseline/);
        route.push((await followLabel('Reports home')).href);
        route.push((await followLabel('Flight and landing control')).href);
        route.push((await followLabel('Terminal landing')).href);
        assert.match(await evaluate('document.querySelector("h1").textContent'), /Terminal landing/);
        route.push((await followLabel('Flight and landing control')).href);
        route.push((await followLabel('Following authored waypoint routes')).href);
        assert.match(await evaluate('document.querySelector("h1").textContent'), /Following authored waypoint routes/);
        route.push((await followLabel('Reports home')).href);
        const libraryHref = (await followLabel('Browse all reports')).href;
        route.push(libraryHref);
        assert.equal(await evaluate('document.querySelector("h1").textContent.trim()'), 'Browse all reports');
        await screenshot(`hierarchy-library-${kind}-${width}.png`);
        const library = await evaluate(`({rows:document.querySelectorAll('.report-row').length,
          types:[...document.querySelector('#type-filter').options].map(o=>({value:o.value,label:o.textContent})),
          topics:[...document.querySelector('#topic-filter').options].map(o=>({value:o.value,label:o.textContent}))})`);
        assert(library.rows > 0);
        await evaluate(`{const s=document.querySelector('#report-search');s.value='terminal';s.dispatchEvent(new Event('input',{bubbles:true}));}`);
        const searchRows = await evaluate(`[...document.querySelectorAll('.report-row:not([hidden])')].map(r=>r.dataset.search)`);
        assert(searchRows.length > 0 && searchRows.every(s=>s.includes('terminal')));
        const planning = library.topics.find(o=>/waypoint planning/i.test(o.label));
        const analytical = library.types.find(o=>/analytical/i.test(o.label));
        assert(planning && analytical);
        await evaluate(`{document.querySelector('#report-search').value='';const t=document.querySelector('#topic-filter'),k=document.querySelector('#type-filter');t.value=${JSON.stringify(planning.value)};k.value=${JSON.stringify(analytical.value)};t.dispatchEvent(new Event('change',{bubbles:true}));k.dispatchEvent(new Event('change',{bubbles:true}));}`);
        const studies = await evaluate(`[...document.querySelectorAll('.report-row:not([hidden])')].map(r=>({topic:r.dataset.topic,type:r.dataset.type,href:r.querySelector('a[href]')?.href}))`);
        assert(studies.length > 0 && studies.every(r=>r.topic===planning.value && r.type===analytical.value));
        const studyHref = studies.find(r=>r.href)?.href; assert(studyHref);
        await evaluate(`[...document.querySelectorAll('.report-row:not([hidden]) a[href]')].find(a=>a.href===${JSON.stringify(studyHref)}).focus()`);
        await enter(); await waitForPage(studyHref);
        assert.match(await evaluate('document.body.innerText'), /analytical|setup.only|no simulation/i);
        route.push(studyHref);
        // Retained historical study bodies are immutable; normal browser Back
        // returns to the live library without inventing links in those bodies.
        await evaluate('history.back()'); await waitForPage(libraryHref);
        route.push((await followLabel('Reports home')).href);
        route.push((await followLabel('Research and history')).href);
        assert.match(await evaluate('document.body.innerText'), /edition|presentation|same capture/i);
        await screenshot(`hierarchy-history-${kind}-${width}.png`);
        route.push((await followLabel('Reports home')).href);
        entrypointChecks.push({kind,width,route,nativeKeyboardNavigationPassed:true,
          tasks:['direct control','one correction','repeated corrections','maintained planner baseline','terminal landing','authored waypoint tracking','library search','topic and report-type filters','analytical study','history editions']});
      }
    } else if (rootUrl) {
      const collectionUrl = new URL('waypoint-v2/index.html', base).href;
      for (const kind of ['root', 'reports-home']) {
        const entryUrl = kind === 'root' ? new URL('/', rootUrl).href : new URL('reports/', rootUrl).href;
        const firstUrl = kind === 'root' ? new URL('index.html', base).href : collectionUrl;
        await navigate(entryUrl);
        assert(await evaluate(`document.documentElement.scrollWidth <= innerWidth + 1`), `Entrypoint overflow: ${kind}/${width}`);
        assert.equal(await evaluate(`[...document.querySelectorAll('a[href]')].filter(a=>a.href===${JSON.stringify(firstUrl)}).length`), 1, 'Selected preview must have one unambiguous entry');
        await screenshot(`entrypoint-${kind}-${width}.png`);
        await evaluate(`[...document.querySelectorAll('a[href]')].find(a=>a.href===${JSON.stringify(firstUrl)}).focus()`);
        await enter(); await waitForPage(firstUrl);
        const route = [entryUrl, firstUrl];
        if (kind === 'root') {
          await evaluate(`[...document.querySelectorAll('a[href]')].find(a=>a.href===${JSON.stringify(collectionUrl)}).focus()`);
          await enter(); await waitForPage(collectionUrl); route.push(collectionUrl);
        }
        await evaluate(`[...document.querySelectorAll('a[href]')].find(a=>a.href===${JSON.stringify(richUrl)}).focus()`);
        await enter(); await waitForPage(richUrl, true); route.push(richUrl);
        assert.equal(await evaluate(`reportData.flightAnnotations.corrections[0].handoff.physicsStep`), 2426);
        entrypointChecks.push({kind,width,route,nativeKeyboardNavigationPassed:true});
      }
    }
    for (const kind of ['home', 'suite', 'rich']) {
      const url = kind === 'home' ? new URL('index.html', base).href : kind === 'suite' ? new URL('waypoint-v2/index.html', base).href : richUrl;
      await navigate(url, kind === 'rich');
      const initial = await evaluate(`({title:document.querySelector('h1')?.innerText, scrollWidth:document.documentElement.scrollWidth,
        detailsOpen:document.querySelectorAll('details[open]').length, linkCount:document.querySelectorAll('a[href]').length,
        headings:[...document.querySelectorAll('h2')].map(e=>e.textContent.trim())})`);
      assert(initial.title);
      assert(initial.scrollWidth <= width + 1, `Overflow ${kind}/${width}: ${initial.scrollWidth}`);
      assert.equal(initial.detailsOpen, 0, 'Diagnostics must start collapsed');
      await screenshot(`${kind}-${width}-initial.png`);
      if (kind === 'suite') {
        const rows = await evaluate(`[...document.querySelectorAll('[data-case-row]')].map(r=>({id:r.id,group:r.dataset.group,links:r.querySelectorAll('a').length,href:r.querySelector('a')?.href,text:r.innerText}))`);
        assert.equal(rows.length, 32);
        assert.deepEqual(rows.reduce((a,r)=>(a[r.group]=(a[r.group]||0)+1,a),{}), {clear:8,ordinary:16,diagnostic:8});
        assert.equal(rows.filter(r=>r.links===0).length, 2);
        for (const r of rows.filter(r=>r.links===0)) assert.match(r.text, /Not simulated[\s\S]*Reason:/);
        assert.equal(rows.find(r=>r.id==='case-v2_ridge_late').href, richUrl);
        assert.equal(rows.filter(r=>r.text.includes('Annotated rich preview')).length, 1);
        assert.equal(rows.filter(r=>r.text.includes('Original full report')).length, 29);
        for (const r of rows.filter(r=>r.links && r.id!=='case-v2_ridge_late')) assert(r.href.includes(navigationEdition ? '/waypoint-v2/cases/' : '/final_hardened_policy_3_a/runs/'), 'Unexpected report edition');
        await evaluate(`document.querySelector('[data-group-filter="ordinary"]').focus()`); await enter();
        assert.equal(await evaluate(`document.querySelectorAll('[data-case-row]:not([hidden])').length`), 16);
        await evaluate(`{document.querySelector('[data-group-filter="all"]').click(); const s=document.querySelector('#case-search');s.value='ridge';s.dispatchEvent(new Event('input',{bubbles:true}));}`);
        const filtered = await evaluate(`[...document.querySelectorAll('[data-case-row]:not([hidden])')].map(r=>r.id)`);
        assert(filtered.length > 0 && filtered.length < 32 && filtered.every(id=>id.includes('ridge')));
        await evaluate(`{const s=document.querySelector('#case-search');s.value='';s.dispatchEvent(new Event('input',{bubbles:true}));location.hash='case-v2_ridge_late';}`);
        assert(await evaluate(`!document.querySelector('#case-v2_ridge_late').hidden`));
        initial.searchFilterKeyboardAnchorPassed = true;
      }
      if (kind === 'rich') {
        const data = await evaluate(`({modes:[...document.querySelectorAll('[data-mode]')].map(b=>b.dataset.mode), samples:reportData.samples.length,
          spatial:document.getElementById('chart-spatial').data.map(t=>({name:t.name,x:t.x,y:t.y})), metrics:document.getElementById('chart-metrics').data.map(t=>t.name),
          annotations:reportData.flightAnnotations, guides:document.getElementById('chart-metrics').layout.shapes,
          assets:[...document.querySelectorAll('script[src]')].map(s=>s.src)})`);
        assert.deepEqual(data.modes, ['mission','guidance','speed','throttle','vectors']);
        assert.equal(data.samples, 341);
        assert.deepEqual(data.metrics, ['velocity','vx','vy','thrust','tx','ty']);
        assert.deepEqual(data.annotations, exactAnnotation);
        assert.deepEqual(data.assets, ['https://cdn.plot.ly/plotly-basic-2.35.2.min.js']);
        const hIndex = data.spatial.findIndex(t=>t.name === 'Waypoint handoffs');
        assert(hIndex >= 0);
        assert.deepEqual(data.spatial[hIndex].x, [-288.84978519456723]);
        assert.deepEqual(data.spatial[hIndex].y, [318.2854446064752]);
        assert(data.guides.some(s=>s.x0===2426/120 && s.x1===2426/120));
        for (const heading of ['Hovered Sample','What Happened','Markers And Config','Landing Quality','Flight Stats','Controller Stats','Run And Sim Performance','Mission Profile']) assert(initial.headings.includes(heading), `Missing rich section ${heading}`);
        const metricBaseShapes = data.guides.length - 1;
        for (const mode of data.modes) {
          await evaluate(`document.querySelector('[data-mode="${mode}"]').click()`); await settle();
          assert(await evaluate(`document.getElementById('chart-spatial').data[${hIndex}].visible === true`));
          if (mode==='vectors') assert(await evaluate(`document.getElementById('chart-spatial').layout.annotations.length > 0`));
        }
        await evaluate(`document.querySelector('#flight-handoffs-visible').click()`); await settle();
        for (const mode of data.modes) {
          await evaluate(`document.querySelector('[data-mode="${mode}"]').click()`); await settle();
          assert.equal(await evaluate(`document.getElementById('chart-spatial').data[${hIndex}].visible`), false);
        }
        assert.equal(await evaluate(`document.getElementById('chart-metrics').layout.shapes.length`), metricBaseShapes);
        await evaluate(`document.querySelector('#flight-handoffs-visible').click();document.querySelector('[data-mode="mission"]').click();`); await settle();
        await evaluate(`document.querySelector('[data-select-correction="0"]').focus()`); await enter();
        assert.equal(await evaluate(`document.activeElement.getAttribute('aria-pressed')`), 'true');
        await evaluate(`document.querySelector('#flight-corrections-panel summary').click()`);
        assert(await evaluate(`document.querySelector('#flight-corrections-panel details').open`));
        assert((await evaluate(`document.querySelector('#flight-corrections-panel details').innerText`)).includes('2426 / 20.216666666666665 s'));
        // Native pointer events exercise Plotly hover and the existing sample inspector.
        await hoverSample('chart-metrics', 0, 40);
        await checkSampleInspector(480);
        const hoverIndex = hIndex - 1;
        await hoverSample('chart-spatial', hoverIndex, 70);
        await checkSampleInspector(840);
        const beforeH = await evaluate(`document.querySelector('#inspect-caption').textContent`);
        await hoverSample('chart-spatial', hIndex, 0);
        assert.equal(await evaluate(`document.querySelector('#inspect-caption').textContent`), beforeH, 'Exact handoff must not fabricate a sampled record');
        const handoffTooltip = await evaluate(`document.getElementById('chart-spatial').querySelector('.hoverlayer').textContent`);
        assert(handoffTooltip.includes('H1') && handoffTooltip.includes('step=2426'), 'Native H1 hover did not reveal the exact handoff');
        await evaluate(`document.querySelector('#hover-metrics').closest('details').querySelector('summary').focus()`); await enter();
        assert(await evaluate(`document.querySelector('#hover-metrics').closest('details').open`), 'Original controller-metric disclosure failed');
        await evaluate(`(async()=>{await Plotly.relayout('chart-metrics',{'xaxis.range':[5,26],'xaxis.autorange':false});return true;})()`);
        await evaluate(`document.querySelector('#flight-handoffs-visible').click();document.querySelector('[data-select-correction="0"]').click();`); await settle();
        assert.deepEqual(await evaluate(`document.getElementById('chart-metrics').layout.xaxis.range.slice()`), [5,26], 'H selection/toggle reset metric zoom');
        await evaluate(`document.querySelector('#flight-handoffs-visible').click()`); await settle();
        await evaluate(`(async()=>{await Plotly.relayout('chart-metrics',{'xaxis.autorange':true});return true;})()`);
        // Exercise real pointer zoom and pan, then show that annotation selection preserves range.
        await evaluate(`document.getElementById('chart-spatial').scrollIntoView({block:'center'})`); await settle();
        const bounds = await evaluate(`(()=>{const p=document.getElementById('chart-spatial'),r=p.getBoundingClientRect(),s=p._fullLayout._size;return{x:r.left+s.l,y:r.top+s.t,w:s.w,h:s.h,range:p.layout.xaxis.range.slice()}})()`);
        await drag(bounds.x + bounds.w * .2, bounds.y + bounds.h * .25, bounds.w * .45, bounds.h * .5);
        const zoomed = await evaluate(`document.getElementById('chart-spatial').layout.xaxis.range.slice()`);
        assert.notDeepEqual(zoomed, bounds.range, 'Native drag zoom did not change the axis');
        await evaluate(`(async()=>{await Plotly.relayout('chart-spatial',{dragmode:'pan'});return true;})()`); await settle();
        await drag(bounds.x + bounds.w * .5, bounds.y + bounds.h * .5, Math.min(35,bounds.w*.1), 0);
        const panned = await evaluate(`document.getElementById('chart-spatial').layout.xaxis.range.slice()`);
        assert.notDeepEqual(panned, zoomed, 'Native drag pan did not change the axis');
        await evaluate(`document.querySelector('[data-select-correction="0"]').click()`); await settle();
        assert.deepEqual(await evaluate(`document.getElementById('chart-spatial').layout.xaxis.range.slice()`), panned);
        await evaluate(`(async()=>{await Plotly.relayout('chart-spatial',{'xaxis.autorange':true,'yaxis.autorange':true,dragmode:'zoom'});return true;})()`); await settle();
        // Existing chart legend behavior remains usable, including optional thrust components.
        await evaluate(`document.getElementById('chart-metrics').scrollIntoView({block:'center'})`); await settle();
        const legend = await evaluate(`(()=>{const p=document.getElementById('chart-metrics');const ts=[...p.querySelectorAll('.legend .traces')];const t=ts.find(e=>e.textContent.trim()==='tx');const r=t.querySelector('.legendtoggle').getBoundingClientRect();return{x:r.left+r.width/2,y:r.top+r.height/2}})()`);
        await send('Input.dispatchMouseEvent',{type:'mousePressed',...legend,button:'left',clickCount:1});
        await send('Input.dispatchMouseEvent',{type:'mouseReleased',...legend,button:'left',clickCount:1});
        await new Promise(r=>setTimeout(r,450)); await settle();
        assert.equal(await evaluate(`document.getElementById('chart-metrics').data[4].visible`), true, 'Legend tx toggle failed');
        initial.modesOverlayExactStateHoverZoomPanLegendKeyboardPassed = true;
      }
      await checkPageLinks();
      checks.push({kind,width,url,...initial});
    }
  }
  assert.equal(browserErrors.length, 0, JSON.stringify(browserErrors));
  await fs.writeFile(path.join(out, 'browser-checks.json'), JSON.stringify({schema:'rich_waypoint_preview_browser_checks_v1',
    browser:await evaluate('navigator.userAgent'), renderer:receipt.renderer_version, sourceRoot:receipt.source_root,
    originalReportPayloadParityPassed:true, navigationOnlyCopyCount:navigationEdition ? 29 : 0,
    inputHashCount:127, outputHashCount:Object.keys(receipt.output_sha256).length, uniqueLinksChecked:checkedLinks.size,
    checks, entrypointChecks, browserErrors, optionalAssetDiagnostics, evidenceScope:'Real local headless-browser desktop/mobile layout and interaction checks; not new flight evidence or user readability acceptance.'},null,2)+'\n',{flag:'wx'});
  const screenshots = (await fs.readdir(out)).filter(name=>name.endsWith('.png')).length;
  console.log(JSON.stringify({status:'passed',checks:checks.length,entrypointRoutes:entrypointChecks.length,screenshots,links:checkedLinks.size,output:out}));
} finally { ws.close(); }
