// Opt-in QA uses only a caller-owned disposable browser and existing report URL.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {resolve} from 'node:path';
import {cdpClient,captureScreenshot,checkFlightInteractions} from '../../scripts/planner-v2-browser-helpers.mjs';

const root=resolve(process.argv[2]),output=resolve(process.argv[3]),url=process.argv[4];
assert(url,'Provide the exact report URL');mkdirSync(output);
const owned=JSON.parse(readFileSync(root+'/browser-qa/browser.json','utf8'));
assert(owned.owned_disposable_browser);assert.equal(owned.profile,root+'/browser-qa/profile');
const presentation=JSON.parse(readFileSync(root+'/report-delivery/presentation-receipt.json','utf8'));
const port=readFileSync(owned.profile+'/DevToolsActivePort','utf8').trim().split('\n')[0],cdp='http://127.0.0.1:'+port;
const target=await fetch(cdp+'/json/new?about:blank',{method:'PUT'}).then(r=>r.json());
const ws=new WebSocket(target.webSocketDebuggerUrl);await new Promise((r,j)=>{ws.addEventListener('open',r,{once:true});ws.addEventListener('error',j,{once:true})});
const c=cdpClient(ws);for(const d of ['Page','Runtime','Network','Log'])await c.send(d+'.enable');
const ready=async(href,rich=false)=>{for(let i=0;i<300;i++){
 if(await c.evaluate(`location.pathname===${JSON.stringify(new URL(href).pathname)}&&document.readyState==='complete'&&${rich?'typeof Plotly!=="undefined"&&Boolean(document.getElementById("chart-spatial")?._fullLayout)&&Boolean(document.getElementById("chart-metrics")?._fullLayout)':'Boolean(window.PdPairedReview)'}`)){
  await c.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');return;
 }await new Promise(r=>setTimeout(r,50));}throw Error('page readiness '+href)};
const checks=[];
try{
 for(const v of [{name:'desktop',width:1440,height:1000,mobile:false},{name:'phone',width:390,height:844,mobile:true}]){
  await c.send('Emulation.setDeviceMetricsOverride',{...v,deviceScaleFactor:1});await c.send('Page.navigate',{url});await ready(url);
  const counts=await c.evaluate('({missions:PdPairedReview.data.missions.length,flights:PdPairedReview.data.missions.reduce((n,m)=>n+m.attempts.length,0),groups:PdPairedReview.data.groups})');
  assert.equal(counts.missions,presentation.rich_captures);assert.equal(counts.flights,presentation.rich_captures);
  assert.equal(await c.evaluate('document.getElementById("composition-state").options.length'),presentation.comparison_states);
  await c.evaluate('document.getElementById("composition-state").value="composition-confirmation-000";document.getElementById("composition-state").dispatchEvent(new Event("change"))');
  assert.equal(await c.evaluate('document.querySelectorAll("#composition-results tr").length'),3);
  assert.equal(await c.evaluate('document.querySelectorAll("#composition-results a").length'),3);
  assert(await c.evaluate('document.querySelector("#composition-results").textContent.includes("verified landing")'));
  assert(await c.evaluate(`document.querySelector('img[src="conditional-fuel-headroom.svg"]').naturalWidth>0`));
  assert(await c.evaluate('document.body.textContent.includes("120/120")&&document.body.textContent.includes("do not prescribe arrival time")'));
  await c.evaluate(`document.querySelector('[data-paired-depth="3"]').click()`);
  const seen=[];
  for(const g of counts.groups){
   await c.evaluate(`document.querySelector('.mission-browser [data-review-stratum-select]').value=${JSON.stringify(g.id)};document.querySelector('.mission-browser [data-review-stratum-select]').dispatchEvent(new Event('change',{bubbles:true}))`);
   seen.push(...await c.evaluate('(()=>{const ids=[];for(;;){ids.push(...[...document.querySelectorAll("[data-paired-mission]")].map(e=>e.dataset.pairedMission));const next=document.querySelector(".mission-browser [data-review-page=next]");if(next.disabled)break;next.click()}return ids})()'));
  }
  assert.equal(seen.length,counts.missions);assert.equal(new Set(seen).size,counts.missions);
  // Intersection-based preview loading needs actual scrolling; a full-page
  // screenshot alone does not bring offscreen mission previews into view.
  const previews=await c.evaluate('document.querySelectorAll("[data-paired-mission] img").length');assert(previews>0);
  for(let index=0;index<previews;index++){
   await c.evaluate(`document.querySelectorAll('[data-paired-mission] img')[${index}].scrollIntoView({block:'center'})`);
   let loaded=false;for(let attempt=0;attempt<100;attempt++){
    loaded=await c.evaluate(`document.querySelectorAll('[data-paired-mission] img')[${index}].naturalWidth>0`);
    if(loaded)break;await new Promise(r=>setTimeout(r,50));
   }assert(loaded,'visible mission preview did not load');
  }
  await c.evaluate('window.scrollTo(0,0)');
  assert(await c.evaluate(`document.documentElement.scrollWidth<=${v.width+2}`));await captureScreenshot(c,output,v.name+'-tree.png');
  const details=await c.evaluate('PdPairedReview.data.missions.flatMap(m=>m.attempts.map(a=>a.href)).filter(Boolean)');assert.equal(new Set(details).size,244);
  for(const href of details){const response=await fetch(new URL(href,url));assert.equal(response.status,200)}
  for(const index of [0,2,44,64,65,66,243]){
   const href=new URL(details[index],url).href;await c.send('Page.navigate',{url:href});await ready(href,true);
   await checkFlightInteractions(c,{correction_count:0},'Composed rich flight');
   assert(await c.evaluate('document.body.textContent.includes("Arrival and landing execution")'));
   for(const mode of ['throttle','guidance','vectors','mission'])await c.evaluate(`document.querySelector('[data-mode="${mode}"]').click()`);
   assert(await c.evaluate(`document.documentElement.scrollWidth<=${v.width+2}`));
   assert(await c.evaluate('Boolean(document.querySelector("[data-back-review]"))'));
   if([44,65,66].includes(index))await captureScreenshot(c,output,`${v.name}-flight-${index}.png`);
  }
  const first=new URL(details[0],url).href;await c.send('Page.navigate',{url:first});await ready(first,true);
  assert(await c.evaluate('!document.querySelector("[data-review-next]").hidden'));
  const next=await c.evaluate('document.querySelector("[data-review-next]").href');await c.evaluate('document.querySelector("[data-review-next]").click()');await ready(next,true);
  await c.evaluate('document.querySelector("[data-back-review]").click()');await ready(url);
  checks.push({viewport:v.name,...counts,comparison_states:presentation.comparison_states,all_occurrences_browsed:true,visible_previews_loaded:previews,all_rich_urls_served:true,rich_details_exercised:7,plot_modes_and_navigation:true});
 }
 assert.deepEqual(c.errors,[]);writeFileSync(output+'/result.json',JSON.stringify({passed:true,url,checks,errors:c.errors},null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify({passed:true,checks:checks.map(x=>({viewport:x.viewport,flights:x.flights,comparison_states:x.comparison_states}))}));
}catch(e){writeFileSync(output+'/fault.json',JSON.stringify({error:String(e),errors:c.errors},null,2)+'\n',{flag:'wx'});throw e}
finally{ws.close();await fetch(cdp+'/json/close/'+target.id)}
