// Primary-owned geometry and integration checks, run by a browser-owned client.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {join} from 'node:path';

async function wait(client, expression) {
  for (let retry=0; retry<240; retry++) {
    if(await client.evaluate(expression))return;
    await new Promise(resolve=>setTimeout(resolve,50));
  }
  throw Error('Compact review readiness: '+expression);
}

export async function checkCompactTree(client,{url,kind,width,output,name,portable=true}) {
  await client.send('Page.navigate',{url});
  await wait(client,`location.pathname===${JSON.stringify(new URL(url).pathname)}&&document.readyState==='complete'&&Boolean(window.PdStudyReview)&&Boolean(window.PdMissionPresentation)`);
  assert.equal(await client.evaluate('PdPairedReview.data.missions.length'),kind==='timing'?445:600);
  const initial=await client.evaluate(`(()=>{
    PdStudyReview.apply({},true);
    return {rows:document.querySelectorAll('[data-paired-mission]').length,
      cards:document.querySelectorAll('.mission-run-card').length,
      runRows:document.querySelectorAll('[data-run-role]').length,
      fullTables:document.querySelectorAll('.mission-run-values-full').length,
      initialFlightRequests:performance.getEntriesByType('resource').filter(r=>/flights-.*json.gz|flight-[0-9]+[.]html/.test(r.name)).length};
  })()`);
  assert.equal(initial.rows,12);assert.equal(initial.cards,24);assert.equal(initial.runRows,24);
  assert.equal(initial.fullTables,0);assert.equal(initial.initialFlightRequests,0);
  const layout=await client.evaluate(`(()=>{
    const rect=e=>{const r=e.getBoundingClientRect();return {left:r.left,right:r.right,width:r.width,height:r.height}};
    const rows=[...document.querySelectorAll('[data-paired-mission]')];
    return {client:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth,
      rows:rows.map(rect),cards:[...document.querySelectorAll('.mission-run-card')].map(rect),
      results:[...document.querySelectorAll('.mission-results')].map(rect),
      legend:document.querySelectorAll('.mission-comparison-legend').length,
      legendNames:[...document.querySelectorAll('.mission-comparison-legend span')].map(e=>e.textContent),
      overviewFieldCounts:[...document.querySelectorAll('.mission-run-values-overview thead tr')].map(r=>r.cells.length-2)};
  })()`);
  assert(layout.scroll<=layout.client+2);assert.equal(layout.legend,1);assert.equal(layout.legendNames.length,2);
  assert(layout.overviewFieldCounts.every(n=>n<=3));
  assert(await client.evaluate('[...document.querySelectorAll(".mission-run-values-overview th,.mission-run-values-overview td")].every(e=>e.scrollWidth<=e.clientWidth+3)'),'Overview labels or values overflow their columns');
  for(const box of [...layout.cards,...layout.results])assert(box.left>=-1&&box.right<=layout.client+2,JSON.stringify(box));
  if(width>=1400)assert(layout.rows.every(r=>r.height>=115&&r.height<=155),JSON.stringify(layout.rows));
  if(width<700)assert(layout.rows.every(r=>r.height<380));
  const familyStats=await client.evaluate(`(()=>{
    document.querySelector('.mission-browser [data-review-tree]').click();
    return [...document.querySelectorAll('.paired-group-row .mission-group-metric')].map(e=>({...e.dataset}));
  })()`);
  assert(familyStats.length>=4);
  if(kind==='timing'){
    assert.equal(familyStats.reduce((n,g)=>n+Number(g.eligibleN),0),300);
    const mean=familyStats.reduce((n,g)=>n+Number(g.deltaMean)*Number(g.eligibleN),0)/300;
    assert(Math.abs(mean+21.97615298709789)<1e-9);
    await client.evaluate('PdStudyReview.apply({cohort:"regressions"},true)');
    assert(await client.evaluate('document.querySelector(".mission-group-metric").dataset.eligibleN==="0"'));
    await client.evaluate('PdStudyReview.apply({cohort:"primary",basis:"strategy"},true,"mission-11")');
  }else{
    await client.evaluate('PdStudyReview.apply({change:"lost"},true,"mission-355")');
    assert(await client.evaluate('document.querySelector(".mission-result-line").textContent.includes("unavailable")'));
  }
  await wait(client,'Boolean(document.querySelector(".mission-time-inspection"))');
  assert(await client.evaluate('document.querySelectorAll(".mission-run-values-full").length===1'));
  assert(await client.evaluate('!document.querySelector(".mission-time-inspection").open&&document.querySelectorAll(".mission-inspection-host svg").length===0'));
  if(kind==='timing')assert(await client.evaluate('document.querySelector(".mission-run-values-full").textContent.includes("Incoming contact speed")'));
  await client.evaluate('document.querySelector(".mission-time-inspection > summary").click()');
  await wait(client,'document.querySelectorAll(".mission-inspection-host .flight-histories svg").length===2');
  const snapshots=await client.evaluate(`(()=>{const input=document.querySelector('.mission-inspection-host input');input.value=${kind==='timing'?18:6};input.dispatchEvent(new Event('input'));return JSON.parse(document.querySelector('.mission-inspection-host output').dataset.snapshots)})()`);
  assert.equal(snapshots[0].ended,false);assert.equal(snapshots[1].ended,true);assert.equal(snapshots[1].command,null);
  await client.evaluate('document.querySelector(".mission-time-inspection > summary").click()');
  await wait(client,'document.querySelectorAll(".mission-inspection-host svg").length===0');
  await client.evaluate('PdStudyReview.apply({cohort:PdStudyProjection.default_cohort,change:"all",q:"",lo:"",hi:""},true);document.querySelector(".mission-browser").scrollIntoView({block:"start"})');
  await client.evaluate(`(async()=>{
    for(let i=0;i<200;i++){
      const images=[...document.querySelectorAll('.mission-run-card img')].filter(e=>{const r=e.getBoundingClientRect();return r.top<innerHeight&&r.bottom>0});
      if(images.every(e=>e.src&&e.complete&&e.naturalWidth>0)){await Promise.all(images.map(e=>e.decode()));break;}
      await new Promise(r=>setTimeout(r,40));
    }
    await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
  })()`);
  const density=await client.evaluate(`(()=>{const edge=document.querySelector('.mission-browser').getBoundingClientRect().bottom;return {fullyVisible:[...document.querySelectorAll('[data-paired-mission]')].filter(e=>{const r=e.getBoundingClientRect();return r.top>=edge&&r.bottom<=innerHeight}).length,legend:document.querySelector('.mission-comparison-legend').textContent}})()`);
  if(width>=1400)assert(density.fullyVisible>=4,JSON.stringify(density));
  if(client.screenshot)writeFileSync(join(output,kind+'-'+name+'.jpg'),await client.screenshot(),{flag:'wx'});
  return {passed:true,kind,viewport:name,portable,initial,layout,familyStats,snapshots,density};
}
