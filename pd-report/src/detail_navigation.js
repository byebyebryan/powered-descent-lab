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
    const state=PdPairedReviewModel.normalizeScope({q:p.get('q')||'',outcome:p.get('outcome')||'all',approach:p.get('approach')||'all',direction:p.get('direction')||'all',group:p.get('group')||''},data.groups);
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
