/* Optional comparison declaration, separate from frozen study qualification. */
(() => {
  const input=document.getElementById('comparison-review-data');
  if(input)globalThis.PdComparisonReviewData=JSON.parse(input.textContent);
  const valid=o=>o&&!['missing','unsupported','unverified','not-scheduled','input-error','execution-error'].includes(o.outcome);
  const target=o=>!valid(o)?'unavailable':o.verified_landing?'met':'unmet';
  const change=(pair,metric)=>!pair.eligible?'unavailable':pair.delta===0?'tie':metric.lower_is_better===null?'descriptive':(metric.lower_is_better?pair.delta<0:pair.delta>0)?'better':'worse';
  const pairHref=(mission,state)=>{
    const url=new URL('compare.html',location.href);for(const [k,v] of Object.entries(state))if(v!==''&&v!==undefined)url.searchParams.set(k,v);url.searchParams.set('mission',mission.id);url.searchParams.set('review',state.review||location.pathname+location.search+'#'+mission.id);return url.href;
  };
  const label=(state,study)=>{
    const c=study.contrasts.find(c=>c.id===state.contrast),a=study.variants.find(v=>v.id===c.baseline).label,b=study.variants.find(v=>v.id===c.candidate).label;
    return state.basis==='earlier'?{a:'Earlier edition · '+b,b:'Current edition · '+b}:{a,b};
  };
  globalThis.PdComparisonReviewModel={target,change,pairHref,label};
})();
