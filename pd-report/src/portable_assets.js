/* Immutable same-origin transport adapter; scientific sample literals are unchanged. */
(() => {
  if(globalThis.PdSavedAssets)return;
  const root=new URL('./',location.href),originalFetch=fetch.bind(globalThis),metadata=new Map(),flightCache=new Map(),previewCache=new Map();
  const manifest=originalFetch(new URL('asset-manifest.json',root)).then(r=>{if(!r.ok)throw Error('Asset manifest unavailable');return r.json();});
  const binary=async path=>{const m=await manifest,url=new URL(m.assets[path]||path,root);if(url.origin!==root.origin)throw Error('Asset origin differs');const response=await originalFetch(url,{redirect:'error'});if(!response.ok)throw Error('Asset unavailable: '+path+' ('+response.status+')');return new Uint8Array(await response.arrayBuffer());};
  const packed=async path=>JSON.parse(fflate.strFromU8(fflate.gunzipSync(await binary(path))));
  const cache=(map,key,loader,limit)=>{if(!map.has(key)){map.set(key,loader().catch(e=>{map.delete(key);throw e}));while(map.size>limit)map.delete(map.keys().next().value);}return map.get(key);};
  const rewriteProjection=value=>{if(value?.missions)for(const m of value.missions)for(const a of m.attempts)if(a.href)a.href='detail.html?case='+a.attempt_index;return value;};
  const json=path=>cache(metadata,path,async()=>rewriteProjection(await packed('data/'+path+'.gz')),12);
  const index=()=>cache(metadata,'index',async()=>JSON.parse(fflate.strFromU8(await binary('data/index.json'))),12);
  const record=async key=>{const idx=await index(),item=idx.cases[key];if(!item)throw Error('Unknown saved case '+key);const data=await cache(flightCache,item.pack,()=>packed(item.pack),2);return {data,record:data.cases[key]};};
  const flip=v=>Number(v)===0?v:v.startsWith('-')?v.slice(1):'-'+v;
  const unpack=(r,pool)=>{const columns=r.columns.map(([id,negative])=>{const v=pool[id].split('\n');return negative?v.map(flip):v});return r.prefix+'['+Array.from({length:r.count},(_,i)=>'{'+r.keys.map((k,j)=>k+':'+columns[j][i]).join(',')+'}').join(',')+']'+r.suffix;};
  const payload=async key=>{const r=await record(key);return JSON.parse(unpack(r.record.payload,r.data.vectors));};
  const preview=async path=>{const match=/flight-(\d+)(-single)?\.svg$/.exec(path);if(!match)throw Error('Unknown preview path');const idx=await index(),item=idx.cases[match[1]];if(!item)throw Error('Unknown preview case');const data=await cache(previewCache,item.previews,()=>packed(item.previews),3);return data[match[1]+(match[2]||'')];};
  const html=async key=>{const r=await record(key);return r.record.before.map(i=>r.data.lines[i]).join('')+unpack(r.record.payload,r.data.vectors)+r.record.after.map(i=>r.data.lines[i]).join('');};
  const presentationPatch=()=>cache(metadata,'presentation-patch',async()=>{const m=await manifest;if(!m.presentation_patch)return [];const url=new URL(m.presentation_patch,root);if(url.origin!==root.origin)throw Error('Presentation patch origin differs');const response=await originalFetch(url,{redirect:'error'});if(!response.ok)throw Error('Presentation patch unavailable');const p=await response.json(),patches=p.version===1?[{...p,id:'inspection',views:['pair','rich']}]:p.version===2?p.patches:null;if(!Array.isArray(patches)||!patches.length||new Set(patches.map(v=>v.id)).size!==patches.length||patches.some(v=>!v.id||typeof v.before!=='string'||!v.before||typeof v.after!=='string'||!v.after||v.before===v.after||!Array.isArray(v.views)||!v.views.length||new Set(v.views).size!==v.views.length||v.views.some(s=>!['study','pair','rich'].includes(s))))throw Error('Invalid presentation patch');return patches;},12);
  const transport=async (text,view)=>{
    const m=await manifest;
    const patches=await presentationPatch(),patchParts=parts=>{for(const patch of patches){const count=parts.reduce((n,s)=>n+s.split(patch.before).length-1,0),expected=patch.views.includes(view)?1:0;if(count!==expected)throw Error('Presentation source binding differs: '+patch.id+' in '+view);if(count)parts=parts.map(s=>s.replace(patch.before,()=>patch.after));}return parts;};
    const rewrite=s=>s.replaceAll('https://cdn.plot.ly/plotly-basic-2.35.2.min.js','assets/plotly-basic-2.35.2.min.js').replace(/href="([^"]+)"/g,(all,path)=>{
      if(path.includes('${'))return all;
      const detail=/^flight-(\d+)\.html$/.exec(path),evidence=/^flight-(\d+)-(evidence|proof)\.json$/.exec(path);
      if(detail)return 'href="detail.html?case='+detail[1]+'"';if(evidence)return 'href="evidence.html?case='+evidence[1]+'&kind='+evidence[2]+'"';
      if(['report-input.json','study.json','source-bindings.json','source-declarations.json','comparison-review.json','navigation.json'].includes(path))return 'href="evidence.html?file='+path+'"';
      if(path.startsWith('../'))return 'href="'+new URL(path,m.source_base).href+'"';return all;
    });
    if(view==='rich'){const marker='const reportData'+' = ',match=new RegExp('(?:^|\\n)[ \\t]*'+marker).exec(text);if(!match)throw Error('Payload declaration missing');const start=match.index+match[0].length;let i=start,depth=0,quoted=false,escaped=false;for(;i<text.length;i++){const c=text[i];if(quoted){if(escaped)escaped=false;else if(c==='\\')escaped=true;else if(c==='"')quoted=false;continue;}if(c==='"')quoted=true;else if(c==='{'||c==='[')depth++;else if(c==='}'||c===']'){if(--depth===0){i++;break;}}}const [before,after]=patchParts([text.slice(0,start),text.slice(i)]);return rewrite(before)+text.slice(start,i)+rewrite(after);}
    text=rewrite(patchParts([text])[0]);return text.replace(/(<script type="application\/json" id="paired-review-data">)([\s\S]*?)(<\/script>)/,(all,a,value,b)=>a+JSON.stringify(rewriteProjection(JSON.parse(value))).replaceAll('<','\\u003c')+b);
  };
  const page=async name=>{const text=fflate.strFromU8(fflate.gunzipSync(await binary('data/'+name+'.gz'))),value=await transport(text,name==='compare.html'?'pair':'study');document.open();document.write(value);document.close();};
  const showDetail=async key=>{const value=await transport(await html(key),'rich');document.open();document.write(value);document.close();};
  globalThis.PdSavedAssets={json,payload,preview,html,record,page,showDetail,manifest};
  globalThis.fetch=(input,init)=>{
    const url=new URL(typeof input==='string'||input instanceof URL?input:input.url,location.href),path=url.pathname.startsWith(root.pathname)?url.pathname.slice(root.pathname.length):'';
    if(url.origin===root.origin&&['navigation.json','report-input.json','study.json','comparison-review.json','source-bindings.json','source-declarations.json'].includes(path))return json(path).then(v=>new Response(JSON.stringify(v),{status:200,headers:{'content-type':'application/json'}}));
    const match=/^flight-(\d+)-(evidence|proof)\.json$/.exec(path);if(url.origin===root.origin&&match)return record(match[1]).then(r=>new Response(JSON.stringify(r.record[match[2]]),{status:200,headers:{'content-type':'application/json'}}));
    return originalFetch(input,init);
  };
})();
