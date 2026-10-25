import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../pd-report/src/portable_assets.js',import.meta.url),'utf8');
function fixture({patch=true,before='/* old display */',repeats=1,decoy=false,revision}={}){
  const payloadSuffix=',"href":"../native/evaluation.json"'+(decoy?',"note":"/* old display */"':'')+'}',payload='{"samples":[{"xM":-0}]'+payloadSuffix,prefix='<html><script>'+before.repeat(repeats)+'</script>\nconst reportData = ',suffix=';\n<a href="flight-1.html">Next</a></html>';
  const files={'asset-manifest.json':{assets:{},source_base:'http://archive:8000/native/',...(patch?{presentation_patch:'assets/patch.json'}:{})},'assets/patch.json':{version:1,before:'/* old display */',after:'/* responsive display */'},'data/report.html.gz':'<main id="review-tree-root"></main><script>/* old tree */ const decoy=\'id="pair-inspection"\';</script>','data/compare.html.gz':'<main id="pair-inspection"></main><script>/* old display */</script>','data/index.json':{cases:{0:{pack:'data/flights-0.json.gz'}}},'data/flights-0.json.gz':{lines:[prefix,suffix],vectors:['-0'],cases:{0:{before:[0],after:[1],payload:{keys:['"xM"'],columns:[[0,false]],count:1,prefix:'{"samples":',suffix:payloadSuffix}}}}};
  if(revision)files['assets/patch.json']=revision;
  let rendered='';const context=vm.createContext({URL,Uint8Array,Map,Response,location:{href:'http://reports:8787/reports/edition/detail.html'},fflate:{gunzipSync:x=>x,strFromU8:x=>new TextDecoder().decode(x)},fetch:async url=>{const key=new URL(url).pathname.split('/edition/')[1],value=files[key];return new Response(value===undefined?'Missing':typeof value==='string'?value:JSON.stringify(value),{status:value===undefined?404:200});},document:{open(){},write(value){rendered=value;},close(){}}});
  vm.runInContext(source,context);return {api:context.PdSavedAssets,rendered:()=>rendered,payload};
}
test('pinned presentation revision preserves exact native literals and rewrites only navigation',async()=>{
  const f=fixture();await f.api.showDetail('0');assert(f.rendered().includes('/* responsive display */'));assert(f.rendered().includes(f.payload));assert(f.rendered().includes('href="detail.html?case=1"'));assert((await f.api.html('0')).includes('/* old display */'));
});
test('multiple source-bound display modules respect declared views and preserve saved payload literals',async()=>{
  const revision={version:2,patches:[{id:'inspection',views:['rich','pair'],before:'/* old display */',after:'/* responsive display */'},{id:'navigation',views:['rich'],before:'>Next</a>',after:'>Next recorded</a>'}]};
  const f=fixture({revision,decoy:true});await f.api.showDetail('0');assert(f.rendered().includes(f.payload));assert(f.rendered().includes('responsive display'));assert(f.rendered().includes('Next recorded'));
  for(const changed of [{...revision,patches:[{...revision.patches[0],views:['study']}]},{...revision,patches:[...revision.patches,revision.patches[0]]},{...revision,patches:[{...revision.patches[0],views:['unknown']}]}])await assert.rejects(fixture({revision:changed}).api.showDetail('0'),/presentation patch|source binding differs/i);
});
test('unselected display patch preserves legacy transport; missing or repeated source fails closed',async()=>{
  const legacy=fixture({patch:false});await legacy.api.showDetail('0');assert(legacy.rendered().includes('/* old display */'));
  await assert.rejects(fixture({repeats:0}).api.showDetail('0'),/source binding differs/);await assert.rejects(fixture({repeats:2}).api.showDetail('0'),/source binding differs/);
});
test('display source appearing in saved scientific metadata is never patched or counted as a template',async()=>{
  const f=fixture({decoy:true});await f.api.showDetail('0');assert(f.rendered().includes(f.payload));assert(f.rendered().includes('/* responsive display */'));
  await assert.rejects(fixture({decoy:true,repeats:0}).api.showDetail('0'),/source binding differs/);
});
test('tree and paired template revisions apply only their explicitly bound module sets',async()=>{
  const f=fixture({revision:{version:2,patches:[{id:'inspection',views:['rich','pair'],before:'/* old display */',after:'/* responsive display */'},{id:'tree',views:['study'],before:'/* old tree */',after:'/* inline tree */'}]}});
  await f.api.page('report.html');assert(f.rendered().includes('/* inline tree */'));assert(!f.rendered().includes('/* responsive display */'));
  await f.api.page('compare.html');assert(f.rendered().includes('/* responsive display */'));assert(!f.rendered().includes('/* inline tree */'));
});
