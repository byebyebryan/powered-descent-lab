#!/usr/bin/env node
// Explicit local file preview QA; never starts a report server or publishes.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync, writeFileSync} from 'node:fs';
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {parseArgs} from 'node:util';
import {cdpClient} from './planner-v2-browser-helpers.mjs';

const {values} = parseArgs({options:{'browser-ws':{type:'string'},report:{type:'string'},output:{type:'string'}},strict:true});
if(!values['browser-ws']||!values.report||!values.output)throw new Error('Require --browser-ws WS --report DIR --output EXISTING_QA_DIR');
const browser = new WebSocket(values['browser-ws']);
await new Promise((r,j)=>{browser.addEventListener('open',r,{once:true});browser.addEventListener('error',j,{once:true});});
const connection=cdpClient(browser);
const {targetId}=await connection.send('Target.createTarget',{url:'about:blank'});
const port=new URL(values['browser-ws']).port;
const targets=await fetch(`http://127.0.0.1:${port}/json/list`).then(r=>r.json());
const target=targets.find(t=>t.id===targetId);
const socket=new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r,j)=>{socket.addEventListener('open',r,{once:true});socket.addEventListener('error',j,{once:true});});
const client=cdpClient(socket);
const screenshots=[];
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const pageUrl=pathToFileURL(join(resolve(values.report),'index.html')).href;
const pause=ms=>new Promise(r=>setTimeout(r,ms));
async function ready() {
  for(let i=0;i<100;i++) {
    if(await client.evaluate("document.readyState==='complete' && !!globalThis.PdInputCoverage"))return;
    await pause(50);
  }
  throw new Error('Input projection did not initialize');
}
async function screenshot(name) {
  await client.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
  const {data}=await client.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
  const bytes=Buffer.from(data,'base64');writeFileSync(join(values.output,name),bytes,{flag:'wx'});
  screenshots.push({file:name,sha256:sha(bytes)});
}
try {
  await client.send('Page.enable');await client.send('Runtime.enable');await client.send('Network.enable');await client.send('Log.enable');
  await client.send('Emulation.setDeviceMetricsOverride',{width:1440,height:1050,deviceScaleFactor:1,mobile:false});
  await client.send('Page.navigate',{url:pageUrl});await ready();
  assert.equal(await client.evaluate("document.querySelectorAll('#input-preview-cards img').length"),0);
  assert.equal(await client.evaluate("document.querySelectorAll('img').length"),0);
  assert.equal(await client.evaluate('document.documentElement.scrollWidth<=innerWidth'),true);
  await screenshot('desktop-overview.png');
  const selection=await client.evaluate(`(()=>{
    const d=PdInputCoverage.data;
    const bands=d.bands.map(b=>({key:PdInputCoverageModel.keyOf(b),count:d.previews.filter(m=>PdInputCoverageModel.keyOf(m)===PdInputCoverageModel.keyOf(b)).length})).sort((a,b)=>b.count-a.count);
    document.querySelectorAll('[data-band]').forEach(b=>{if(b.dataset.band===bands[0].key){b.closest('details').open=true;b.click();}});
    return bands[0];
  })()`);
  assert.ok(selection.count>12,'Pilot must exercise more than one preview page');
  assert.equal(await client.evaluate("document.querySelectorAll('#input-preview-cards img').length"),12);
  assert.equal(await client.evaluate('PdInputCoverage.pageInfo.total'),selection.count);
  for(let i=0;i<60;i++) {
    if(await client.evaluate("[...document.querySelectorAll('#input-preview-cards img')].slice(0,3).every(i=>i.complete&&i.naturalWidth>0)"))break;
    await pause(50);
  }
  assert.equal(await client.evaluate("[...document.querySelectorAll('#input-preview-cards img')].slice(0,3).every(i=>i.complete&&i.naturalWidth>0)"),true);
  await screenshot('desktop-preview-band.png');
  const first=await client.evaluate("[...document.querySelectorAll('#input-preview-cards a')].map(a=>a.href)");
  await client.evaluate("document.querySelector('[data-page=next]').click()");
  assert.equal(await client.evaluate('PdInputCoverage.pageInfo.page'),2);
  assert.ok(await client.evaluate("document.querySelectorAll('#input-preview-cards img').length<=12"));
  const second=await client.evaluate("[...document.querySelectorAll('#input-preview-cards a')].map(a=>a.href)");
  assert.ok(!second.some(s=>first.includes(s)));
  await client.send('Page.reload');await ready();
  assert.equal(await client.evaluate('PdInputCoverage.pageInfo.page'),2);
  assert.equal(await client.evaluate('PdInputCoverage.state.band'),selection.key);
  await client.evaluate("document.getElementById('input-direction').value='-1';document.getElementById('input-direction').dispatchEvent(new Event('change')); ");
  assert.equal(await client.evaluate('PdInputCoverage.pageInfo.missions.every(m=>m.direction===-1)'),true);
  await client.evaluate("document.getElementById('input-collapse').click()");
  assert.equal(await client.evaluate("document.querySelectorAll('#input-preview-cards img').length"),0);
  await client.send('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true});
  await client.evaluate('scrollTo(0,0)');
  assert.equal(await client.evaluate('document.documentElement.scrollWidth<=innerWidth'),true);
  await screenshot('mobile-overview.png');
  await client.evaluate(`(()=>{
    document.getElementById('input-direction').value='all';document.getElementById('input-direction').dispatchEvent(new Event('change'));
    [...document.querySelectorAll('[data-band]')].find(b=>b.dataset.band===${JSON.stringify(selection.key)}).click();
  })()`);
  await screenshot('mobile-preview-band.png');
  assert.ok(await client.evaluate("document.querySelectorAll('#input-preview-cards img').length<=12"));
  const detail=await client.evaluate("document.querySelector('#input-preview-cards a').href");
  await client.send('Page.navigate',{url:detail});
  for(let i=0;i<100;i++) {if(await client.evaluate("document.readyState==='complete'&&document.querySelector('img')?.complete"))break;await pause(50);}
  assert.equal(await client.evaluate("document.querySelector('img').naturalWidth>0"),true);
  assert.equal(await client.evaluate('document.documentElement.scrollWidth<=innerWidth'),true);
  await screenshot('mobile-input-detail.png');
  await client.evaluate("document.querySelector('nav a').click()");await ready();
  assert.equal(await client.evaluate('PdInputCoverage.state.band'),selection.key);
  assert.ok(await client.evaluate("document.querySelectorAll('#input-preview-cards img').length<=12"));
  assert.deepEqual(client.errors,[]);
  const result={schema_version:1,passed:true,report_file_sha256:sha(readFileSync(join(values.report,'index.html'))),selection,initial_preview_requests:0,maximum_materialized_previews:12,pagination_reload_direction_and_detail:true,desktop_mobile_overflow:false,screenshots,errors:client.errors};
  writeFileSync(join(values.output,'browser-result.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(result,null,2));
} finally {
  await connection.send('Target.closeTarget',{targetId}).catch(()=>{});
  socket.close();browser.close();
}
