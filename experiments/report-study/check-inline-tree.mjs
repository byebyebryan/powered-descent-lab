// Run through the browser-owned client, against a prepared saved edition.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {join} from 'node:path';

async function wait(client, expression) {
  for (let retry=0; retry<240; retry++) {
    if (await client.evaluate(expression)) return;
    await new Promise(resolve=>setTimeout(resolve,50));
  }
  throw Error('Inline review readiness: '+expression);
}

export async function checkInlineTree(client,{url,kind,width,output,name}) {
  await client.send('Page.navigate',{url});
  await wait(client,'Boolean(window.PdStudyReview)&&Boolean(window.PdMissionPresentation)');
  const lazy=await client.evaluate(`(()=>{
    PdStudyReview.apply({},true);
    return {rows:document.querySelectorAll('[data-paired-mission]').length,
      cards:document.querySelectorAll('.mission-run-card').length,
      runRows:document.querySelectorAll('[data-run-role]').length,
      flightPacks:performance.getEntriesByType('resource').filter(r=>/flights-.*json.gz/.test(r.name)).length};
  })()`);
  assert.equal(lazy.rows,12); assert.equal(lazy.cards,24); assert.equal(lazy.runRows,24);
  assert.equal(lazy.flightPacks,0);
  assert.equal(await client.evaluate('document.querySelector(".paired-table").tHead.rows[0].cells.length'),2);
  const layout=await client.evaluate(`(()=>{
    const boxes=s=>[...document.querySelectorAll(s)].map(e=>{const r=e.getBoundingClientRect();return {left:r.left,right:r.right,width:r.width}});
    return {client:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth,
      tables:boxes('.paired-table'),cards:boxes('.mission-run-card')};
  })()`);
  assert(layout.scroll<=width+2);
  if(width>=1400) assert(layout.cards.every(box=>box.width>=300));
  if(width<700) for(const box of [...layout.tables,...layout.cards]) {
    assert(box.left>=-1&&box.right<=layout.client+2,JSON.stringify(box));
  }
  const paint=await client.evaluate(`(async()=>{
    const trials=[];
    for(let i=0;i<5;i++){const start=performance.now();PdStudyReview.apply({},true);
      await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));trials.push(performance.now()-start)}
    return trials;
  })()`);
  assert([...paint].sort((a,b)=>a-b)[2]<250,JSON.stringify(paint));
  if(kind==='rl') {
    await client.evaluate('PdStudyReview.apply({change:"lost"},true,"mission-355")');
    assert(await client.evaluate(`(()=>{const m=PdPairedReview.data.missions.find(m=>m.mission_index===355),v=PdMissionPresentation.build(m,PdPairedReview.state,PdPairedReview.data,PdStudyProjection);return v.sides[1].outcome==='crashed'&&v.studyPair.eligible===false})()`));
    assert(await client.evaluate('document.querySelector(".mission-selected-delta").textContent.includes("Unavailable")'));
  } else {
    await client.evaluate('PdStudyReview.apply({cohort:"regressions",lo:"",hi:""},true)');
    assert(await client.evaluate('document.querySelectorAll(".mission-run-card").length===document.querySelectorAll("[data-paired-mission]").length*2&&document.querySelector(".mission-selected-delta").textContent.includes("Unavailable")'));
    await client.evaluate('PdStudyReview.apply({cohort:"primary",lo:"",hi:"",basis:"strategy"},true,"mission-11")');
  }
  await wait(client,'Boolean(document.querySelector(".mission-time-inspection"))');
  assert(await client.evaluate(`(()=>{const heading=document.querySelector('.mission-run-heading'),r=heading.getBoundingClientRect();return Boolean(document.elementFromPoint(r.x+Math.min(30,r.width/2),r.y+5)?.closest('.mission-run-heading'))})()`),'Sticky navigation covers the first visible run heading');
  assert(await client.evaluate('!document.querySelector(".mission-time-inspection").open&&document.querySelectorAll(".mission-inspection-host svg").length===0'));
  await client.evaluate('document.querySelector(".mission-time-inspection > summary").click()');
  await wait(client,'document.querySelectorAll(".mission-inspection-host .flight-histories svg").length===2');
  const time=kind==='rl'?6:18;
  const snapshots=await client.evaluate(`(()=>{const input=document.querySelector('.mission-inspection-host input');input.value=${time};input.dispatchEvent(new Event('input'));return JSON.parse(document.querySelector('.mission-inspection-host output').dataset.snapshots)})()`);
  assert.equal(snapshots.length,2); assert.equal(snapshots[0].ended,false); assert.equal(snapshots[1].ended,true); assert.equal(snapshots[1].command,null);
  await client.evaluate('document.querySelector(".mission-inspection-host").scrollIntoView({block:"center"})');
  if(client.screenshot)writeFileSync(join(output,kind+'-'+name+'-inline.jpg'),await client.screenshot(),{flag:'wx'});
  const pair=await client.evaluate('document.querySelector(".compare-pair").href');
  await client.evaluate('document.querySelector(".mission-time-inspection > summary").click()');
  await wait(client,'document.querySelectorAll(".mission-inspection-host svg").length===0');
  await client.send('Page.navigate',{url:pair});
  await wait(client,'document.body.dataset.pairReady==="true"');
  assert.equal(await client.evaluate('PdPairState.records'),2);
  return {passed:true,kind,viewport:name,lazy,layout,paint_ms:paint,snapshots,secondary_pair_compatible:true};
}
