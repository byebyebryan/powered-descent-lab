/* Restore the comparison's exact filter and choose neighbours within it. */
(() => {
  const body=document.body;
  if(!body.dataset.reviewRoot)return;
  const root=new URL(body.dataset.reviewRoot,location.origin);
  const params=new URLSearchParams(location.search);
  const supplied=params.get('review');
  let review=new URL('report.html',root);
  if(supplied){try{const candidate=new URL(supplied,location.origin);if(candidate.origin===location.origin&&candidate.pathname===review.pathname)review=candidate;}catch{}}
  const current=body.dataset.reviewMission;
  review.hash=current;
  const back=document.querySelector('[data-back-review]');if(back)back.href=review.pathname+review.search+review.hash;
  const nav=document.querySelector('.review-detail-nav');if(!nav)return;
  fetch(new URL('navigation.json',root),{redirect:'error'}).then(response=>{if(!response.ok)throw Error('Navigation unavailable');return response.json();}).then(data=>{
    if(data.schema_version!==1)return;
    const p=review.searchParams;
    let state=PdPairedReviewModel.normalizeScope({q:p.get('q')||'',outcome:p.get('outcome')||'all',approach:p.get('approach')||'all',direction:p.get('direction')||'all',group:p.get('group')||''},data.groups);
    if(data.study){
      globalThis.PdStudyProjection=data.study;
      for(const key of PdStudyReviewModel.fields)state[key]=p.get(key)||'';
      state=PdStudyReviewModel.normalize(state,data.study);
      const study=data.study,contrast=study.contrasts.find(c=>c.id===state.contrast),cohort=study.cohorts.find(c=>c.id===state.cohort),metric=study.metrics.find(m=>m.id===state.metric);
      const box=document.createElement('section');box.className='study-detail-context';
      box.style.cssText='padding:12px;border:1px solid #b8c8c4;background:#edf4f1;border-radius:10px;margin:8px 0;font:14px/1.5 system-ui';
      const title=document.createElement('strong');title.textContent=study.question;box.append(title);
      const note=document.createElement('p');note.textContent=`${cohort.label} · ${contrast.label} · ${metric.label} · ${state.change==='all'?'all outcomes':state.change}${state.lo!==''||state.hi!==''?' · selected metric range':''}. Frozen study claims remain unchanged.`;box.append(note);
      const mission=data.missions.find(m=>m.id===current),pair=mission?PdStudyReviewModel.pair(mission,state,study):null;
      if(data.comparison_review){globalThis.PdComparisonReviewData=data.comparison_review;const labels=PdComparisonReviewModel.label(state,study);const note=document.createElement('p');note.textContent=`${labels.a} → ${labels.b} · native landing target A ${PdComparisonReviewModel.target(pair?.a)}, B ${PdComparisonReviewModel.target(pair?.b)} · fuel/cost budgets undeclared.`;box.append(note);if(mission){const link=document.createElement('a');link.href=PdComparisonReviewModel.pairHref(mission,{...state,review:review.pathname+review.search+review.hash});link.textContent='Inspect paired trajectories and direction histories →';box.append(link);}}
      for(const [label,id] of [['Baseline',state.basis==='earlier'?contrast.candidate:contrast.baseline],['Candidate',contrast.candidate]]){
        const variant=study.variants.find(v=>v.id===id),approach=study.variants.findIndex(v=>v.id===id),attempt=mission?.attempts.find(a=>a.approach===approach);
        if(!attempt?.href){const missing=document.createElement('span');missing.textContent=`${label}: not scheduled. `;box.append(missing);continue;}
        const link=document.createElement('a'),url=new URL(attempt.href,root);url.searchParams.set('review',review.pathname+review.search+review.hash);url.searchParams.set('mission',mission.id);url.searchParams.set('approach',String(approach));link.href=url.pathname+url.search;link.textContent=`${label}: ${variant.label}`;link.style.marginRight='12px';box.append(link);
      }
      if(pair?.eligible){const delta=document.createElement('p');delta.textContent=`Recorded candidate − baseline difference: ${pair.delta.toPrecision(6)} ${metric.unit}. ${metric.measurement}`;box.append(delta);}
      nav.after(box);
    }
    const selected=data.missions.filter(m=>PdPairedReviewModel.matches(m,state));
    const index=selected.findIndex(m=>m.id===current);
    if(index<0)return;
    const approach=Number(body.dataset.reviewApproach);
    for(const [kind,offset] of [['previous',-1],['next',1]]){
      const mission=selected[index+offset];const link=nav.querySelector(`[data-review-${kind}]`);
      if(!link)continue;
      if(!mission){link.hidden=true;continue;}
      const href=mission.attempts.find(a=>a.approach===approach)?.href;if(!href){link.hidden=true;continue;}
      const url=new URL(href,root);const context=new URL(review);context.searchParams.set('mission',mission.id);context.hash=mission.id;
      url.searchParams.set('review',context.pathname+context.search+context.hash);url.searchParams.set('mission',mission.id);url.searchParams.set('approach',String(approach));
      link.href=url.pathname+url.search;link.hidden=false;link.textContent=`${kind==='previous'?'Previous':'Next'} matched mission (${mission.mission_index})`;
    }
    nav.dataset.contextReady='true';
  }).catch(()=>{nav.dataset.contextReady='unavailable';});
})();
