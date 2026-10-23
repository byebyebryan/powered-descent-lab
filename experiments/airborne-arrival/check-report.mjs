// Opt-in QA against an explicitly owned disposable Chrome and existing server.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {resolve} from 'node:path';
import {cdpClient,captureScreenshot,checkFlightInteractions} from '../../scripts/planner-v2-browser-helpers.mjs';

const root=resolve(process.argv[2]);
const output=resolve(process.argv[3]);mkdirSync(output);
const reportUrl=process.argv[4]||'http://127.0.0.1:8000/'+root.split('/outputs/')[1]+'/report/report.html';
const port=readFileSync(root+'/browser-qa/profile/DevToolsActivePort','utf8').trim().split('\n')[0];
const cdp='http://127.0.0.1:'+port;
const open=async url=>{const ws=new WebSocket(url);await new Promise((r,j)=>{ws.addEventListener('open',r,{once:true});ws.addEventListener('error',j,{once:true})});return ws};
const target=await fetch(cdp+'/json/new?about:blank',{method:'PUT'}).then(r=>r.json());
const ws=await open(target.webSocketDebuggerUrl);const c=cdpClient(ws);
for(const d of ['Page','Runtime','Network','Log'])await c.send(d+'.enable');
const ready=async(url,rich=false)=>{for(let i=0;i<300;i++){if(await c.evaluate(`location.pathname===${JSON.stringify(new URL(url).pathname)}&&document.readyState==='complete'&&${rich?'typeof Plotly!=="undefined"&&Boolean(document.getElementById("chart-spatial")?._fullLayout)':'Boolean(window.PdPairedReview)'}`))return;await new Promise(r=>setTimeout(r,50))}throw Error('page readiness '+url)};
const checks=[];
try{
 for(const size of [{name:'desktop',width:1440,height:1000,mobile:false},{name:'phone',width:390,height:844,mobile:true}]){
  await c.send('Emulation.setDeviceMetricsOverride',{...size,deviceScaleFactor:1});
  await c.send('Page.navigate',{url:reportUrl});await ready(reportUrl);
  const counts=await c.evaluate('({missions:PdPairedReview.data.missions.length,flights:PdPairedReview.data.missions.reduce((n,m)=>n+m.attempts.length,0),groups:PdPairedReview.data.groups})');
  const choices=await c.evaluate('document.getElementById("arrival-case").options.length');assert(choices>=73);
  await c.evaluate('document.getElementById("arrival-case").value="Core factorial / arrival-100";document.getElementById("arrival-case").dispatchEvent(new Event("change"))');
  assert.equal(await c.evaluate('document.querySelectorAll("#arrival-comparison tr").length'),7);
  assert(await c.evaluate('document.querySelector("#arrival-comparison").textContent.includes("expired")'));
  await c.evaluate(`document.querySelector('[data-paired-depth="3"]').click()`);
  const seen=[];
  for(const g of counts.groups){
   await c.evaluate(`document.querySelector('.mission-browser [data-review-stratum-select]').value=${JSON.stringify(g.id)};document.querySelector('.mission-browser [data-review-stratum-select]').dispatchEvent(new Event('change',{bubbles:true}))`);
   seen.push(...await c.evaluate('(()=>{const ids=[];for(;;){ids.push(...[...document.querySelectorAll("[data-paired-mission]")].map(e=>e.dataset.pairedMission));const b=document.querySelector(".mission-browser [data-review-page=next]");if(b.disabled)break;b.click()}return ids})()'));
  }
  assert.equal(seen.length,counts.missions);assert.equal(new Set(seen).size,counts.missions);
  assert(await c.evaluate(`document.documentElement.scrollWidth<=${size.width+2}`));
  await captureScreenshot(c,output,size.name+'-tree.png');
  const details=await c.evaluate('PdPairedReview.data.missions.flatMap(m=>m.attempts.map(a=>a.href)).filter(Boolean)');
  for(const href of [details[0],details.at(-1)]){
   const url=new URL(href,reportUrl).href;await c.send('Page.navigate',{url});await ready(url,true);
   await checkFlightInteractions(c,{correction_count:0},'Arrival rich flight');
   assert(await c.evaluate(`document.documentElement.scrollWidth<=${size.width+2}`));
   assert(await c.evaluate('Boolean(document.querySelector("[data-back-review]"))'));
  }
  await captureScreenshot(c,output,size.name+'-rich-flight.png');
  checks.push({viewport:size.name,...counts,choice_inputs:choices,all_missions_browsed:true,rich_plotly:true});
 }
 assert.deepEqual(c.errors,[]);
 writeFileSync(output+'/result.json',JSON.stringify({passed:true,url:reportUrl,checks,errors:c.errors},null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify({passed:true,checks:checks.map(x=>({viewport:x.viewport,missions:x.missions,flights:x.flights}))}));
}catch(error){writeFileSync(output+'/fault.json',JSON.stringify({error:String(error),errors:c.errors},null,2)+'\n',{flag:'wx'});throw error}
finally{ws.close();await fetch(cdp+'/json/close/'+target.id)}
