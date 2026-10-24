// Opt-in report QA against an explicitly owned disposable Chrome.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {resolve} from 'node:path';
import {cdpClient,captureScreenshot,checkFlightInteractions} from '../../scripts/planner-v2-browser-helpers.mjs';

const root=resolve(process.argv[2]);
const output=resolve(process.argv[3]);mkdirSync(output);
const reportUrl=process.argv[4];assert(reportUrl,'Provide the exact report URL');
const owned=JSON.parse(readFileSync(root+'/browser-qa/browser.json','utf8'));
assert(owned.owned_disposable_browser);
assert.equal(owned.profile,root+'/browser-qa/profile');
const port=readFileSync(owned.profile+'/DevToolsActivePort','utf8').trim().split('\n')[0];
const cdp='http://127.0.0.1:'+port;
const target=await fetch(cdp+'/json/new?about:blank',{method:'PUT'}).then(r=>r.json());
const ws=new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r,j)=>{ws.addEventListener('open',r,{once:true});ws.addEventListener('error',j,{once:true})});
const c=cdpClient(ws);
for(const d of ['Page','Runtime','Network','Log'])await c.send(d+'.enable');
const ready=async(url,rich=false)=>{
 for(let i=0;i<300;i++){
  if(await c.evaluate(`location.pathname===${JSON.stringify(new URL(url).pathname)}&&document.readyState==='complete'&&${rich?'typeof Plotly!=="undefined"&&Boolean(document.getElementById("chart-spatial")?._fullLayout)&&Boolean(document.getElementById("chart-metrics")?._fullLayout)':'Boolean(window.PdPairedReview)'}`)){
   await c.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');return;
  }
  await new Promise(r=>setTimeout(r,50));
 }
 throw Error('page readiness '+url);
};
const checks=[];
try{
 for(const size of [{name:'desktop',width:1440,height:1000,mobile:false},{name:'phone',width:390,height:844,mobile:true}]){
  await c.send('Emulation.setDeviceMetricsOverride',{...size,deviceScaleFactor:1});
  await c.send('Page.navigate',{url:reportUrl});await ready(reportUrl);
  const counts=await c.evaluate('({missions:PdPairedReview.data.missions.length,flights:PdPairedReview.data.missions.reduce((n,m)=>n+m.attempts.length,0),groups:PdPairedReview.data.groups})');
  assert.equal(counts.missions,60);assert.equal(counts.flights,60);
  const choices=await c.evaluate('document.getElementById("actuation-case").options.length');assert.equal(choices,219);
  await c.evaluate('document.getElementById("actuation-case").value="core-arrival-109-p35-v70";document.getElementById("actuation-case").dispatchEvent(new Event("change"))');
  assert.equal(await c.evaluate('document.querySelectorAll("#actuation-results tr").length'),2);
  assert(await c.evaluate('document.querySelector("#actuation-results").textContent.includes("plane_miss")'));
  await c.evaluate('document.getElementById("actuation-case").value="witness-vertical-30-+1";document.getElementById("actuation-case").dispatchEvent(new Event("change"))');
  assert.equal(await c.evaluate('document.querySelectorAll("#actuation-results tr").length'),3);
  assert(await c.evaluate('document.querySelector("#actuation-results").textContent.includes("plane_miss")'));
  assert(await c.evaluate(`document.querySelector('img[src="actuation-calibration.svg"]').naturalWidth>0`));
  assert(await c.evaluate('document.body.textContent.includes("Broad arrival-plus-landing qualification does not pass")'));
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
  assert.equal(new Set(details).size,60);
  // Every rich occurrence must be linked and served, even when only four
  // representative details are exercised through Plotly in each viewport.
  for(const href of details){const response=await fetch(new URL(href,reportUrl));assert.equal(response.status,200)}
  for(const index of [0,2,22,59]){
   const url=new URL(details[index],reportUrl).href;await c.send('Page.navigate',{url});await ready(url,true);
   await checkFlightInteractions(c,{correction_count:0},'Actuation rich flight');
   assert(await c.evaluate(`document.documentElement.scrollWidth<=${size.width+2}`));
   assert(await c.evaluate('Boolean(document.querySelector("[data-back-review]"))'));
   assert(await c.evaluate('document.body.textContent.includes("Local arrival outcome")'));
   await captureScreenshot(c,output,`${size.name}-flight-${index}.png`);
  }
  checks.push({viewport:size.name,...counts,choice_inputs:choices,all_missions_browsed:true,all_rich_urls_served:true,rich_details_exercised:4,rich_plotly:true});
 }
 assert.deepEqual(c.errors,[]);
 writeFileSync(output+'/result.json',JSON.stringify({passed:true,url:reportUrl,checks,errors:c.errors},null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify({passed:true,checks:checks.map(x=>({viewport:x.viewport,missions:x.missions,flights:x.flights}))}));
}catch(error){writeFileSync(output+'/fault.json',JSON.stringify({error:String(error),errors:c.errors},null,2)+'\n',{flag:'wx'});throw error}
finally{ws.close();await fetch(cdp+'/json/close/'+target.id)}
