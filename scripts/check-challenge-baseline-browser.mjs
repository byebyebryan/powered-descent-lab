#!/usr/bin/env node
// Explicit disposable-browser QA for V2 rich paired reports; no server lifecycle.
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {parseArgs} from 'node:util';
import {cdpClient} from './planner-v2-browser-helpers.mjs';

const {values}=parseArgs({options:{'browser-ws':{type:'string'},report:{type:'string'},output:{type:'string'}},strict:true});
assert(values['browser-ws']&&values.report&&values.output,'Require --browser-ws WS --report HTTP_URL --output NEW_QA_DIR');
const report=new URL(values.report);assert(['127.0.0.1','localhost'].includes(report.hostname));
const browserUrl=new URL(values['browser-ws']);assert.equal(browserUrl.hostname,'127.0.0.1');
const output=resolve(values.output);mkdirSync(output);
const receipt=await fetch(new URL('render.json',report)).then(r=>{assert(r.ok);return r.json();});
assert.equal(receipt.kind,'saved_procedural_presentation_refinement');
assert.equal(receipt.scheduled_attempts,receipt.native_details+receipt.status_details);
assert.equal(receipt.captures.length,receipt.native_details);
assert.equal(receipt.new_flights,0);assert.equal(receipt.new_replays,0);
const bounds=new Map();
for(const capture of receipt.captures){const value=JSON.stringify(capture.shared_world_bounds);if(bounds.has(capture.mission))assert.equal(value,bounds.get(capture.mission));else bounds.set(capture.mission,value);}
const open=async url=>{const ws=new WebSocket(url);await new Promise((r,j)=>{ws.addEventListener('open',r,{once:true});ws.addEventListener('error',j,{once:true});});return ws;};
const browserSocket=await open(browserUrl.href);const browser=cdpClient(browserSocket);
const {targetId}=await browser.send('Target.createTarget',{url:'about:blank'});
const targets=await fetch(`http://127.0.0.1:${browserUrl.port}/json/list`).then(r=>r.json());
const socket=await open(targets.find(t=>t.id===targetId).webSocketDebuggerUrl);const client=cdpClient(socket);
const pause=ms=>new Promise(r=>setTimeout(r,ms));
const wait=async expression=>{for(let i=0;i<200;i++){if(await client.evaluate(expression))return;await pause(50);}throw Error(`Readiness: ${expression}`);};
const screenshots=[];const checks=[];
const screenshot=async name=>{await pause(100);const {data}=await client.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});writeFileSync(join(output,name),Buffer.from(data,'base64'),{flag:'wx'});screenshots.push(name);};
const noOverflow=async()=>assert(await client.evaluate('document.documentElement.scrollWidth<=innerWidth+1'));
try {
  for(const domain of ['Page','Runtime','Network','Log'])await client.send(`${domain}.enable`);
  for(const size of [{name:'desktop',width:1440,height:1000,mobile:false},{name:'mobile',width:390,height:844,mobile:true}]) {
    await client.send('Emulation.setDeviceMetricsOverride',{...size,deviceScaleFactor:1});
    await client.send('Page.navigate',{url:report.href});await wait('document.readyState==="complete"&&!!window.PdPairedReview');
    await noOverflow();assert.equal(await client.evaluate('document.querySelectorAll("#review-tree-root img").length'),0);
    assert(await client.evaluate('PdPairedReview.data.missions.every(m=>m.path.length===3)&&PdPairedReview.data.branches.some(b=>b.kind==="morphology")&&PdPairedReview.data.branches.some(b=>b.kind==="band")&&!PdPairedReview.data.branches.some(b=>b.kind==="stratum")'));
    await screenshot(`${size.name}-overview.png`);
    const selected=await client.evaluate(`(()=>{const d=PdPairedReview.data;const leaves=d.branches.filter(b=>!d.branches.some(c=>c.parent===b.id));const b=leaves.map(b=>({id:b.id,count:d.missions.filter(m=>m.path.includes(b.id)).length})).sort((a,b)=>b.count-a.count)[0];PdPairedReview.focus(b.id,'0');document.querySelector('[data-paired-mission]').scrollIntoView({block:'start'});return b;})()`);
    assert.equal(await client.evaluate('document.querySelectorAll("[data-paired-mission]").length'),Math.min(12,selected.count));
    await wait('[...document.querySelectorAll(".mission-preview img")].slice(0,2).every(i=>i.complete&&i.naturalWidth>0)');
    await noOverflow();await screenshot(`${size.name}-terrain-band.png`);
    if(selected.count>12){
      await client.evaluate('document.querySelector(".mission-browser [data-review-page=next]").click()');
      assert.equal(await client.evaluate('PdPairedReview.pageInfo.page'),2);
      const url=await client.evaluate('location.href');await client.send('Page.navigate',{url});await wait('!!window.PdPairedReview&&PdPairedReview.pageInfo.page===2');
      assert(await client.evaluate('document.querySelectorAll("[data-paired-mission]").length<=12'));
    }
    await client.evaluate('document.getElementById("review-direction").value="-1";document.getElementById("review-direction").dispatchEvent(new Event("change"))');
    assert(await client.evaluate('[...document.querySelectorAll("[data-paired-mission]")].every(r=>r.dataset.direction==="-1")'));
    await client.evaluate('document.getElementById("review-direction").value="all";document.getElementById("review-direction").dispatchEvent(new Event("change"))');
    await client.evaluate(`PdPairedReview.focus(${JSON.stringify(selected.id)},'0')`);
    await client.evaluate('document.querySelector("[data-paired-mission]").click();document.querySelector(".mission-comparison").open=true;document.querySelector(".paired-preview-row").scrollIntoView({block:"start"})');
    const detail=await client.evaluate('document.querySelector(".paired-detail").href');
    await client.send('Page.navigate',{url:detail});await wait('document.readyState==="complete"&&!!document.getElementById("static-flight-fallback")');
    assert.equal(await client.evaluate('document.querySelectorAll("#static-flight-fallback").length'),1);
    assert.equal(await client.evaluate('document.querySelectorAll("#static-metrics-fallback").length'),1);
    assert(await client.evaluate('document.querySelectorAll("[data-mode]").length===5'));
    await noOverflow();await screenshot(`${size.name}-rich-detail.png`);
    await wait('document.querySelector(".review-detail-nav")?.dataset.contextReady==="true"');
    const back=await client.evaluate('document.querySelector("[data-back-review]").href');
    await client.send('Page.navigate',{url:back});await wait('!!window.PdPairedReview&&document.querySelector("[data-paired-mission][aria-expanded=true]")');
    assert(await client.evaluate(`PdPairedReview.data.missions.find(m=>m.id===location.hash.slice(1)).path.includes(${JSON.stringify(selected.id)})`));
    assert(await client.evaluate('document.querySelectorAll("[data-paired-mission]").length<=12'));
    await noOverflow();
    checks.push({viewport:size.name,largest_band:selected.count,materialized_bound:12,pagination_exercised:selected.count>12,three_level_geometry:true,detail_static_and_rich:true,detail_back_context:true});
  }
  assert.deepEqual(client.errors,[]);
  const result={schema_version:1,passed:true,report:report.href,shared_preview_bounds_checked:bounds.size,checks,screenshots,errors:client.errors};
  writeFileSync(join(output,'browser-result.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(result,null,2));
} finally {
  await browser.send('Target.closeTarget',{targetId}).catch(()=>{});
  socket.close();browserSocket.close();
}
