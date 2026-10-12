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
    assert.equal(await client.evaluate('document.querySelector("[data-paired-depth=\\"3\\"]").textContent'),'Start browsing');
    const traversal=await client.evaluate(`(()=>{
      document.querySelector('[data-paired-depth="3"]').click();
      const ids=[...document.querySelector('.mission-browser [data-review-stratum-select]').options].map(o=>o.value);
      const visited=[];let maximumGroups=0;
      for(let i=0;i<ids.length;i++){
        const d=PdPairedReview.data;const current=d.branches.find(b=>b.id===PdPairedReview.state.stratum);
        const groups=[...document.querySelectorAll('[data-paired-group]')];maximumGroups=Math.max(maximumGroups,groups.length);
        if(current.id!==ids[i]||groups.length!==3)throw Error('Group navigation accumulated siblings or skipped a group');
        const next=document.querySelector('.mission-browser [data-review-stratum=next]');
        const control=next.getBoundingClientRect();const active=document.querySelector('[data-paired-group="'+current.id+'"]').getBoundingClientRect();
        const mission=document.querySelector('[data-paired-mission]').getBoundingClientRect();
        if(control.top<0||control.bottom>innerHeight||active.top<control.bottom||active.bottom>innerHeight||mission.top>=innerHeight)throw Error('Group navigation and active missions left the viewport');
        if(document.querySelectorAll('[data-paired-mission]').length>12)throw Error('Unbounded mission page');
        visited.push(current.id);if(i<ids.length-1)next.click();else if(!next.disabled)throw Error('Last group remains advanceable');
      }
      return {visited,maximumGroups};
    })()`);
    await noOverflow();await screenshot(`${size.name}-last-group.png`);
    assert.equal(await client.evaluate('document.querySelector(".mission-browser-footer [data-review-stratum=next]").disabled'),true);
    await client.evaluate('document.querySelector(".mission-browser-footer [data-review-stratum=previous]").click()');
    assert.equal(await client.evaluate('PdPairedReview.state.stratum'),traversal.visited.at(-2));
    await client.evaluate('document.querySelector("[data-review-tree]").click()');
    assert(await client.evaluate('document.querySelectorAll("[data-paired-group]").length>3'));
    await client.evaluate(`(()=>{
      const nav=document.querySelector('.mission-browser');
      const ids=[...nav.querySelector('select').options].map(o=>o.value);
      const check=id=>{if(!PdPairedReview.showGroupTree||PdPairedReview.state.stratum!==id||document.querySelectorAll('[data-paired-group]').length<=3)throw Error('Group navigation changed the full-tree view');};
      nav.querySelector('[data-review-stratum=next]').click();check(ids.at(-1));
      document.querySelector('.mission-browser [data-review-stratum=previous]').click();check(ids.at(-2));
      document.querySelector('.mission-browser-footer [data-review-stratum=previous]').click();check(ids.at(-3));
      const select=document.querySelector('.mission-browser select');select.value=ids[1];select.dispatchEvent(new Event('change',{bubbles:true}));check(ids[1]);
      document.querySelector('[data-paired-depth="3"]').click();check(ids[1]);
    })()`);
    await screenshot(`${size.name}-full-tree-navigation.png`);
    let treeUrl=await client.evaluate('location.href');
    await client.send('Page.navigate',{url:treeUrl});await wait('!!window.PdPairedReview');
    assert(await client.evaluate('PdPairedReview.showGroupTree&&new URLSearchParams(location.search).get("tree")==="full"'));
    await client.evaluate(`(()=>{
      const id=PdPairedReview.state.stratum;const path=PdPairedReview.data.missions.find(m=>m.path.includes(id)).path;
      document.querySelector('[data-paired-group="'+id+'"] .expander').click();
      if(PdPairedReview.state.stratum!==id||!PdPairedReview.showGroupTree||PdPairedReview.opened.has(id)||!path.slice(0,-1).every(p=>PdPairedReview.opened.has(p))||document.querySelectorAll('[data-paired-mission]').length)throw Error('Band collapse closed its parent levels or changed navigation');
      if(!document.getElementById('review-count').textContent.includes('collapsed'))throw Error('Collapsed group still claims visible missions');
    })()`);
    await screenshot(`${size.name}-collapsed-band.png`);
    treeUrl=await client.evaluate('location.href');
    await client.send('Page.navigate',{url:treeUrl});await wait('!!window.PdPairedReview');
    assert(await client.evaluate('PdPairedReview.showGroupTree&&!PdPairedReview.opened.has(PdPairedReview.state.stratum)&&document.querySelectorAll("[data-paired-mission]").length===0'));
    await client.evaluate(`(()=>{
      const id=PdPairedReview.state.stratum;const path=PdPairedReview.data.missions.find(m=>m.path.includes(id)).path;
      document.querySelector('[data-paired-group="'+id+'"] .expander').click();
      document.querySelector('[data-paired-mission]').click();
      document.querySelector('[data-paired-group="'+path[1]+'"] .expander').click();
      if(PdPairedReview.state.stratum!==id||!PdPairedReview.opened.has(path[0])||PdPairedReview.opened.has(path[1])||PdPairedReview.opened.has(id)||document.querySelector('[data-paired-group="'+id+'"]')||document.querySelectorAll('[data-paired-mission]').length)throw Error('Morphology collapse closed its family or reopened the selected mission');
      document.querySelector('[data-paired-depth="3"]').click();
      if(!PdPairedReview.showGroupTree||!document.querySelector('[data-paired-mission]'))throw Error('Resume changed view or failed to reopen current group');
    })()`);
    await client.evaluate('document.querySelector("[data-review-tree]").click()');
    assert.equal(await client.evaluate('document.querySelectorAll("[data-paired-group]").length'),3);
    await client.evaluate(`(()=>{
      const id=PdPairedReview.state.stratum;const path=PdPairedReview.data.missions.find(m=>m.path.includes(id)).path;
      const close=id=>document.querySelector('[data-paired-group="'+id+'"] .expander').click();
      close(id);if(document.querySelectorAll('[data-paired-group]').length!==3||document.querySelectorAll('[data-paired-mission]').length)throw Error('Focused band collapse lost its hierarchy');
      close(path[1]);if(document.querySelectorAll('[data-paired-group]').length!==2||!PdPairedReview.opened.has(path[0]))throw Error('Focused morphology collapse lost its family');
      close(path[0]);if(document.querySelectorAll('[data-paired-group]').length!==1||PdPairedReview.opened.size||PdPairedReview.state.stratum!==id)throw Error('Focused family collapse lost selection');
    })()`);
    treeUrl=await client.evaluate('location.href');
    await client.send('Page.navigate',{url:treeUrl});await wait('!!window.PdPairedReview');
    assert(await client.evaluate('!PdPairedReview.showGroupTree&&PdPairedReview.opened.size===0&&document.querySelectorAll("[data-paired-group]").length===1'));
    await client.evaluate('document.querySelector("[data-paired-depth=\\"3\\"]").click();document.querySelector(".mission-browser [data-review-stratum=next]").click()');
    assert(await client.evaluate('!PdPairedReview.showGroupTree&&document.querySelectorAll("[data-paired-group]").length===3'));
    const selected=await client.evaluate(`(()=>{const d=PdPairedReview.data;const leaves=d.branches.filter(b=>!d.branches.some(c=>c.parent===b.id));const b=leaves.map(b=>({id:b.id,count:d.missions.filter(m=>m.path.includes(b.id)).length})).sort((a,b)=>b.count-a.count)[0];PdPairedReview.focus(b.id,'0');document.querySelector('[data-paired-mission]').scrollIntoView({block:'start'});return b;})()`);
    assert(await client.evaluate('document.querySelector("[data-review-scope=clear]")&&document.querySelector(".mission-browser [data-review-stratum=next]").disabled'));
    await client.evaluate('document.querySelector("[data-review-scope=clear]").click()');
    assert.equal(await client.evaluate('PdPairedReview.state.stratum'),selected.id);
    assert.equal(await client.evaluate('PdPairedReview.state.group'),'');
    assert.equal(await client.evaluate('document.querySelectorAll("[data-paired-mission]").length'),Math.min(12,selected.count));
    await wait('[...document.querySelectorAll(".mission-preview img")].slice(0,2).every(i=>i.complete&&i.naturalWidth>0)');
    await noOverflow();await screenshot(`${size.name}-terrain-band.png`);
    await client.evaluate('[...document.querySelectorAll("[data-paired-mission]")].at(-1).scrollIntoView({block:"center"})');
    assert(await client.evaluate('(()=>{const n=document.querySelector(".mission-browser");const r=n.getBoundingClientRect();return r.top>=0&&r.bottom<innerHeight&&getComputedStyle(n).position==="sticky"&&!["transparent","rgba(0, 0, 0, 0)"].includes(getComputedStyle(n).backgroundColor);})()'));
    await screenshot(`${size.name}-sticky-navigation.png`);
    if(selected.count>12){
      await client.evaluate('document.querySelector(".mission-browser [data-review-page=next]").click()');
      assert.equal(await client.evaluate('PdPairedReview.pageInfo.page'),2);
      await client.evaluate('document.querySelector("[data-paired-depth=\\"3\\"]").click()');
      assert.equal(await client.evaluate('PdPairedReview.pageInfo.page'),2);
      assert.equal(await client.evaluate('PdPairedReview.state.stratum'),selected.id);
      await client.evaluate(`document.querySelector('[data-paired-group="'+PdPairedReview.state.stratum+'"] .expander').click();document.querySelector('[data-paired-group="'+PdPairedReview.state.stratum+'"] .expander').click()`);
      assert.equal(await client.evaluate('PdPairedReview.pageInfo.page'),2);
      const url=await client.evaluate('location.href');await client.send('Page.navigate',{url});await wait('!!window.PdPairedReview&&PdPairedReview.pageInfo.page===2');
      assert(await client.evaluate('document.querySelectorAll("[data-paired-mission]").length<=12'));
    }
    await client.evaluate('document.getElementById("review-direction").value="-1";document.getElementById("review-direction").dispatchEvent(new Event("change"))');
    assert(await client.evaluate('[...document.querySelectorAll("[data-paired-mission]")].every(r=>r.dataset.direction==="-1")'));
    await client.evaluate('document.getElementById("review-direction").value="all";document.getElementById("review-direction").dispatchEvent(new Event("change"))');
    await client.evaluate(`PdPairedReview.focus(${JSON.stringify(selected.id)},'0')`);
    await client.evaluate('document.querySelector("[data-paired-mission]").click();document.querySelector(".mission-comparison").open=true;document.querySelector(".paired-preview-row").scrollIntoView({block:"start"})');
    await client.evaluate('document.querySelector("[data-review-tree]").click()');
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
    assert(await client.evaluate('PdPairedReview.showGroupTree'));
    await noOverflow();
    checks.push({viewport:size.name,largest_band:selected.count,materialized_bound:12,pagination_exercised:selected.count>12,groups_visited:traversal.visited.length,maximum_visible_group_rows:traversal.maximumGroups,group_controls_with_active_missions:true,sticky_navigation_readable:true,footer_group_navigation:true,full_group_tree_available:true,group_navigation_preserves_view:true,collapse_preserves_parents_and_selection:true,collapsed_state_and_view_survive_reload:true,collapse_preserves_page:true,resume_preserves_page:true,three_level_geometry:true,detail_static_and_rich:true,detail_back_context:true,detail_return_preserves_view:true});
  }
  assert.deepEqual(client.errors,[]);
  const result={schema_version:1,passed:true,report:report.href,shared_preview_bounds_checked:bounds.size,checks,screenshots,errors:client.errors};
  writeFileSync(join(output,'browser-result.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(result,null,2));
} finally {
  await browser.send('Target.closeTarget',{targetId}).catch(()=>{});
  socket.close();browserSocket.close();
}
