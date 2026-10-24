// Browser-owned adapter; no discovery, server changes, flight or report writes.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {join} from 'node:path';

async function ready(client,url,rich=false){
  for(let i=0;i<250;i++){
    if(await client.evaluate(`location.pathname===${JSON.stringify(new URL(url).pathname)}&&document.readyState==='complete'&&${rich?'Boolean(document.getElementById("chart-spatial")?._fullLayout)&&Boolean(document.querySelector(".review-detail-nav")?.dataset.contextReady)':'Boolean(window.PdStudyReview)&&Boolean(window.PdPairedReview)'}`))return;
    await new Promise(r=>setTimeout(r,50));
  }
  throw Error('Study page readiness: '+url);
}
async function shot(client,path){
  // Lazy previews must be decoded and painted before accepting a screenshot.
  await client.evaluate(`(async()=>{
    const visible=[...document.images].filter(i=>{const r=i.getBoundingClientRect();return r.bottom>0&&r.top<innerHeight&&r.right>0&&r.left<innerWidth});
    await Promise.all(visible.map(i=>Promise.race([i.decode(),new Promise((_,reject)=>setTimeout(()=>reject(Error('Preview decode timeout')),8000))])));
    if(document.fonts)await document.fonts.ready;
    await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
    return visible.length;
  })()`);
  const {data}=await client.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});writeFileSync(path,Buffer.from(data,'base64'),{flag:'wx'});
}

