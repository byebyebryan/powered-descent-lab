import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

const context=vm.createContext({});
vm.runInContext(readFileSync(new URL('../pd-report/src/paired_model.js',import.meta.url),'utf8'),context);
const {matches}=context.PdPairedReviewModel;
const mission={id:'mission-0',mission_index:5,label:'Value noise · left · stratum 0',group:'coverage-0',physical_sha256:'abcdef01234',attempts:[{approach:0,outcome:'timed-out'},{approach:1,outcome:'landed'},{approach:2,outcome:'unsupported'}]};
const all={q:'',group:'',outcome:'all',approach:'all'};

test('coverage focus and selected outcome approach are exact intersections',()=>{
  assert(matches(mission,all));
  assert(matches(mission,{...all,group:'coverage-0',outcome:'landed',approach:'1'}));
  assert(!matches(mission,{...all,group:'coverage-1',outcome:'landed',approach:'1'}));
  assert(!matches(mission,{...all,outcome:'landed',approach:'0'}));
  assert(matches(mission,{...all,outcome:'unsupported'}));
  assert(matches(mission,{...all,outcome:'misses',approach:'0'}));
  assert(!matches(mission,{...all,outcome:'misses',approach:'1'}));
});

test('numeric search identifies a mission; identity and terrain search stay literal',()=>{
  assert(matches(mission,{...all,q:'5'}));
  assert(!matches(mission,{...all,q:'0'}));
  assert(matches(mission,{...all,q:'ABCDEF'}));
  assert(matches(mission,{...all,q:'value noise · left'}));
  assert(!matches(mission,{...all,q:'<script>'}));
});

test('selection retains repeat occurrences and leaves the paired attempts intact',()=>{
  const repeat={...mission,id:'mission-1',occurrence:1};
  const originals=JSON.stringify([mission,repeat]);
  const selected=[mission,repeat].filter(m=>matches(m,{...all,outcome:'timed-out',approach:'0'}));
  assert.equal(selected.length,2);
  assert.equal(selected[0].attempts.length,3);
  assert.equal(JSON.stringify([mission,repeat]),originals);
});

test('detail navigation rejects an external return URL and filters neighbours',async()=>{
  const links=Object.fromEntries(['previous','next'].map(k=>[k,{hidden:true}]));
  const back={};const nav={dataset:{},querySelector:s=>links[s.includes('previous')?'previous':'next']};
  const m={...mission,attempts:mission.attempts.map(a=>({...a,href:`details/mission-0-a${a.approach}.html`}))};
  const neighbour={...m,id:'mission-2',mission_index:13,attempts:m.attempts.map(a=>({...a,href:`details/mission-2-a${a.approach}.html`}))};
  const excluded={...neighbour,id:'mission-1',group:'coverage-1'};
  const run=async review=>{
    const body={dataset:{reviewRoot:'/edition/',reviewMission:'mission-0',reviewApproach:'0'}};
    const ctx=vm.createContext({URL,URLSearchParams,location:{origin:'http://localhost:8000',search:'?review='+encodeURIComponent(review)},document:{body,querySelector:s=>s==='[data-back-review]'?back:nav},fetch:async()=>({ok:true,json:async()=>({schema_version:1,missions:[m,excluded,neighbour]})})});
    vm.runInContext(readFileSync(new URL('../pd-report/src/paired_model.js',import.meta.url),'utf8'),ctx);
    vm.runInContext(readFileSync(new URL('../pd-report/src/detail_navigation.js',import.meta.url),'utf8'),ctx);
    await new Promise(r=>setImmediate(r));
  };
  await run('/edition/report.html?group=coverage-0&approach=0&outcome=timed-out');
  assert.equal(nav.dataset.contextReady,'true');
  assert(links.previous.hidden);
  assert.equal(new URL(links.next.href,'http://localhost').pathname,'/edition/details/mission-2-a0.html');
  assert(new URL(links.next.href,'http://localhost').searchParams.get('review').includes('group=coverage-0'));
  await run('https://outside.example/report.html');
  assert.equal(back.href,'/edition/report.html#mission-0');
});
