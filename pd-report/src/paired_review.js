/* Mission hierarchy owns navigation; controller attempts are mission children. */
(() => {
  const {matches, normalizeScope, pageMissions, visibleBranches, collapseBranch} = globalThis.PdPairedReviewModel;
  if (typeof document === 'undefined') return;
  const start = () => {
    const dataNode = document.getElementById('paired-review-data');
    if (!dataNode) return;
    const data = JSON.parse(dataNode.textContent);
    const root = document.getElementById('review-tree-root');
    const tbody = root.querySelector('tbody');
    const browser = document.createElement('nav');
    browser.className='mission-browser';browser.setAttribute('aria-label','Browse mission strata');
    const browserAnchor = document.createElement('div');browserAnchor.className='mission-browser-anchor';
    document.getElementById('review-count').after(browserAnchor);browserAnchor.after(browser);
    const footer = document.createElement('nav');
    footer.className='mission-browser mission-browser-footer';footer.setAttribute('aria-label','Mission group and page navigation');root.after(footer);
    const revealBrowser = () => browserAnchor.scrollIntoView({block:'start'});
    const startButton = document.querySelector('[data-paired-depth="3"]');
    const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    const controls = Object.fromEntries(['q','outcome','approach','direction','view','a','b'].map(key => [key,document.getElementById(key==='q'?'review-search':'review-'+key)]));
    const params = new URLSearchParams(location.search);
    const state = {q:params.get('q') || '',outcome:params.get('outcome') || 'all',approach:params.get('approach') || 'all',direction:params.get('direction') || 'all',group:params.get('group') || '',mission:params.get('mission') || location.hash.slice(1),stratum:params.get('stratum') || '',page:params.get('page') || 1,view:params.get('view') || '0',a:params.get('a') || '0',b:params.get('b') || String(data.labels.length-1),run:params.get('run')||''};
    const study=globalThis.PdStudyProjection;
    const reviewMode=PdMissionPresentation.mode(data,study);
    // Legacy projections may still carry the old four-column scaffold.
    // The active renderer owns its two-column leaf vocabulary in either mode.
    root.querySelector('thead').innerHTML=`<tr><th>Mission / occurrence</th><th>${reviewMode==='comparison'?'Declared runs':'Saved run'}</th></tr>`;
    if(reviewMode==='single')for(const input of [controls.a,controls.b])input?.closest('.comparison-options')?.remove();
    if(study){
      for(const key of PdStudyReviewModel.fields)state[key]=params.get(key)||'';
      Object.assign(state,PdStudyReviewModel.normalize(state,study));
      const contrast=study.contrasts.find(c=>c.id===state.contrast);
      state.a=String(study.variants.findIndex(v=>v.id===contrast.baseline));state.b=String(study.variants.findIndex(v=>v.id===contrast.candidate));
      if(state.basis==='earlier')state.a=state.b;
      if(!params.has('view'))state.view=state.b;
      if(controls.a&&controls.b){controls.a.disabled=controls.b.disabled=true;controls.a.title=controls.b.title='Pairing follows the declared study comparison above.';}
    }
    if(reviewMode==='comparison')state.view=state.b;
    for (const key of ['outcome','approach','direction','view','a','b']) if (controls[key]&&![...controls[key].options].some(o=>o.value===state[key])) state[key]=['outcome','approach','direction'].includes(key)?'all':key==='b'?String(data.labels.length-1):'0';
    if(reviewMode==='comparison')state.view=state.b;
    if(controls.view&&reviewMode==='comparison'){controls.view.disabled=true;controls.view.title='The displayed run follows the selected declared comparison.';}
    Object.assign(state,normalizeScope(state,data.groups));
    const branches = data.branches || data.groups.map(g=>({...g,parent:null,depth:0,kind:'coverage'}));
    const unit = study ? 'input group' : branches.some(b=>b.kind==='band') ? 'terrain band' : 'stratum';
    browser.setAttribute('aria-label',`Browse mission ${unit}s`);
    const byId = new Map(branches.map(b=>[b.id,b]));
    const leaves = branches.filter(b=>!branches.some(child=>child.parent===b.id));
    const leafIds = new Set(leaves.map(b=>b.id));
    const path = mission => mission.path || [mission.group];
    const ancestors = id => {const result=[];for(let b=byId.get(id);b;b=byId.get(b.parent))result.unshift(b.id);return result;};
    if (!byId.has(state.group)) state.group='';
    const branchId = id => data.groups.find(g=>g.id===id)?.branch || id;
    state.stratum=branchId(state.stratum);
    const opened = new Set((params.get('open') || '').split(',').map(branchId).filter(id=>byId.has(id)));
    // Old expand-all URLs remain readable, with one materialized stratum.
    if (!state.stratum) state.stratum=[...opened].find(id=>leafIds.has(id)) || '';
    if(state.stratum&&!params.has('open'))ancestors(state.stratum).forEach(id=>opened.add(id));
    if (state.mission) {const m=data.missions.find(m=>m.id===state.mission);if(m)path(m).forEach(id=>opened.add(id));else state.mission='';}
    const updateUrl = () => {
      if(state.comparison_error)return;
      const p = new URLSearchParams();
      for (const key of ['q','outcome','approach','direction','group','mission','stratum','view','a','b']) if(state[key]&&state[key]!=='all')p.set(key,state[key]);
      if(state.run&&state.mission)p.set('run',state.run);
      if(study)for(const key of PdStudyReviewModel.fields)if(state[key]!==''&&state[key]!==undefined&&state[key]!=='all')p.set(key,state[key]);
      if(state.stratum&&state.page>1)p.set('page',String(state.page));
      if(opened.size||state.stratum)p.set('open',[...opened].join(','));
      if(showGroupTree)p.set('tree','full');
      history.replaceState(null,'',location.pathname+(p.size?'?'+p:'')+(state.mission?'#'+state.mission:''));
    };
    const detailHref = (attempt, mission) => {
      if(!attempt.href)return '';
      const url=new URL(attempt.href,location.href);
      url.searchParams.set('review',location.pathname+location.search+'#'+mission.id);
      url.searchParams.set('mission',mission.id);url.searchParams.set('approach',String(attempt.approach));
      return esc(url.pathname+url.search);
    };
    const selectedPairHref = mission => study&&globalThis.PdComparisonReviewModel
      ? esc(PdComparisonReviewModel.pairHref(mission,state)) : '';
    let available=[], pageInfo=pageMissions([]), showGroupTree=params.get('tree')==='full';
    const chooseStratum = id => {
      state.stratum=id;state.page=1;state.mission='';
      if(id)ancestors(id).forEach(parent=>opened.add(parent));
    };
    const pageControls = () => `<div class="mission-page-controls"><button type="button" data-review-page="previous"${pageInfo.page<=1?' disabled':''}>Previous page</button><span aria-live="polite">Page ${pageInfo.page} of ${pageInfo.pages} · missions ${pageInfo.start}–${pageInfo.end} of ${pageInfo.total} in this group</span><button type="button" data-review-page="next"${pageInfo.page>=pageInfo.pages?' disabled':''}>Next page</button></div>`;
    const missionModels = new Map();
    const render = () => {
      missionModels.clear();
      if(study)Object.assign(state,PdStudyReviewModel.normalize(state,study));
      if(reviewMode==='comparison')state.view=state.b;
      Object.assign(state,normalizeScope(state,data.groups));
      for (const [key,input] of Object.entries(controls))if(input)input.value=state[key];
      if(state.direction!=='all')controls.direction.closest('details').open=true;
      const selected=data.missions.filter(m=>matches(m,state));
      const bins=new Map(branches.map(b=>[b.id,[]]));selected.forEach(m=>path(m).forEach(id=>bins.get(id)?.push(m)));
      available=leaves.filter(b=>bins.get(b.id).length);
      const current=selected.find(m=>m.id===state.mission);
      if(current){state.stratum=path(current).at(-1);ancestors(state.stratum).forEach(id=>opened.add(id));}
      else state.mission='';
      if(!current||!current.attempts.some(a=>String(a.attempt_index)===String(state.run)))state.run='';
      if(state.stratum&&!available.some(b=>b.id===state.stratum))chooseStratum(available[0]?.id || '');
      leaves.forEach(b=>{if(b.id!==state.stratum)opened.delete(b.id);});
      if(state.stratum&&!showGroupTree){const active=new Set(ancestors(state.stratum));for(const id of opened)if(!active.has(id))opened.delete(id);}
      pageInfo=pageMissions(bins.get(state.stratum) || [],state.page,state.mission);state.page=pageInfo.page;
      updateUrl();
      browser.hidden=footer.hidden=!state.stratum;
      if(startButton){startButton.textContent=state.stratum?'Resume browsing':'Start browsing';startButton.title=state.stratum?'Return to the current group and page. Use Next group to advance.':'Open the first group matching these filters.';}
      if(state.stratum){
        const index=available.findIndex(b=>b.id===state.stratum);
        const label=b=>ancestors(b.id).map(id=>byId.get(id).label).join(' → ');
        const groupControls=`<div class="stratum-controls"><button type="button" data-review-stratum="previous" aria-label="Previous ${unit}"${index<=0?' disabled':''}>Previous group</button><label>Group ${index+1} of ${available.length} · ${unit}<select data-review-stratum-select aria-label="Current mission group">${available.map(b=>`<option value="${esc(b.id)}"${b.id===state.stratum?' selected':''}>${esc(label(b))} · ${bins.get(b.id).length} missions</option>`).join('')}</select></label><button type="button" data-review-stratum="next" aria-label="Next ${unit}"${index>=available.length-1?' disabled':''}>Next group</button></div>`;
        const scope=state.group?`<p class="mission-group-scope">Focused on ${esc(byId.get(state.group)?.label || state.group)}. <button type="button" data-review-scope="clear">Show all matching groups</button></p>`:'';
        const studyContext=study?`<p data-study-tree-context class="study-mini" style="margin:5px 0"></p>`:'';
        const heading=`<div class="mission-browser-heading"><strong>Mission browsing</strong><button type="button" data-review-tree aria-pressed="${showGroupTree}">${showGroupTree?'Focus current group':'Show group tree'}</button></div>${studyContext}${scope}`;
        browser.innerHTML=`${heading}${groupControls}${pageControls()}`;
        footer.innerHTML=`${groupControls}${pageControls()}`;
      }else{browser.innerHTML='';footer.innerHTML='';}
      const missionRow = mission => {
        const open=state.mission===mission.id;
        const depth=path(mission).length;
        const label=`Mission ${mission.mission_index}${mission.occurrence?` · repeat ${mission.occurrence+1}`:''}`;
        const direction=mission.direction_unavailable?'Direction unavailable':mission.direction<0?'Travel left':mission.direction>0?'Travel right':'Vertical';
        const leaf=PdMissionPresentation.render(mission,state,data,study,{
          detailHref:attempt=>detailHref(attempt,mission),
          comparisonReview:globalThis.PdComparisonReviewData
        });
        missionModels.set(mission.id,leaf.model);
        let html=`<tr class="paired-mission-row" id="${mission.id}" data-paired-mission="${mission.id}" data-direction="${mission.direction}" tabindex="0" aria-expanded="${open}" style="--mission-depth:${depth}"><td class="mission-identity"><span class="expander">${open?'−':'+'}</span> ${label}<span class="row-note mission-direction">${direction}</span></td><td class="mission-presentation-cell">${leaf.html}</td></tr>`;
        if(!open)return html;
        const pairHref=selectedPairHref(mission);
        html+=`<tr class="mission-context-row"><td colspan="2"><span class="mission-path">${path(mission).map(id=>esc(byId.get(id).label)).join(' → ')} → ${label}</span><details class="row-evidence"><summary>Physical input and pairing</summary><p><code>${esc(mission.physical_sha256)}</code> · occurrence ${mission.occurrence+1}. Pairing uses physical identity and scheduled occurrence, without best-repeat selection.</p></details>${pairHref?`<a class="compare-pair" href="${pairHref}">Open paired inspection page</a>`:''}<strong>All saved attempts</strong></td></tr>`;
        for(const a of mission.attempts){const link=detailHref(a,mission);html+=`<tr class="paired-attempt-row" data-approach="${a.approach}" data-outcome="${esc(a.outcome)}"><td colspan="2"><div class="mission-attempt-summary">${link?`<a class="paired-detail" href="${link}">${esc(data.labels[a.approach])}</a>`:esc(data.labels[a.approach])}<span class="row-note">Attempt ${a.attempt_index} · ${esc(a.outcome)}</span><details class="legacy-run-summary"><summary>Saved summary values</summary><table class="legacy-run-cells"><tbody><tr>${a.cells}</tr></tbody></table></details></div></td></tr>`;}
        html+=`<tr class="mission-inspection-row"><td colspan="2"><details class="mission-time-inspection" data-mission-inspection="${esc(mission.id)}"><summary>Inspect recorded time</summary><div class="mission-inspection-host" data-inspection-host></div></details></td></tr>`;
        return html;
      };
      const displayedBranches=visibleBranches(branches,state.stratum,showGroupTree);
      const branchRows = parent => displayedBranches.filter(b=>b.parent===parent).map(branch=>{
        const missions=bins.get(branch.id);if(!missions.length)return '';
        const expanded=opened.has(branch.id);
        const landed=missions.filter(m=>m.attempts.find(a=>a.approach===Number(state.view))?.outcome==='landed').length;
        let comparisonSummary='';if(study&&globalThis.PdComparisonReviewData){const s=PdStudyReviewModel.summary(missions,state,study);comparisonSummary=`<span class="row-note">A→B: ${s.counts.preserved} preserved · ${s.counts.gained} gained · ${s.counts.lost} lost · ${s.counts.missed} neither · ${s.counts.unpaired} unavailable; ${s.pairs.length} eligible metric pairs. Native landing target met ${s.counts.preserved+s.counts.gained}/${missions.length}.</span>`;}
        let html=`<tr class="paired-group-row" data-paired-group="${branch.id}" data-kind="${branch.kind}" data-depth="${branch.depth}" tabindex="0" aria-expanded="${expanded}" style="--mission-depth:${branch.depth}"><td><span class="expander">${expanded?'−':'+'}</span> ${esc(branch.label)}</td><td><strong>${missions.length} missions</strong><span class="row-note">${landed}/${missions.length} verified landings</span>${comparisonSummary}</td></tr>`;
        if(expanded)html+=leafIds.has(branch.id)?pageInfo.items.map(missionRow).join(''):branchRows(branch.id);
        return html;
      }).join('');
      PdMissionInspection.clear();
      tbody.innerHTML=branchRows(null)||'<tr><td colspan="2">No missions match these filters. Reset to review every scheduled input.</td></tr>';
      tbody.querySelectorAll('[data-mission-inspection]').forEach(details=>details.addEventListener('toggle',()=>{
        if(details.open){const model=missionModels.get(details.dataset.missionInspection);PdMissionInspection.mountInline(details.querySelector('[data-inspection-host]'),model?.sides||[]);}
        else PdMissionInspection.clear(details.querySelector('[data-inspection-host]'));
      }));
      const groupVisible=state.stratum&&ancestors(state.stratum).every(id=>opened.has(id));
      document.getElementById('review-count').textContent=`${selected.length} / ${data.missions.length} scheduled mission occurrences matched${state.stratum?groupVisible?` · showing ${pageInfo.items.length} from one ${unit}`:' · current group collapsed':''}`;
      window.PdCampaignPreviews?.loadVisible();
      root.style.setProperty('--review-navigation-height',`${browser.hidden?0:browser.getBoundingClientRect().height+20}px`);
      window.dispatchEvent(new CustomEvent('pd-review-updated'));
      return selected.length;
    };
    for(const [key,input] of Object.entries(controls))if(input)input.addEventListener(key==='q'?'input':'change',()=>{
      state[key]=input.value;
      if(reviewMode==='comparison'&&key==='b')state.view=state.b;
      if(key==='view')state.run='';
      if(['q','outcome','approach','direction'].includes(key)){
        state.page=1;state.mission='';state.run='';
        if(state.q){const first=data.missions.find(m=>matches(m,state));chooseStratum(first?path(first).at(-1):'');}
      }
      render();
    });
    root.addEventListener('change',event=>{
      const select=event.target.closest('[data-mission-run]');if(!select)return;
      state.mission=select.dataset.missionRun;state.run=select.value;render();
      document.querySelector(`[data-mission-run="${state.mission}"]`)?.focus({preventScroll:true});
    });
    const activate = target => {
      if(target.closest('a,button,input,select,details'))return;
      const group=target.closest('[data-paired-group]');const mission=target.closest('[data-paired-mission]');
      let focusId;
      if(group){const id=group.dataset.pairedGroup;
        if(opened.has(id)){
          const remaining=collapseBranch(branches,opened,id);opened.clear();remaining.forEach(id=>opened.add(id));
          if(ancestors(state.stratum).includes(id)){state.mission='';state.run='';}
        }else if(leafIds.has(id)){
          if(state.stratum===id)ancestors(id).forEach(parent=>opened.add(parent));
          else chooseStratum(id);
        }else opened.add(id);
        focusId=id;render();root.querySelector(`[data-paired-group="${focusId}"]`)?.focus({preventScroll:true});}
      else if(mission){state.mission=state.mission===mission.dataset.pairedMission?'':mission.dataset.pairedMission;if(!state.mission)state.run='';focusId=mission.id;render();document.getElementById(focusId)?.focus({preventScroll:true});}
    };
    root.addEventListener('click',e=>activate(e.target));
    root.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){if(e.target.matches('[data-paired-group],[data-paired-mission]')){e.preventDefault();activate(e.target);}}});
    const focus = (group,approach) => {Object.assign(state,{group,direction:'all',approach:String(approach),view:String(approach),outcome:'all',q:'',run:''},normalizeScope({group,direction:'all'},data.groups));opened.clear();const first=data.missions.find(m=>matches(m,state));chooseStratum(first?path(first).at(-1):'');render();revealBrowser();root.querySelector(`[data-paired-group="${state.group}"]`)?.focus({preventScroll:true});};
    document.querySelectorAll('[data-paired-focus]').forEach(button=>button.addEventListener('click',()=>focus(button.dataset.pairedFocus,button.dataset.approach)));
    document.querySelectorAll('[data-review-outcome]').forEach(button=>button.addEventListener('click',()=>{state.outcome=button.dataset.reviewOutcome;state.approach=button.dataset.approach || 'all';if(state.approach!=='all')state.view=state.approach;state.group=button.closest('[data-outcome-family]')?.dataset.outcomeFamily || '';state.direction='all';state.q='';chooseStratum('');opened.clear();render();root.scrollIntoView({block:'start'});}));
    document.querySelectorAll('[data-paired-depth]').forEach(button=>button.addEventListener('click',()=>{
      const depth=Number(button.dataset.pairedDepth);
      if(depth>=3){
        if(!state.stratum){const first=data.missions.find(m=>matches(m,state));chooseStratum(first?path(first).at(-1):'');}
        else ancestors(state.stratum).forEach(id=>opened.add(id));
      }else{
        opened.clear();chooseStratum('');
        branches.filter(b=>!leafIds.has(b.id)&&b.depth<depth).forEach(b=>opened.add(b.id));
      }
      render();
      if(depth>=3&&state.stratum)revealBrowser();
    }));
    document.querySelectorAll('[data-paired-action]').forEach(button=>button.addEventListener('click',()=>{if(button.dataset.pairedAction==='reset'){Object.assign(state,{q:'',outcome:'all',approach:'all',direction:'all',group:'',run:''});if(study)Object.assign(state,{change:'all',lo:'',hi:'',cell:'',metric_change:'all',target_status:'all'});}chooseStratum('');opened.clear();render();}));
    for(const nav of [browser,footer]){
      nav.addEventListener('click',event=>{
        if(event.target.closest('[data-review-tree]')){showGroupTree=!showGroupTree;render();revealBrowser();browser.querySelector('[data-review-tree]').focus({preventScroll:true});return;}
        if(event.target.closest('[data-review-scope="clear"]')){state.group='';render();revealBrowser();return;}
        const page=event.target.closest('[data-review-page]');const stratum=event.target.closest('[data-review-stratum]');
        if(page&&!page.disabled){state.mission='';state.run='';state.page+=page.dataset.reviewPage==='next'?1:-1;ancestors(state.stratum).forEach(id=>opened.add(id));}
        else if(stratum&&!stratum.disabled){const index=available.findIndex(b=>b.id===state.stratum);chooseStratum(available[index+(stratum.dataset.reviewStratum==='next'?1:-1)].id);}
        else return;
        render();revealBrowser();
        browser.querySelector(page?`[data-review-page="${page.dataset.reviewPage}"]`:`[data-review-stratum="${stratum.dataset.reviewStratum}"]`)?.focus({preventScroll:true});
      });
      nav.addEventListener('change',event=>{if(event.target.matches('[data-review-stratum-select]')){chooseStratum(event.target.value);render();revealBrowser();browser.querySelector('select').focus({preventScroll:true});}});
    }
    window.PdPairedReview={data,state,render,focus,matches,opened,get pageInfo(){return pageInfo;},get showGroupTree(){return showGroupTree;}};
    render();
    if(state.mission)document.getElementById(state.mission)?.scrollIntoView({block:'start'});
  };
  document.addEventListener('pd-batch-ready',start,{once:true});
})();