export async function checkStudyViewport(client,{url,kind,output,name,width}){
  await client.send('Page.navigate',{url});await ready(client,url);
  const initial=await client.evaluate('({missions:PdPairedReview.data.missions.length,claims:document.getElementById("study-fixed-claims").textContent,scope:document.getElementById("study-scope").textContent})');
  assert.equal(initial.missions,kind==='timing'?445:600);
  assert(await client.evaluate(`document.documentElement.scrollWidth<=${width+2}`));
  await shot(client,join(output,kind+'-'+name+'-overview.png'));
  let detail;
  if(kind==='rl'){
    const counts=await client.evaluate('PdStudyReviewModel.summary(PdPairedReview.data.missions,PdPairedReview.state,PdStudyProjection).counts');
    assert.equal(JSON.stringify(counts),JSON.stringify({preserved:443,gained:146,lost:3,missed:8,unpaired:0}));
    await client.evaluate('document.querySelector("[data-study-change=lost]").click()');
    assert(await client.evaluate('JSON.stringify(PdStudyReview.selected().map(m=>m.mission_index))===JSON.stringify([355,436,514])'));
    assert.equal(await client.evaluate('document.getElementById("study-fixed-claims").textContent'),initial.claims);
    assert(await client.evaluate('document.querySelector("[data-study-tree-context]").textContent.includes("Lost landings")'));
    await shot(client,join(output,kind+'-'+name+'-lost.png'));
    detail=await client.evaluate('document.querySelector(".mission-preview").href');
    await client.send('Page.navigate',{url:detail});await ready(client,detail,true);
    assert(await client.evaluate('document.querySelector(".study-detail-context").textContent.includes("Heuristic")&&document.querySelector(".study-detail-context").textContent.includes("lost")'));
    const next=await client.evaluate('document.querySelector("[data-review-next]").href');
    await client.send('Page.navigate',{url:next});await ready(client,next,true);
    assert(await client.evaluate('document.querySelector("[data-review-previous]").href.includes("change%3Dlost")'));
    const back=await client.evaluate('document.querySelector("[data-back-review]").href');
    await client.send('Page.navigate',{url:back});await ready(client,back);
    assert(await client.evaluate('PdPairedReview.state.change==="lost"&&PdPairedReview.state.contrast==="heuristic-rl"&&JSON.stringify(PdStudyReview.selected().map(m=>m.mission_index))===JSON.stringify([355,436,514])'));
    await client.evaluate('PdStudyReview.apply({change:"all",lo:"",hi:"",cell:""})');
  }else{
    assert(await client.evaluate('PdStudyReview.selected().length===300&&PdPairedReview.state.contrast==="expanded"'));
    assert(await client.evaluate('PdStudyProjection.performance.every(p=>p.originals===12&&p.measured_repeats===7&&p.values.length===84)'));
    await client.evaluate('PdStudyReview.apply({lo:"0",hi:"0",change:"all"},true)');
    assert.equal(await client.evaluate('PdStudyReview.selected().length'),72);
    assert.equal(await client.evaluate('document.getElementById("study-fixed-claims").textContent'),initial.claims);
    await client.evaluate('PdStudyReview.apply({cohort:"regressions",lo:"",hi:"",change:"all"},true)');
    assert(await client.evaluate('PdStudyReview.selected().length===21&&PdStudyReviewModel.summary(PdPairedReview.data.missions,PdPairedReview.state,PdStudyProjection).counts.unpaired===21'));
    await client.evaluate('document.getElementById("review-view").value="0";document.getElementById("review-view").dispatchEvent(new Event("change"))');
    assert(await client.evaluate('document.querySelector("[data-paired-mission]").textContent.includes("Not scheduled")'));
    await client.evaluate('PdStudyReview.apply({cohort:"primary",lo:"",hi:"",change:"all"},false)');
    const mission=await client.evaluate('PdPairedReview.data.missions.find(m=>m.mission_index===0&&PdStudyReviewModel.pair(m,PdPairedReview.state,PdStudyProjection).inCohort).id');
    await client.evaluate(`PdStudyReview.apply({lo:"",hi:"",change:"all"},true,${JSON.stringify(mission)})`);
    for(let i=0;i<100;i++){
      if(await client.evaluate('document.querySelectorAll("#study-explanation rect").length===132'))break;
      await new Promise(r=>setTimeout(r,50));
    }
    assert.equal(await client.evaluate('document.querySelectorAll("#study-explanation rect").length'),132);
    await client.evaluate('document.getElementById("study-explanation").scrollIntoView({block:"center"})');
    await shot(client,join(output,kind+'-'+name+'-explanation.png'));
    detail=await client.evaluate('document.querySelector(".mission-preview").href');
    await client.send('Page.navigate',{url:detail});await ready(client,detail,true);
    assert(await client.evaluate('document.querySelector(".study-detail-context").textContent.includes("63 pairs")'));
    const back=await client.evaluate('document.querySelector("[data-back-review]").href');
    await client.send('Page.navigate',{url:back});await ready(client,back);
  }
  // Every scheduled original remains reachable through bounded common browsing.
  const cohorts=await client.evaluate('PdStudyProjection.cohorts.map(c=>({id:c.id,n:c.originals}))');
  let total=0;
  for(const cohort of cohorts){
    await client.evaluate(`PdStudyReview.apply({cohort:${JSON.stringify(cohort.id)},change:"all",lo:"",hi:"",cell:""},true)`);
    const groups=await client.evaluate('(()=>{const s=PdPairedReview.state;return [...new Set(PdStudyReview.selected().map(m=>(m.path||[m.group]).at(-1)))]})()');
    const seen=[];
    for(const group of groups){
      await client.evaluate(`document.querySelector('.mission-browser [data-review-stratum-select]').value=${JSON.stringify(group)};document.querySelector('.mission-browser [data-review-stratum-select]').dispatchEvent(new Event('change',{bubbles:true}))`);
      seen.push(...await client.evaluate('(()=>{const seen=[];for(;;){seen.push(...[...document.querySelectorAll("[data-paired-mission]")].map(e=>e.dataset.pairedMission));const next=document.querySelector(".mission-browser [data-review-page=next]");if(next.disabled)break;next.click()}return seen})()'));
    }
    assert.equal(seen.length,cohort.n);assert.equal(new Set(seen).size,cohort.n);total+=seen.length;
  }
  await client.send('Page.navigate',{url:detail});await ready(client,detail,true);
  const graphics={};
  for(const mode of ['speed','throttle']){
    await client.evaluate(`document.querySelector('[data-mode="${mode}"]').click()`);
    const measure=await client.evaluate('(()=>{const c=document.getElementById("chart-spatial"),p=[...c.querySelectorAll(".scatterlayer .js-line")].filter(p=>getComputedStyle(p).strokeWidth==="4.5px");return {traces:c.data.length,positive:p.filter(p=>p.getTotalLength()>0).length,colours:new Set(p.map(p=>getComputedStyle(p).stroke)).size}})()');
    assert(measure.traces<100&&measure.positive>0&&measure.colours>0);graphics[mode]=measure;
  }
  await client.evaluate('document.querySelector("[data-mode=vectors]").click();document.getElementById("chart-spatial").scrollIntoView({block:"start"})');
  const arrows=await client.evaluate('(()=>{const a=document.getElementById("chart-spatial").layout.annotations;return {count:a.length,max:Math.max(0,...a.map(v=>Math.hypot(v.ax,v.ay))),pixels:a.every(v=>v.axref==="pixel"&&v.ayref==="pixel")}})()');
  assert(arrows.count<=18&&arrows.max<=24.001&&arrows.pixels);graphics.vectors=arrows;
  assert(await client.evaluate(`document.documentElement.scrollWidth<=${width+2}`));
  await shot(client,join(output,kind+'-'+name+'-detail.png'));
  return {kind,viewport:name,original_occurrences:total,all_occurrences_browsed:true,fixed_claims:true,context_roundtrip:true,graphics,overflow:false};
}
