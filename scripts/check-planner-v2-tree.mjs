#!/usr/bin/env node
// Read-only capture/site audit. Writes only create-only check receipts/screenshots.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync, readdirSync, readlinkSync, lstatSync, mkdirSync, writeFileSync} from 'node:fs';
import {resolve, join, relative, dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {cdpClient, waitForBrowserPage, captureScreenshot, checkFlightInteractions} from './check-planner-v2-batch.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const CAPTURE = 'outputs/eval/planner_v2_lab_suite/capture-1791074995928276719';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const json = path => JSON.parse(readFileSync(path, 'utf8'));
const write = (path, value) => writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, {flag:'wx'});

function treeHash(path) {
  const files = [];
  function visit(path, name) {
    const stat = lstatSync(path);
    if(stat.isSymbolicLink()) {files.push([name, '@symlink', readlinkSync(path)]);return;}
    if (stat.isDirectory()) {
      for (const entry of readdirSync(path).sort()) visit(join(path,entry), `${name}/${entry}`);
    } else { assert(stat.isFile()); files.push([name, sha(readFileSync(path))]); }
  }
  visit(path, '.');
  return {files:files.length, sha256:sha(JSON.stringify(files))};
}

function snapshot() {
  const retained = json(join(ROOT,'outputs/research/planner_v2_eval_activation_20261003/preservation.json'));
  const roots = [...Object.keys(retained.before), CAPTURE,
    'outputs/eval/planner_v2_lab_suite/current.json',
    'outputs/reports/eval/planner_v2_lab_suite',
    'outputs/reports/eval/terminal_bot_lab_suite',
    'outputs/reports/eval/transfer_route_angle_radius_suite',
    'outputs/eval/terminal_bot_lab_suite',
    'outputs/eval/transfer_route_angle_radius_suite',
    'outputs/index.html','outputs/reports/index.html',
    'outputs/reports/topics/waypoint-planning/index.html','fixtures/packs/planner_v2_lab_suite.json'];
  return Object.fromEntries(roots.map(path=>[path,treeHash(join(ROOT,path))]));
}

export function expectedProjection(record, flight, scenario) {
  const departed = Boolean((flight.manifest && flight.manifest.physics_steps > 0) || flight.ordinary_flight?.samples.some(s=>s.physics_step>0));
  const landed = flight.planning_stop === 'landed' && flight.physical_outcome === 'landed_on_target'
    && flight.mission_outcome === 'success' && flight.integrity_passed && flight.final_source_replay_passed;
  const unsupported = flight.planning_stop === 'unsupported';
  return {
    case_id:record.case_id, group:record.group, family:record.family,
    label:record.case_id.replace(/^(?:v2_|fresh_)/,'').replaceAll('_',' '),
    detail_href:record.annotated_report_path,
    display_outcome:unsupported ? 'Unsupported — not simulated' : !departed ? `Not departed — ${record.planning_stop ?? 'planning stop'}`
      : landed ? 'Landed on target' : `${record.physical_outcome ?? 'unverified'} — ${record.planning_stop ?? 'missing stop'}`,
    departed, landed:Boolean(landed), unsupported,
    integrity_passed:flight.integrity_passed, replay_passed:unsupported ? null : flight.final_source_replay_passed,
    correction_count:flight.correction_count,
    fuel_used_pct:departed && flight.manifest && scenario.vehicle.max_fuel_kg > 0 ? 100 * flight.manifest.summary.fuel_used_kg / scenario.vehicle.max_fuel_kg : null,
    flight_s:departed && flight.manifest ? flight.manifest.sim_time_s : null,
    landing_offset_m:departed && landed && flight.manifest?.summary.landing ? Math.abs(flight.manifest.summary.landing.touchdown_center_offset_m) : null,
    planning_s:unsupported ? null : record.planning_s,
    handoffs:flight.segments.filter(s=>s.kind==='local_correction').map(s=>s.end_state.position_m),
  };
}

