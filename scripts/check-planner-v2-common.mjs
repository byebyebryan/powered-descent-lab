#!/usr/bin/env node
// Canonical-site acceptance. No simulations; writes only receipts and a rollback
// copy beneath this checkpoint's create-only research evidence directory.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync,readdirSync,readlinkSync,lstatSync,mkdirSync,writeFileSync,cpSync} from 'node:fs';
import {resolve,join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {expectedProjection,extractProjection} from './check-planner-v2-tree.mjs';
import {extractReportData,verifyCorrectionAnnotations,cdpClient,waitForBrowserPage,captureScreenshot,checkFlightInteractions} from './check-planner-v2-batch.mjs';

const ROOT=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const TASK=join(ROOT,'outputs/research/waypoint_v2_common_templates_20261003');
const CAPTURE='outputs/eval/planner_v2_lab_suite/capture-1791074995928276719';
const SITE='outputs/reports/eval/planner_v2_lab_suite';
const BASE='/reports/eval/planner_v2_lab_suite/';
const OLD='outputs/research/waypoint_v2_batch_tree_20261003/baseline_full_b/baseline.json';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const json=p=>JSON.parse(readFileSync(p,'utf8'));
const write=(p,obj)=>writeFileSync(p,JSON.stringify(obj,null,2)+'\n',{flag:'wx'});
export function treeHash(path) {
  const items=[];
  function visit(p,n) {const s=lstatSync(p);if(s.isSymbolicLink()){items.push([n,'@symlink',readlinkSync(p)]);return;}if(s.isDirectory())for(const name of readdirSync(p).sort())visit(join(p,name),`${n}/${name}`);else {assert(s.isFile());items.push([n,sha(readFileSync(p))]);}}
  visit(path,'.');return {files:items.length,sha256:sha(JSON.stringify(items))};
}
export function protectedSnapshot() {
  const paths=Object.keys(json(join(ROOT,OLD)).snapshot).filter(p=>p!==SITE);
  return Object.fromEntries(paths.map(p=>[p,treeHash(join(ROOT,p))]));
}
export function assertSharedBatch(html) {
  assert(html.includes('data-batch-template="common-v1"'),'must render through the shared full batch template');
  const headings=[...html.matchAll(/<h2[^>]*>(.*?)<\/h2>/gs)].map(m=>m[1].replace(/<[^>]+>/g,'').trim());
  for(const name of ['Overview','Coverage','Context','Review Tree'])assert(headings.includes(name),`missing common section ${name}`);
  assert(headings.indexOf('Overview')<headings.indexOf('Coverage')&&headings.indexOf('Coverage')<headings.indexOf('Context')&&headings.indexOf('Context')<headings.indexOf('Review Tree'));
  assert(!html.includes('<base '),'canonical reader navigation must not be rebased into raw captures');
  assert(!html.includes('preview_final')&&!html.includes('Presentation preview'));
  for(const header of ['Selector','Success / Outcome','Fuel Used','Flight Time','Landing Offset','Reference deviation','Preview'])assert(html.includes(`<th>${header}</th>`),`missing established tree column ${header}`);
}
export function assertSharedSummary(html,summary){
  for(const text of [`${summary.valid_landing_count}/36 target landings`,`${summary.direct_landing_count} direct · ${summary.corrected_landing_count} corrected · ${summary.non_landing_count} non-landings`,`${summary.diagnostic_landing_count} landed · ${summary.diagnostic_non_landing_count} supported non-landings · ${summary.unsupported_count} unsupported`,`${summary.integrity_passed_count}/44 integrity passed`,`${summary.final_source_replay_passed_count}/42 supported source replays passed`])assert(html.includes(text),`incorrect common summary: ${text}`);
}
function checkData() {
  const batch=json(join(ROOT,CAPTURE,'summary.json')),html=readFileSync(join(ROOT,SITE,'index.html'),'utf8');
  assertSharedBatch(html);
  assertSharedSummary(html,batch.summary);
  const receipt=json(join(ROOT,SITE,'render.json'));assert.equal(receipt.schema_id,'planner_v2_common_report_site_v1');
  assert.equal(resolve(receipt.source_capture),join(ROOT,CAPTURE));assert.equal(receipt.source_summary_sha256,sha(readFileSync(join(ROOT,CAPTURE,'summary.json'))));
  for(const [path,hash] of Object.entries(receipt.page_sha256))assert.equal(sha(readFileSync(join(ROOT,SITE,path))),hash,path);
  const expected=batch.cases.map(c=>expectedProjection(c,json(join(ROOT,CAPTURE,c.flight_path)),json(join(ROOT,CAPTURE,c.scenario_path))));
  assert.deepEqual(extractProjection(html),expected);
  const ids=[...html.matchAll(/<tr\b[^>]*data-case-id="([^"]+)"/g)].map(m=>m[1]);assert.equal(ids.length,44);assert.equal(new Set(ids).size,44);
  let hs=0;
  for(const [caseIndex,row] of expected.entries()){
    const match=html.match(new RegExp(`(<tr\\b[^>]*data-case-id="${row.case_id}"[^>]*>)(.*?)<\\/tr>`,'s'));assert(match,row.case_id);
    const attrs=Object.fromEntries([...match[1].matchAll(/([\w-]+)="([^"]*)"/g)].map(m=>[m[1],m[2]]));
    const direct=['clear','diagnostic'].includes(row.group);
    assert.equal(attrs['data-parent'],direct?`v2-${row.group}`:`v2-${row.group}-${row.family}`);
    assert.equal(Number(attrs['data-depth']),direct?1:2);
    assert(match[2].includes(`href="${row.detail_href}"`));
    const cells=[...match[2].matchAll(/<td\b[^>]*>(.*?)<\/td>/gs)].map(m=>m[1].replace(/<[^>]*>/g,''));assert.equal(cells.length,8);
    const metric=(value,unit,digits)=>value===null?'—':`${value.toFixed(digits)}${unit}`;
    assert(cells[1].includes(row.display_outcome));
    for(const [column,value,unit,digits] of [[2,row.fuel_used_pct,'%',1],[3,row.flight_s,'s',2],[4,row.landing_offset_m,'m',3]])assert.equal(cells[column],metric(value,unit,digits),`${row.case_id}: actual displayed metric`);
    assert(cells[1].includes('planning '+metric(row.planning_s,'s',3)));
    const markers=[...match[2].matchAll(/<[^>]*data-handoff="([^"]+)"[^>]*>/g)].map(m=>{const a=Object.fromEntries([...m[0].matchAll(/([\w-]+)="([^"]*)"/g)].map(m=>[m[1],m[2]]));return {number:Number(a['data-handoff']),x:Number(a['data-world-x']),y:Number(a['data-world-y'])};});
    assert.deepEqual(markers,row.handoffs.map((p,i)=>({number:i+1,x:p.x,y:p.y})));hs+=markers.length;
    const c=batch.cases.find(c=>c.case_id===row.case_id),detail=readFileSync(join(ROOT,SITE,c.annotated_report_path),'utf8');
    if(c.status==='simulated'){
      const actual=extractReportData(detail,c.case_id),original=extractReportData(readFileSync(join(ROOT,CAPTURE,c.annotated_report_path),'utf8'),c.case_id);
      verifyCorrectionAnnotations(json(join(ROOT,CAPTURE,c.flight_path)),actual,c.case_id);
      const nav=actual.flightAnnotations.navigation;
      assert.equal(nav.home.href,'/reports/');assert.equal(nav.collection.href,'../../index.html');
      assert.equal(nav.previous?.href??null,caseIndex?`../${batch.cases[caseIndex-1].case_id}/index.html`:null);
      assert.equal(nav.next?.href??null,caseIndex+1<batch.cases.length?`../${batch.cases[caseIndex+1].case_id}/index.html`:null);
      for(const source of [c.scenario_path,c.flight_path,c.summary_path,'expanded-inputs.json'])assert(nav.sourceLinks.some(link=>link.href===receipt.source_base_href+source),`${c.case_id}: raw source ${source}`);
      delete actual.flightAnnotations;delete original.flightAnnotations;assert.deepEqual(actual,original,`${c.case_id}: complete rich payload unchanged`);
      for(const text of ['chart-spatial','chart-metrics','flightStats','botStats','missionDetails','Speed','Throttle','Vectors'])assert(detail.includes(text),`${c.case_id}: ${text}`);
    } else {assert(!detail.includes('const reportData = '));assert(detail.includes('No simulator trajectory'));assert(detail.includes('href="../../index.html"'));for(const source of [c.scenario_path,c.flight_path,c.summary_path])assert(detail.includes(receipt.source_base_href+source));}
  }
  for(const tag of html.matchAll(/<tr\b[^>]*data-group="([^"]+)"[^>]*>/g)){
    const attrs=Object.fromEntries([...tag[0].matchAll(/([\w-]+)="([^"]*)"/g)].map(m=>[m[1],m[2]]));
    const group=attrs['data-kind']==='group'?tag[1].slice(3):attrs['data-parent'].slice(3);
    const family=attrs['data-kind']==='family'?tag[1].slice(attrs['data-parent'].length+1):null;
    const rows=expected.filter(r=>r.group===group&&(!family||r.family===family));
    assert.equal(Number(attrs['data-case-count']),rows.length);assert.equal(Number(attrs['data-landed']),rows.filter(r=>r.landed).length);
    assert.equal(Number(attrs['data-direct']),rows.filter(r=>r.landed&&r.correction_count===0).length);assert.equal(Number(attrs['data-corrected']),rows.filter(r=>r.landed&&r.correction_count>0).length);
  }
  assert.equal(hs,36);
  return {batch,checks:{case_count:44,handoff_count:hs,metrics_exact:true,rich_payloads_unchanged:42,common_batch_template:true,canonical_case_pages:true}};
}
async function browser(batch,out,rootUrl,cdpUrl,legacyPreviewDir){
  const root=new URL(rootUrl);assert(['127.0.0.1','192.168.1.110','localhost'].includes(root.hostname));
  const cdp=new URL(cdpUrl);assert(['127.0.0.1','localhost'].includes(cdp.hostname));
  const tabs=await(await fetch(new URL('/json/list',cdp))).json();const tab=tabs.find(t=>t.type==='page');assert(tab?.webSocketDebuggerUrl);
  const ws=new WebSocket(tab.webSocketDebuggerUrl);await new Promise((ok,bad)=>{ws.addEventListener('open',ok,{once:true});ws.addEventListener('error',bad,{once:true});});
  const client=cdpClient(ws),screenshots=[],checks=[];
  const navigate=async path=>{const url=new URL(path,root).href;await client.send('Page.navigate',{url});await waitForBrowserPage(client,url);};
  const clickLink=async href=>{const url=new URL(href,root).href;assert(await client.evaluate(`(()=>{const a=[...document.querySelectorAll('a')].find(a=>a.href===${JSON.stringify(url)});if(!a)return false;a.click();return true;})()`),`missing navigable link ${href}`);await waitForBrowserPage(client,url);};
  try {
    for(const domain of ['Runtime','Page','Log','Network'])await client.send(`${domain}.enable`);
    for(const size of [{name:'desktop',width:1440,height:1000,mobile:false},{name:'mobile',width:390,height:844,mobile:true}]){
      await client.send('Emulation.setDeviceMetricsOverride',{width:size.width,height:size.height,deviceScaleFactor:1,mobile:size.mobile});
      for(const entry of ['/','/reports/']){await navigate(entry);await clickLink(BASE+'index.html');assert(await client.evaluate(`!!document.querySelector('[data-batch-template="common-v1"]')`));}
      await navigate('/');await clickLink('/reports/topics/waypoint-planning/index.html');await clickLink(BASE+'index.html');
      const state=()=>client.evaluate(`(()=>({groups:[...document.querySelectorAll('tr[data-kind="group"]')].filter(r=>!r.hidden).length,families:[...document.querySelectorAll('tr[data-kind="family"]')].filter(r=>!r.hidden).length,missions:[...document.querySelectorAll('tr.mission-row')].filter(r=>!r.hidden).length,width:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth}))()`);
      const initial=await state();assert.equal(initial.groups,4);assert.equal(initial.families,7);assert.equal(initial.missions,0);assert(initial.scroll<=initial.width+1);
      assert(await client.evaluate(`(()=>[...document.querySelectorAll('.coverage-table th')].every(th=>{const range=document.createRange();range.selectNodeContents(th);const cell=th.getBoundingClientRect();return [...range.getClientRects()].every(text=>text.right<=cell.right+1&&text.left>=cell.left-1)}))()`),'coverage labels must stay inside their cells');
      await client.evaluate(`document.querySelector('.header-context').open=true;document.querySelector('#provenance').open=true`);const open=await state();assert(open.scroll<=open.width+1,'expanded context/provenance must wrap');
      await client.evaluate(`document.querySelector('.header-context').open=false;document.querySelector('#provenance').open=false`);
      screenshots.push(await captureScreenshot(client,out,`${size.name}-common-batch.png`));
      await client.evaluate(`document.querySelector('[data-tree-action="expand-seeds"]').click()`);assert.equal((await state()).missions,44);
      await client.evaluate(`document.querySelector('[data-tree-action="collapse-seeds"]').click()`);assert.equal((await state()).missions,0);
      await client.evaluate(`document.querySelector('[data-tree-action="collapse-depth"]').click()`);assert.equal((await state()).families,0);
      await client.evaluate(`document.querySelector('[data-tree-action="expand-depth"]').click()`);assert.equal((await state()).families,7);
      await client.evaluate(`document.querySelector('tr[data-group="v2-ordinary-ridge"]').dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}))`);assert.equal((await state()).missions,4);
      await client.evaluate(`document.querySelector('tr[data-group="v2-ordinary-ridge"]').dispatchEvent(new KeyboardEvent('keydown',{key:' ',bubbles:true}))`);assert.equal((await state()).missions,0);
      const jump=await client.evaluate(`(()=>{const cell=document.querySelector('[data-tree-group="v2-ordinary-ridge"]');if(!cell)return false;cell.click();return !!document.querySelector('.coverage-target');})()`);assert(jump,'coverage opens and highlights tree branch');
      assert.equal((await state()).missions,4,'coverage opens only the requested family');
      screenshots.push(await captureScreenshot(client,out,`${size.name}-ridge-branch.png`));
      for(const id of ['v2_clear_845','v2_ridge_early','v2_plateau_reference_900','v2_diag_high_900','v2_diag_lunar_gravity']){
        const record=batch.cases.find(c=>c.case_id===id);await client.evaluate(`document.querySelector('[data-tree-action="expand-seeds"]').click()`);
        const path=BASE+record.annotated_report_path;await clickLink(path);await waitForBrowserPage(client,new URL(path,root).href,record.status==='simulated');
        if(record.status==='simulated')await checkFlightInteractions(client,record,`${size.name}/${id}`);
        screenshots.push(await captureScreenshot(client,out,`${size.name}-${id}.png`));
        await clickLink(BASE+'index.html');
      }
      await clickLink('/reports/topics/waypoint-planning/index.html');await clickLink('/reports/index.html');
      await navigate('/reports/library/index.html');await clickLink(BASE+'index.html');
      checks.push({width:size.name,root_and_reports_round_trips:true,topic_library_reachable:true,tree_coverage_keyboard_passed:true,rich_interactions_passed:true,no_document_overflow:true,coverage_labels_contained:true});
      if(legacyPreviewDir)for(const pack of ['terminal','transfer']){
        const path='/'+resolve(legacyPreviewDir).slice(join(ROOT,'outputs').length+1)+`/${pack}.html`;await navigate(path);
        assert(await client.evaluate(`!!document.querySelector('[data-batch-template="common-v1"]')`));
        assert(await client.evaluate(`document.documentElement.scrollWidth<=document.documentElement.clientWidth+1`),'legacy local table scrolling must not overflow document');
        const leaves=()=>client.evaluate(`[...document.querySelectorAll('tr.seed-row')].filter(r=>!r.hidden).length`);
        assert.equal(await leaves(),0);await client.evaluate(`document.querySelector('[data-tree-action="expand-seeds"]').click()`);assert(await leaves()>0);
        await client.evaluate(`document.querySelector('[data-tree-action="collapse-seeds"]').click()`);assert.equal(await leaves(),0);
        const initialSummaries=await client.evaluate(`[...document.querySelectorAll('tr.summary-row')].filter(r=>!r.hidden).length`);
        await client.evaluate(`document.querySelector('[data-tree-action="collapse-depth"]').click();document.querySelector('[data-tree-action="expand-depth"]').click()`);
        assert.equal(await client.evaluate(`[...document.querySelectorAll('tr.summary-row')].filter(r=>!r.hidden).length`),initialSummaries);
        const coverage=await client.evaluate(`(()=>{const c=document.querySelector('[data-tree-tokens]');if(c)c.click();return !!document.querySelector('.coverage-target');})()`);assert(coverage);
        screenshots.push(await captureScreenshot(client,out,`${size.name}-shared-${pack}.png`));
      }
    }
    for(const c of batch.cases){const r=await fetch(new URL(BASE+c.annotated_report_path,root));assert(r.ok,c.case_id);for(const source of [c.scenario_path,c.flight_path,c.summary_path]){const raw=await fetch(new URL('/'+CAPTURE.replace(/^outputs\//,'')+'/'+source,root),{method:'HEAD'});assert(raw.ok,`${c.case_id}: ${source}`);}}
    assert.deepEqual(client.errors,[]);return {checks,screenshots,errors:client.errors};
  } finally {ws.close();}
}
async function main(){
  const args=process.argv.slice(2),options={};for(let i=0;i<args.length;i+=2){assert(args[i].startsWith('--')&&args[i+1]);options[args[i].slice(2)]=args[i+1];}
  const out=resolve(options['output-dir']);assert(out!==TASK&&out.startsWith(TASK+'/'));mkdirSync(dirname(out),{recursive:true});mkdirSync(out);
  if(options.snapshot==='yes'){
    const old=json(join(ROOT,OLD)).snapshot,current=Object.fromEntries(Object.keys(old).map(p=>[p,treeHash(join(ROOT,p))]));assert.deepEqual(current,old,'start must match previous preservation gate');
    cpSync(join(ROOT,SITE),join(out,'previous-published-v2'),{recursive:true,force:false,errorOnExist:true});
    write(join(out,'baseline.json'),{protected:protectedSnapshot(),previous_site:current[SITE]});console.log(JSON.stringify({baseline:join(out,'baseline.json')}));return;
  }
  const receipt={status:'failed',errors:[]};try{
    const baseline=json(resolve(options.baseline));assert.deepEqual(protectedSnapshot(),baseline.protected,'protected source and unrelated reports must remain identical');
    const result=checkData();receipt.checks=result.checks;
    if(options['cdp-url'])receipt.browser=await browser(result.batch,out,options['root-url'],options['cdp-url'],options['legacy-preview-dir']);
    assert.deepEqual(protectedSnapshot(),baseline.protected,'post-check preservation');receipt.status='passed';
  }catch(e){receipt.errors.push(e.stack??String(e));process.exitCode=1;}write(join(out,'acceptance.json'),receipt);console.log(JSON.stringify({status:receipt.status,receipt:join(out,'acceptance.json'),errors:receipt.errors}));
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))await main();
