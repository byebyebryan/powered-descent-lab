/* Load the exact saved samples on demand for both inline and pair-page views. */
(() => {
  let sequence=0,activeHost=null,controller=null,activeInspector=null;
  const text=(host,value,className='mission-inspection-status')=>{
    const p=document.createElement('p');p.className=className;p.textContent=value;host.replaceChildren(p);return p;
  };
  const goalFor=evidence=>{
    const point=evidence?.policy_decisions?.arrival?.request?.waypoint?.position_m;
    return point?[point]:[];
  };
  const loadAttempt=async(attempt,{label='Saved run',color='#356a96'}={},signal)=>{
    if(!attempt?.href) return {unavailable:`${label}: ${attempt?.outcome||'Not scheduled'}; no flight was recorded.`};
    const key=String(attempt.attempt_index);
    let data,evidence;
    if(globalThis.PdSavedAssets){
      const [payload,record]=await Promise.all([PdSavedAssets.payload(key),PdSavedAssets.record(key)]);
      data=payload;evidence=record?.record?.evidence||null;
    }else{
      const response=await fetch(attempt.href,{redirect:'error',signal});
      if(!response.ok)throw Error('Saved rich detail unavailable');
      data=PdFlightModel.extract(await response.text());
      evidence=null;
      if(/(?:^|\/)flight-\d+\.html(?:[?#].*)?$/.test(attempt.href)){
        const evidenceUrl=new URL(attempt.href,location.href);
        evidenceUrl.pathname=evidenceUrl.pathname.replace(/\.html$/,'-evidence.json');
        try{
          const evidenceResponse=await fetch(evidenceUrl,{redirect:'error',signal});
          if(evidenceResponse.ok)evidence=await evidenceResponse.json();
        }catch(error){if(error.name==='AbortError')throw error;}
      }
    }
    if(!Array.isArray(data?.samples)||!data.samples.length)return {unavailable:`${label}: the saved detail has no recorded flight samples.`};
    return {label,data,evidence,goals:goalFor(evidence),color};
  };
  const clear=host=>{
    if(host&&activeHost!==host)return;
    sequence++;controller?.abort();controller=null;activeInspector?.destroy?.();activeInspector=null;
    if(activeHost&&(!host||activeHost===host))activeHost.replaceChildren();
    if(!host||activeHost===host)activeHost=null;
  };
  const mountInline=async(host,sides)=>{
    clear();activeHost=host;const token=sequence,abort=new AbortController();controller=abort;
    text(host,'Loading recorded flight samples…');
    try{
      const results=await Promise.all(sides.map(side=>loadAttempt(side.attempt,{label:side.label,color:side.color},abort.signal)));
      if(token!==sequence||!host.isConnected)return;
      const records=results.filter(result=>result?.data),unavailable=results.filter(result=>result?.unavailable);
      host.replaceChildren();
      for(const result of unavailable){const p=document.createElement('p');p.className='mission-inspection-status';p.textContent=result.unavailable;host.append(p);}
      if(!records.length){if(!unavailable.length)text(host,'No saved flight is available for this mission.');return;}
      activeInspector=globalThis.PdFlightInspection.mountPair(host,records);
    }catch(error){
      if(token!==sequence||error.name==='AbortError')return;
      text(host,`Time inspection unavailable: ${error.message}`);
    }finally{if(token===sequence)controller=null;}
  };
  globalThis.PdMissionInspection={loadAttempt,mountInline,clear};
})();