export function extractProjection(html) {
  const match = html.match(/const batchTreeData = (.*?);<\/script>/s);
  assert(match, 'missing tree projection');
  return JSON.parse(match[1]);
}

export function assertSummaryText(html, summary) {
  for (const text of [
    `${summary.integrity_passed_count}/44 integrity passed`,
    `${summary.final_source_replay_passed_count}/42 supported source replays passed`,
    `<strong>${summary.valid_landing_count}/36</strong>`,
    `${summary.direct_landing_count} direct · ${summary.corrected_landing_count} corrected · ${summary.non_landing_count} non-landings`,
    `${summary.diagnostic_landing_count} landed · ${summary.diagnostic_non_landing_count} supported non-landings · ${summary.unsupported_count} unsupported`,
    `${summary.crash_count} crashes.`,
  ]) assert(html.includes(text), `missing or inaccurate page-level summary: ${text}`);
}

function checkTree(previewRoot) {
  const receipt = json(join(previewRoot,'preview.json'));
  assert.equal(receipt.schema_id,'planner_v2_batch_tree_preview_v1');
  assert.equal(resolve(receipt.source_capture),join(ROOT,CAPTURE));
  assert.equal(receipt.source_summary_sha256,sha(readFileSync(join(ROOT,CAPTURE,'summary.json'))));
  const html = readFileSync(join(previewRoot,'index.html'),'utf8');
  assert.equal(receipt.rendered_html_sha256,sha(Buffer.from(html)), 'preview receipt must bind exact rendered HTML');
  const batch = json(join(ROOT,CAPTURE,'summary.json'));
  assertSummaryText(html,batch.summary);
  const expected = batch.cases.map(record=>expectedProjection(record,json(join(ROOT,CAPTURE,record.flight_path)),json(join(ROOT,CAPTURE,record.scenario_path))));
  assert.deepEqual(extractProjection(html),expected,'all 44 display metrics/statuses/handoffs must exactly match source');
  const ids = [...html.matchAll(/<tr\b[^>]*data-case-id="([^"]+)"/g)].map(m=>m[1]);
  assert.equal(ids.length,44); assert.equal(new Set(ids).size,44);
  assert.deepEqual([...ids].sort(),batch.cases.map(c=>c.case_id).sort());
  for (const row of expected) {
    const match = html.match(new RegExp(`(<tr\\b[^>]*data-case-id="${row.case_id}"[^>]*>)(.*?)<\\/tr>`,'s'));
    const section = match?.[2];
    assert(section,`${row.case_id}: missing actual row`);
    const attrs = Object.fromEntries([...match[1].matchAll(/([\w-]+)="([^"]*)"/g)].map(m=>[m[1],m[2]]));
    const directParent = ['clear','diagnostic'].includes(row.group);
    assert.equal(attrs['data-parent'],directParent?`v2-${row.group}`:`v2-${row.group}-${row.family}`,`${row.case_id}: correct branch`);
    assert.equal(Number(attrs['data-depth']),directParent?1:2,`${row.case_id}: correct tree depth`);
    assert(section.includes(`href="${row.detail_href}"`));
    const handoffs = [...section.matchAll(/<[^>]*\bdata-handoff="([^\"]+)"[^>]*>/g)].map(m=>{
      const attrs = Object.fromEntries([...m[0].matchAll(/([\w-]+)="([^\"]*)"/g)].map(m=>[m[1],m[2]]));
      return {number:Number(attrs['data-handoff']),x:Number(attrs['data-world-x']),y:Number(attrs['data-world-y'])};
    });
    assert.deepEqual(handoffs,row.handoffs.map((p,i)=>({number:i+1,x:p.x,y:p.y})),`${row.case_id}: exact preview H coordinates`);
    if (!row.departed) {
      assert(section.includes(row.unsupported ? 'not simulated' : 'not departed'));
      assert(!section.includes('Recorded trajectory')); assert(!section.includes('Direct ·'));
    }
  }
  assert.equal([...html.matchAll(/data-kind="group"/g)].length,4);
  assert.equal([...html.matchAll(/data-kind="family"/g)].length,7);
  for(const tag of [...html.matchAll(/<tr\b[^>]*data-group="([^"]+)"[^>]*>/g)]) {
    const attrs=Object.fromEntries([...tag[0].matchAll(/([\w-]+)="([^"]*)"/g)].map(m=>[m[1],m[2]]));
    const group=attrs['data-kind']==='group'?tag[1].slice(3):attrs['data-parent'].slice(3);
    const family=attrs['data-kind']==='family'?tag[1].slice(attrs['data-parent'].length+1):null;
    const children=expected.filter(r=>r.group===group&&(!family||r.family===family));
    assert.equal(Number(attrs['data-case-count']),children.length);
    assert.equal(Number(attrs['data-landed']),children.filter(r=>r.landed).length);
    assert.equal(Number(attrs['data-direct']),children.filter(r=>r.landed&&r.correction_count===0).length);
    assert.equal(Number(attrs['data-corrected']),children.filter(r=>r.landed&&r.correction_count>0).length);
  }
  assert(!html.includes('Expand Seeds')); assert(!html.includes('data-tree-action="toggle-baseline"'));
  assert(html.includes('Expand Missions')); assert(html.includes('id="provenance"'));
  assert(!html.includes('<details class="panel" id="provenance" open'));
  return {batch,receipt,checks:{case_count:44,group_count:4,family_count:7,metrics_exact:true,handoff_count:expected.reduce((n,r)=>n+r.handoffs.length,0),source_summary_unchanged:true}};
}

async function browser(previewRoot, result, outputDir, rootUrl, cdpUrl, legacyPreviewDir) {
  const root = new URL(rootUrl);
  assert(['127.0.0.1','localhost','192.168.1.110'].includes(root.hostname));
  const previewUrl = new URL(`/${relative(join(ROOT,'outputs'),join(previewRoot,'index.html')).replaceAll('\\','/')}`,root).href;
  const baseUrl = new URL(result.receipt.capture_base_href,root).href;
  const cdp = new URL(cdpUrl); assert(['127.0.0.1','localhost'].includes(cdp.hostname));
  const targets = await (await fetch(new URL('/json/list',cdp))).json();
  const target = targets.find(t=>t.type==='page'); assert(target?.webSocketDebuggerUrl);
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((done,reject)=>{ws.addEventListener('open',done,{once:true});ws.addEventListener('error',reject,{once:true});});
  const client = cdpClient(ws), screenshots=[],checks=[];
  try {
    for(const name of ['Runtime','Page','Log','Network']) await client.send(`${name}.enable`);
    for(const size of [{name:'desktop',width:1440,height:1000,mobile:false},{name:'mobile',width:390,height:844,mobile:true}]) {
      await client.send('Emulation.setDeviceMetricsOverride',{width:size.width,height:size.height,deviceScaleFactor:1,mobile:size.mobile});
      await client.send('Page.navigate',{url:previewUrl}); await waitForBrowserPage(client,previewUrl);
      const view=()=>client.evaluate(`(()=>({groups:[...document.querySelectorAll('tr[data-kind="group"]')].filter(r=>!r.hidden).length,families:[...document.querySelectorAll('tr[data-kind="family"]')].filter(r=>!r.hidden).length,missions:[...document.querySelectorAll('tr.mission-row')].filter(r=>!r.hidden).length,width:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth}))()`);
      let state=await view(); assert.equal(state.groups,4);assert.equal(state.families,7);assert.equal(state.missions,0);assert(state.scroll<=state.width+1);
      screenshots.push(await captureScreenshot(client,outputDir,`${size.name}-tree-summary.png`));
      await client.evaluate(`document.querySelector('tr[data-group="v2-ordinary-ridge"]').dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}))`);
      assert.equal((await view()).missions,4,'Enter expands one family');
      await client.evaluate(`document.querySelector('tr[data-group="v2-ordinary-ridge"]').dispatchEvent(new KeyboardEvent('keydown',{key:' ',bubbles:true}))`);
      assert.equal((await view()).missions,0,'Space collapses one family');
      await client.evaluate(`document.querySelector('tr[data-group="v2-clear"]').click()`);
      assert.equal((await view()).missions,11,'first click reveals directly nested missions');
      await client.evaluate(`document.querySelector('tr[data-group="v2-clear"]').click()`);
      assert.equal((await view()).missions,0,'second click collapses directly nested missions');
      await client.evaluate(`document.querySelector('[data-tree-action="expand-seeds"]').click()`);
      assert.equal((await view()).missions,44,'Expand Missions reveals all');
      screenshots.push(await captureScreenshot(client,outputDir,`${size.name}-tree-missions.png`));
      await client.evaluate(`document.querySelector('[data-tree-action="collapse-seeds"]').click()`);
      assert.equal((await view()).missions,0,'Collapse Missions hides all');
      await client.evaluate(`document.querySelector('[data-tree-action="collapse-depth"]').click()`);
      assert.equal((await view()).families,0,'Collapse hides families');
      await client.evaluate(`document.querySelector('[data-tree-action="expand-depth"]').click()`);
      assert.equal((await view()).families,7,'Expand reveals families');
      await client.evaluate(`document.querySelector('tr[data-group="v2-ordinary"]').click()`);
      assert.equal((await view()).families,3,'collapsing parent recursively hides four families');
      await client.evaluate(`document.querySelector('[data-tree-action="expand-seeds"]').click()`);
      for(const id of ['v2_clear_845','v2_ridge_early','v2_plateau_reference_900','v2_diag_high_900','v2_diag_lunar_gravity']) {
        const record=result.batch.cases.find(r=>r.case_id===id);
        const href=await client.evaluate(`document.querySelector('tr[data-case-id="${id}"] a.mission-link').href`);
        assert.equal(href,new URL(record.annotated_report_path,baseUrl).href);
        await client.evaluate(`document.querySelector('tr[data-case-id="${id}"] a.mission-link').click()`);
        await waitForBrowserPage(client,href,record.status==='simulated');
        if(record.status==='simulated') await checkFlightInteractions(client,record,`${id}/${size.name}`);
        else assert(await client.evaluate(`document.body.innerText.includes('No simulator trajectory')`));
        screenshots.push(await captureScreenshot(client,outputDir,`${size.name}-${id}.png`));
        await client.send('Page.navigate',{url:previewUrl});await waitForBrowserPage(client,previewUrl);
        await client.evaluate(`document.querySelector('[data-tree-action="expand-seeds"]').click()`);
      }
      checks.push({width:size.name,mouse_keyboard_toolbar_passed:true,all_missions_reachable:true,no_document_overflow:true});
    }
    if(legacyPreviewDir)for(const width of [{name:'desktop',width:1440,height:1000,mobile:false},{name:'mobile',width:390,height:844,mobile:true}]) {
      await client.send('Emulation.setDeviceMetricsOverride',{width:width.width,height:width.height,deviceScaleFactor:1,mobile:width.mobile});
      for(const pack of ['terminal','transfer']) {
        const url=new URL(`/${relative(join(ROOT,'outputs'),join(resolve(legacyPreviewDir),`${pack}.html`)).replaceAll('\\','/')}`,root).href;
        await client.send('Page.navigate',{url});await waitForBrowserPage(client,url);
        const state=()=>client.evaluate(`(()=>({summaries:[...document.querySelectorAll('tr.summary-row')].filter(r=>!r.hidden).length,leaves:[...document.querySelectorAll('tr.seed-row')].filter(r=>!r.hidden).length,width:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth,initialized:!!document.getElementById('review-tree-root').__pdBatchTreeApi}))()`);
        const initial=await state();assert(initial.initialized);assert(initial.summaries>0);assert.equal(initial.leaves,0);assert(initial.scroll<=initial.width+1);
        await client.evaluate(`document.querySelector('[data-tree-action="expand-seeds"]').click()`);
        assert((await state()).leaves>0,'legacy seeds expand');
        await client.evaluate(`document.querySelector('[data-tree-action="collapse-seeds"]').click()`);
        assert.equal((await state()).leaves,0,'legacy seeds collapse');
        await client.evaluate(`document.querySelector('[data-tree-action="collapse-depth"]').click()`);
        assert((await state()).summaries<=initial.summaries);
        await client.evaluate(`document.querySelector('[data-tree-action="expand-depth"]').click()`);
        assert.equal((await state()).summaries,initial.summaries,'legacy depth restoration');
        const coverage=await client.evaluate(`(()=>{const c=document.querySelector('[data-tree-tokens]');if(c)c.click();return {available:!!c,target:!!document.querySelector('.coverage-target')}})()`);
        if(coverage.available)assert(coverage.target,'legacy coverage jumps to a matching row');
        const lane=await client.evaluate(`(()=>{const r=[...document.querySelectorAll('tr[data-kind="lane"]')].find(r=>!r.hidden&&r.dataset.group);if(!r)return false;r.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return true;})()`);
        if(lane)assert((await state()).leaves>0,'legacy lane keyboard seed expansion');
        screenshots.push(await captureScreenshot(client,outputDir,`${width.name}-legacy-${pack}.png`));
        checks.push({kind:`legacy-${pack}`,width:width.name,initial,coverage,seed_expansion_passed:true,depth_controls_passed:true});
      }
    }
    // All case links must resolve even when collapsed.
    for(const record of result.batch.cases) {
      const response=await fetch(new URL(record.annotated_report_path,baseUrl));assert(response.ok,record.case_id);
    }
    assert.deepEqual(client.errors,[],'browser JavaScript/assets');
    return {preview_url:previewUrl,checks,screenshots,errors:client.errors};
  } finally {ws.close();}
}

async function main() {
  const args=process.argv.slice(2), options={};
  for(let i=0;i<args.length;i+=2) {assert(args[i].startsWith('--')&&args[i+1]);options[args[i].slice(2)]=args[i+1];}
  const out=resolve(options['output-dir']);
  const auditRoot=join(ROOT,'outputs/research/waypoint_v2_batch_tree_20261003');
  assert(out!==auditRoot&&out.startsWith(`${auditRoot}/`),'audit writes must stay beneath this checkpoint research directory');
  mkdirSync(dirname(out),{recursive:true});mkdirSync(out);
  if(options.snapshot==='yes') {
    const current=snapshot();
    if(options.baseline) {const previous=json(resolve(options.baseline)).snapshot;assert.deepEqual(Object.fromEntries(Object.keys(previous).map(p=>[p,current[p]])),previous,'original preservation scope must be unchanged before extending baseline');}
    write(join(out,'baseline.json'),{schema_id:'planner_v2_tree_preservation_v1',snapshot:current});console.log(JSON.stringify({baseline:join(out,'baseline.json')}));return;
  }
  let receipt={status:'failed',errors:[]};
  try {
    assert.deepEqual(snapshot(),json(resolve(options.baseline)).snapshot,'all captured/detail/published/legacy evidence must remain byte-identical');
    const previewRoot=resolve(options['preview-dir']);const result=checkTree(previewRoot);receipt.checks=result.checks;
    if(options['cdp-url'])receipt.browser=await browser(previewRoot,result,out,options['root-url'],options['cdp-url'],options['legacy-preview-dir']);
    assert.deepEqual(snapshot(),json(resolve(options.baseline)).snapshot,'post-audit preservation');
    receipt.status='passed';
  } catch(error) {receipt.errors.push(error.stack??String(error));process.exitCode=1;}
  write(join(out,'acceptance.json'),receipt);console.log(JSON.stringify({status:receipt.status,receipt:join(out,'acceptance.json'),errors:receipt.errors}));
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))await main();
