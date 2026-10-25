// Browser-owned client adapter; no service discovery or evaluation execution.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {checkStudyViewport} from './check-study-report.mjs';

async function ready(client,kind){
  for(let i=0;i<300;i++){
    const expression=kind==='pair'?'Boolean(document.body.dataset.pairReady)':kind==='rich'?'Boolean(document.getElementById("chart-spatial")?._fullLayout)&&Boolean(document.querySelector(".review-detail-nav")?.dataset.contextReady)':'Boolean(window.PdStudyReview)&&Boolean(window.PdPairedReview)';
    if(await client.evaluate(expression))return;
    await new Promise(r=>setTimeout(r,50));
  }
  throw Error('View readiness failed: '+kind);
}
async function paint(client){await client.evaluate('(async()=>{await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true})()');}
async function screenshot(client,path){await paint(client);const r=await client.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});writeFileSync(path,Buffer.from(r.data,'base64'),{flag:'wx'});}

export async function checkRefinement(client,{url,kind,output,name,width}){
  const common=await checkStudyViewport(client,{url,kind,output,name,width});
  const navigate=async(href,view='study')=>{await client.send('Page.navigate',{url:href});await ready(client,view);};
  await navigate(url);
  assert.equal(await client.evaluate('performance.getEntriesByType("resource").filter(r=>r.name.includes("/flights-")).length'),0);
  const frozen=await client.evaluate('document.getElementById("study-fixed-claims").textContent');
  if(kind==='timing'){
    await client.evaluate('PdStudyReview.apply({metric_change:"worse"})');assert.equal(await client.evaluate('PdStudyReview.selected().length'),0);
    await client.evaluate('PdStudyReview.apply({metric_change:"tie"})');assert.equal(await client.evaluate('PdStudyReview.selected().length'),72);
    await client.evaluate('PdStudyReview.apply({metric_change:"all",target_status:"unmet"})');assert.equal(await client.evaluate('PdStudyReview.selected().length'),0);
  }else{
    await client.evaluate('PdStudyReview.apply({target_status:"unmet"})');assert.equal(await client.evaluate('PdStudyReview.selected().length'),11);
    await client.evaluate('PdStudyReview.apply({target_status:"met"})');assert.equal(await client.evaluate('PdStudyReview.selected().length'),589);
  }
  await client.evaluate('document.getElementById("study-clear").click()');assert.equal(await client.evaluate('PdStudyReview.selected().length'),kind==='timing'?300:600);
  await client.evaluate('PdStudyReview.apply({basis:"earlier"})');
  assert(await client.evaluate('PdStudyReviewModel.summary(PdPairedReview.data.missions,PdPairedReview.state,PdStudyProjection).pairs.every(p=>p.delta===0)'));
  assert(await client.evaluate('document.getElementById("comparison-reference-context").textContent.includes("zero scientific changes")'));
  assert.equal(await client.evaluate('document.getElementById("study-fixed-claims").textContent'),frozen);
  await client.evaluate('PdStudyReview.apply({basis:"strategy"})');
  const bad=new URL(url);bad.searchParams.set('contrast','not-a-declared-baseline');await navigate(bad.href);
  assert.equal(await client.evaluate('PdStudyReview.selected().length'),0);assert(await client.evaluate('document.getElementById("study-scope").textContent.includes("unavailable")'));
  await navigate(url);
  const trials=[];
  await client.send('Page.bringToFront',{});
  for(let i=0;i<5;i++)trials.push(await client.evaluate('(async()=>{const t=performance.now();PdStudyReview.apply({metric_change:"all",target_status:"all"});const rendered=performance.now();await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return {total:performance.now()-t,sync:rendered-t,visibility:document.visibilityState,focus:document.hasFocus()}})()'));
  const median=trials.map(t=>t.total).sort((a,b)=>a-b)[2];assert(median<250,'Warm filter trials '+JSON.stringify(trials));
  if(kind==='timing'){
    const id=await client.evaluate('PdPairedReview.data.missions.find(m=>m.mission_index===11&&PdStudyReviewModel.pair(m,PdPairedReview.state,PdStudyProjection).inCohort).id');
    await client.evaluate(`PdStudyReview.apply({},true,${JSON.stringify(id)})`);
    for(let i=0;i<150;i++){if(await client.evaluate('(()=>{const i=document.querySelector(".mission-preview img");return i?.complete&&i.naturalWidth>0})()'))break;await new Promise(r=>setTimeout(r,50));}
    await client.evaluate('(async()=>{await document.querySelector(".mission-preview img").decode();await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true})()');
    const alignment=await client.evaluate('(()=>{const i=document.querySelector(".mission-preview"),p=i.closest("td"),a=i.getBoundingClientRect(),b=p.getBoundingClientRect();return {offset:Math.abs((a.left+a.right-b.left-b.right)/2),loaded:i.querySelector("img").naturalWidth}})()');assert(alignment.offset<12&&alignment.loaded>0);
  }else await client.evaluate('PdStudyReview.apply({change:"lost"},true)');
  const pair=await client.evaluate('document.querySelector(".compare-pair").href');await navigate(pair,'pair');
  assert.equal(await client.evaluate('document.body.dataset.pairReady'),'true');
  assert.equal(await client.evaluate('document.querySelectorAll("#pair-inspection svg").length'),4);
  await paint(client);
  const historyPixels=await client.evaluate('[...document.querySelectorAll(".flight-histories svg")].map(s=>({width:s.getBoundingClientRect().width,viewbox:s.viewBox.baseVal.width,label_px:12*s.getBoundingClientRect().width/s.viewBox.baseVal.width,height:s.getBoundingClientRect().height}))');
  assert(historyPixels.every(h=>h.label_px>=11.9&&Math.abs(h.height-170)<1),JSON.stringify(historyPixels));
  assert(await client.evaluate(`document.documentElement.scrollWidth<=${width+2}`));
  let ended=null;
  if(kind==='timing'){
    await client.evaluate('PdPairInspector.seek(18)');ended=await client.evaluate('JSON.parse(PdPairInspector.output.dataset.snapshots)');assert(!ended[0].ended&&ended[1].ended&&ended[1].command===null);
    assert(await client.evaluate('document.querySelector("#pair-summary table").textContent.includes("-1.500")'));
  }else assert(await client.evaluate('document.getElementById("pair-summary").textContent.includes("A met, B unmet")'));
  const pairVectors=await client.evaluate('[...document.querySelectorAll(".flight-pair-panes svg")].map(s=>{const p=s.getBoundingClientRect().width/s.viewBox.baseVal.width,line=s.querySelector("line[vector-effect]"),origin=s.querySelector("circle");return {length:Math.hypot(line.x2.baseVal.value-line.x1.baseVal.value,line.y2.baseVal.value-line.y1.baseVal.value)*p,radius:origin.r.baseVal.value*p}})');assert(pairVectors.every(v=>v.length<=24.001&&Math.abs(v.radius-4)<.001),JSON.stringify(pairVectors));
  await screenshot(client,join(output,kind+'-'+name+'-paired.png'));
  const rich=await client.evaluate('[...document.querySelectorAll("#pair-summary a")].filter(a=>a.textContent.startsWith("Full rich")).at(-1).href');await navigate(rich,'rich');
  assert.equal(await client.evaluate('document.querySelectorAll("#direction-history svg").length'),2);
  await paint(client);assert(await client.evaluate('[...document.querySelectorAll("#direction-history svg")].every(s=>12*s.getBoundingClientRect().width/s.viewBox.baseVal.width>=11.9)'));
  const expectedClock=await client.evaluate('(()=>{const t=Math.min(8,reportData.samples.at(-1).simTimeS*.6),input=document.querySelector("#direction-history input");input.value=t;input.dispatchEvent(new Event("input"));return PdFlightModel.at(reportData.samples,t).sample.simTimeS.toFixed(2)})()');await paint(client);
  assert(await client.evaluate(`document.getElementById("inspect-caption").textContent.includes(${JSON.stringify(expectedClock)})`));
  await client.evaluate('document.querySelector("[data-mode=vectors]").click()');await paint(client);
  // Plotly's pixel projection is rounded; 0.02 px is well below a display pixel.
  const geometry=await client.evaluate('(()=>{const c=document.getElementById("chart-spatial"),l=c._fullLayout,a=c.layout.annotations;let matched=0;for(const v of a){const x=l.xaxis.l2p(v.x)+v.ax,y=l.yaxis.l2p(v.y)+v.ay;if(reportData.samples.some(s=>Math.hypot(l.xaxis.l2p(s.xM)-x,l.yaxis.l2p(s.yM)-y)<.02&&Math.abs(v.ax+Math.sin(s.attitudeRad)*24*s.throttleFrac)<1e-5&&Math.abs(v.ay-Math.cos(s.attitudeRad)*24*s.throttleFrac)<1e-5))matched++;}return {arrows:a.length,matched,max:Math.max(0,...a.map(v=>Math.hypot(v.ax,v.ay)))}})()');
  assert(geometry.arrows<=18&&geometry.max<=24.001&&geometry.matched===geometry.arrows,JSON.stringify(geometry));
  const pairLink=await client.evaluate('[...document.querySelectorAll(".study-detail-context a")].find(a=>a.textContent.includes("paired trajectories")).href');assert(new URL(pairLink).pathname.endsWith('/compare.html'));
  const back=await client.evaluate('document.querySelector("[data-back-review]").href');await navigate(back);
  if(kind==='rl')assert(await client.evaluate('PdPairedReview.state.change==="lost"&&PdStudyReview.selected().length===3'));
  assert.equal(await client.evaluate('document.getElementById("study-fixed-claims").textContent'),frozen);
  return {passed:true,kind,viewport:name,common,earlier_no_op:true,invalid_reference_disclosed:true,target_filters:true,pair_context:true,ended,history_pixels:historyPixels,paired_vectors:pairVectors,vector_geometry:geometry,warm_filter_ms:trials,median_filter_ms:median};
}
