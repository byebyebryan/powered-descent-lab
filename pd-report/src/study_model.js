/* Fixed source declarations and exploratory queries are separate. No statistics fit here. */
(() => {
  const caches=new WeakMap();
  const index=study=>{
    if(!caches.has(study)){
      const missions=new Map();
      for(const o of study.observations){if(!missions.has(o.mission))missions.set(o.mission,[]);missions.get(o.mission).push(o);}
      caches.set(study,missions);
    }
    return caches.get(study);
  };
  const normalize=(state,study)=>{
    if(!study)return {...state};
    const choose=(key,list,fallback)=>list.some(x=>x.id===state[key])?state[key]:fallback;
    const result={...state,cohort:choose('cohort',study.cohorts,study.default_cohort),contrast:choose('contrast',study.contrasts,study.default_contrast),metric:choose('metric',study.metrics,study.default_metric),change:['all','preserved','gained','lost','missed','unpaired'].includes(state.change)?state.change:'all'};
    if(globalThis.PdComparisonReviewData){if(state.contrast&&!study.contrasts.some(c=>c.id===state.contrast))result.comparison_error='Requested contrast is unavailable: '+state.contrast;if(state.basis&&!['strategy','earlier'].includes(state.basis))result.comparison_error='Requested comparison reference is unavailable: '+state.basis;if(state.basis==='earlier'&&!PdComparisonReviewData.earlier?.same_captures_verified)result.comparison_error='Pinned earlier edition is not verified';}
    result.basis=state.basis==='earlier'&&globalThis.PdComparisonReviewData?.earlier?.same_captures_verified?'earlier':'strategy';
    result.metric_change=['all','better','worse','tie','descriptive','unavailable'].includes(state.metric_change)?state.metric_change:'all';
    result.target_status=['all','met','unmet','unavailable'].includes(state.target_status)?state.target_status:'all';
    for(const key of ['lo','hi'])if(result[key]!==''&&result[key]!==undefined&&!Number.isFinite(Number(result[key])))result[key]='';
    return result;
  };
  const pair=(mission,state,study)=>{
    const contrast=study.contrasts.find(c=>c.id===state.contrast);
    const records=(index(study).get(mission.id)||[]).filter(o=>o.cohort===state.cohort);
    const a=records.find(o=>o.variant===(state.basis==='earlier'?contrast?.candidate:contrast?.baseline)),b=records.find(o=>o.variant===contrast?.candidate);
    const valid=o=>o&&!['missing','unsupported','unverified','not-scheduled','input-error','execution-error'].includes(o.outcome);
    const category=!valid(a)||!valid(b)?'unpaired':a.verified_landing?(b.verified_landing?'preserved':'lost'):(b.verified_landing?'gained':'missed');
    const metric=study.metrics.find(m=>m.id===state.metric);
    const eligible=Boolean(valid(a)&&valid(b)&&metric&&(!metric.successful_pairs_only||(a.verified_landing&&b.verified_landing))&&Number.isFinite(a.values[state.metric])&&Number.isFinite(b.values[state.metric]));
    return {a,b,category,eligible,delta:eligible?b.values[state.metric]-a.values[state.metric]:null,inCohort:records.length>0};
  };
  const matches=(mission,state,study)=>{
    if(!study||!state.cohort)return true;
    if(state.comparison_error)return false;
    const p=pair(mission,state,study);if(!p.inCohort)return false;
    if(globalThis.PdComparisonReviewModel){const metric=study.metrics.find(m=>m.id===state.metric);if(state.metric_change&&state.metric_change!=='all'&&PdComparisonReviewModel.change(p,metric)!==state.metric_change)return false;if(state.target_status&&state.target_status!=='all'&&PdComparisonReviewModel.target(p.b)!==state.target_status)return false;}
    if(state.change&&state.change!=='all'&&p.category!==state.change)return false;
    if(state.cell){const [axisA,axisB]=study.condition_axes;const o=p.a||p.b;if([o?.attributes[axisA]||'Unavailable',o?.attributes[axisB]||'All'].join('|')!==state.cell)return false;}
    const has=key=>state[key]!==''&&state[key]!==undefined&&state[key]!==null;
    if(has('lo')||has('hi')){
      if(!p.eligible)return false;
      if(has('lo')&&p.delta<Number(state.lo))return false;
      if(has('hi')&&p.delta>Number(state.hi))return false;
    }
    return true;
  };
  const summary=(missions,state,study)=>{
    const counts={preserved:0,gained:0,lost:0,missed:0,unpaired:0};const pairs=[];
    for(const m of missions){const p=pair(m,state,study);if(!p.inCohort)continue;counts[p.category]++;if(p.eligible)pairs.push({mission:m,...p});}
    return {counts,pairs,total:Object.values(counts).reduce((a,b)=>a+b,0)};
  };
  const fields=['cohort','contrast','metric','change','lo','hi','cell','basis','metric_change','target_status'];
  globalThis.PdStudyReviewModel={normalize,pair,matches,summary,fields};
})();
