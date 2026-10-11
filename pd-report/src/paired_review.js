/* Mission hierarchy owns navigation; controller attempts are mission children. */
(() => {
  const {matches, pageMissions} = globalThis.PdPairedReviewModel;
  if (typeof document === 'undefined') return;
  const start = () => {
    const dataNode = document.getElementById('paired-review-data');
    if (!dataNode) return;
    const data = JSON.parse(dataNode.textContent);
    const root = document.getElementById('review-tree-root');
    const tbody = root.querySelector('tbody');
    const browser = document.createElement('nav');
    browser.className='mission-browser';browser.setAttribute('aria-label','Browse mission strata');
    document.getElementById('review-count').after(browser);
    const footer = document.createElement('nav');
    footer.className='mission-browser mission-browser-footer';footer.setAttribute('aria-label','Mission pages');root.after(footer);
    const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    const controls = Object.fromEntries(['q','outcome','approach','view','a','b'].map(key => [key,document.getElementById(key==='q'?'review-search':'review-'+key)]));
    const params = new URLSearchParams(location.search);
    const state = {q:params.get('q') || '',outcome:params.get('outcome') || 'all',approach:params.get('approach') || 'all',group:params.get('group') || '',mission:params.get('mission') || location.hash.slice(1),stratum:params.get('stratum') || '',page:params.get('page') || 1,view:params.get('view') || '0',a:params.get('a') || '0',b:params.get('b') || String(data.labels.length-1)};
    for (const key of ['outcome','approach','view','a','b']) if (![...controls[key].options].some(o=>o.value===state[key])) state[key]=key==='outcome'||key==='approach'?'all':key==='b'?String(data.labels.length-1):'0';
    const branches = data.branches || data.groups.map(g=>({...g,parent:null,depth:0,kind:'coverage'}));
    const byId = new Map(branches.map(b=>[b.id,b]));
    const leaves = branches.filter(b=>!branches.some(child=>child.parent===b.id));
    const leafIds = new Set(leaves.map(b=>b.id));
    const path = mission => mission.path || [mission.group];
    const ancestors = id => {const result=[];for(let b=byId.get(id);b;b=byId.get(b.parent))result.unshift(b.id);return result;};
    if (!byId.has(state.group)) state.group='';
    const opened = new Set((params.get('open') || '').split(',').filter(id=>byId.has(id)));
    // Old expand-all URLs remain readable, with one materialized stratum.
    if (!state.stratum) state.stratum=[...opened].find(id=>leafIds.has(id)) || '';
    if (state.mission) {const m=data.missions.find(m=>m.id===state.mission);if(m)path(m).forEach(id=>opened.add(id));else state.mission='';}
    const updateUrl = () => {
      const p = new URLSearchParams();
      for (const key of ['q','outcome','approach','group','mission','stratum','view','a','b']) if(state[key]&&state[key]!=='all')p.set(key,state[key]);
      if(state.stratum&&state.page>1)p.set('page',String(state.page));
      if(opened.size)p.set('open',[...opened].join(','));
      history.replaceState(null,'',location.pathname+(p.size?'?'+p:'')+(state.mission?'#'+state.mission:''));
    };
    const detailHref = (attempt, mission) => {
      if(!attempt.href)return '';
      const url=new URL(attempt.href,location.href);
      url.searchParams.set('review',location.pathname+location.search+'#'+mission.id);
      url.searchParams.set('mission',mission.id);url.searchParams.set('approach',String(attempt.approach));
      return esc(url.pathname+url.search);
    };
    const preview = attempt => attempt.preview?`<img data-preview-src="${esc(attempt.preview)}" width="156" height="92" alt="${esc(attempt.caption)}" decoding="async">`:'<span class="muted">Preview unavailable</span>';
    const pairCard = (attempt, mission) => {
      if (!attempt) return '';
      const link=detailHref(attempt,mission);
      return `<article class="pair-card"><strong>${esc(data.labels[attempt.approach])}</strong><span class="outcome-badge outcome-${esc(attempt.outcome)}">${esc(controls.outcome.querySelector(`option[value="${attempt.outcome}"]`)?.textContent || attempt.outcome)}</span>${link?`<a class="run-preview" href="${link}">${preview(attempt)}</a>`:preview(attempt)}<p class="preview-caption muted">${esc(attempt.caption)}</p></article>`;
    };
    let available=[], pageInfo=pageMissions([]);
    const chooseStratum = id => {
      leaves.forEach(b=>opened.delete(b.id));
      state.stratum=id;state.page=1;state.mission='';
      if(id)ancestors(id).forEach(parent=>opened.add(parent));
    };
    const pageControls = () => `<div class="mission-page-controls"><button type="button" data-review-page="previous"${pageInfo.page<=1?' disabled':''}>Previous page</button><span aria-live="polite">Missions ${pageInfo.start}–${pageInfo.end} of ${pageInfo.total} · page ${pageInfo.page} of ${pageInfo.pages}</span><button type="button" data-review-page="next"${pageInfo.page>=pageInfo.pages?' disabled':''}>Next page</button></div>`;
    const render = () => {
      for (const [key,input] of Object.entries(controls))input.value=state[key];
      const selected=data.missions.filter(m=>matches(m,state));
      const bins=new Map(branches.map(b=>[b.id,[]]));selected.forEach(m=>path(m).forEach(id=>bins.get(id)?.push(m)));
      available=leaves.filter(b=>bins.get(b.id).length);
      const current=selected.find(m=>m.id===state.mission);
      if(current){state.stratum=path(current).at(-1);ancestors(state.stratum).forEach(id=>opened.add(id));}
      else state.mission='';
      if(state.stratum&&!available.some(b=>b.id===state.stratum))state.stratum=available[0]?.id || '';
      leaves.forEach(b=>{if(b.id!==state.stratum)opened.delete(b.id);});
      if(state.stratum)ancestors(state.stratum).forEach(id=>opened.add(id));
      pageInfo=pageMissions(bins.get(state.stratum) || [],state.page,state.mission);state.page=pageInfo.page;
      updateUrl();
      browser.hidden=footer.hidden=!state.stratum;
      if(state.stratum){
        const index=available.findIndex(b=>b.id===state.stratum);
        const label=b=>ancestors(b.id).map(id=>byId.get(id).label).join(' → ');
        browser.innerHTML=`<div class="stratum-controls"><button type="button" data-review-stratum="previous"${index<=0?' disabled':''}>Previous stratum</button><label>Current stratum<select data-review-stratum-select>${available.map(b=>`<option value="${esc(b.id)}"${b.id===state.stratum?' selected':''}>${esc(label(b))} · ${bins.get(b.id).length} missions</option>`).join('')}</select></label><button type="button" data-review-stratum="next"${index>=available.length-1?' disabled':''}>Next stratum</button></div>${pageControls()}`;
        footer.innerHTML=pageControls();
      }else{browser.innerHTML='';footer.innerHTML='';}
      const missionRow = mission => {
        const open=state.mission===mission.id;
        const attempt=mission.attempts[Number(state.view)];
        const href=detailHref(attempt,mission);
        const depth=path(mission).length;
        const label=`Mission ${mission.mission_index}${mission.occurrence?` · repeat ${mission.occurrence+1}`:''}`;
        let html=`<tr class="paired-mission-row" id="${mission.id}" data-paired-mission="${mission.id}" tabindex="0" aria-expanded="${open}" style="--mission-depth:${depth}"><td><span class="expander">${open?'−':'+'}</span> ${label}${href?`<a class="run-preview mission-preview" href="${href}">${preview(attempt)}</a>`:preview(attempt)}</td>${attempt.cells}</tr>`;
        if(!open)return html;
        html+=`<tr class="mission-context-row"><td colspan="4"><span class="mission-path">${path(mission).map(id=>esc(byId.get(id).label)).join(' → ')} → ${label}</span><details class="row-evidence"><summary>Physical input and pairing</summary><p><code>${esc(mission.physical_sha256)}</code> · occurrence ${mission.occurrence+1}. Pairing uses physical identity and scheduled occurrence, without best-repeat selection.</p></details><strong>Controller attempts for this mission</strong></td></tr>`;
        for(const a of mission.attempts){const link=detailHref(a,mission);html+=`<tr class="paired-attempt-row" data-approach="${a.approach}" data-outcome="${esc(a.outcome)}"><td>${link?`<a class="paired-detail" href="${link}">${esc(data.labels[a.approach])}</a>`:esc(data.labels[a.approach])}<span class="row-note">Attempt ${a.attempt_index}</span></td>${a.cells}</tr>`;}
        html+=`<tr class="paired-preview-row"><td colspan="4"><details class="mission-comparison"><summary>Compare controller previews for this mission</summary><div class="paired-previews">${pairCard(mission.attempts[Number(state.a)],mission)}${state.a!==state.b?pairCard(mission.attempts[Number(state.b)],mission):''}</div></details></td></tr>`;
        return html;
      };
      const branchRows = parent => branches.filter(b=>b.parent===parent).map(branch=>{
        const missions=bins.get(branch.id);if(!missions.length)return '';
        const expanded=opened.has(branch.id);
        const landed=missions.filter(m=>m.attempts[Number(state.view)].outcome==='landed').length;
        let html=`<tr class="paired-group-row" data-paired-group="${branch.id}" data-kind="${branch.kind}" data-depth="${branch.depth}" tabindex="0" aria-expanded="${expanded}" style="--mission-depth:${branch.depth}"><td><span class="expander">${expanded?'−':'+'}</span> ${esc(branch.label)}</td><td colspan="3"><strong>${missions.length} missions</strong><span class="row-note">${landed}/${missions.length} verified landings</span></td></tr>`;
        if(expanded)html+=leafIds.has(branch.id)?pageInfo.items.map(missionRow).join(''):branchRows(branch.id);
        return html;
      }).join('');
      tbody.innerHTML=branchRows(null)||'<tr><td colspan="4">No missions match these filters. Reset to review every scheduled input.</td></tr>';
      document.getElementById('review-count').textContent=`${selected.length} / ${data.missions.length} scheduled mission occurrences matched${state.stratum?` · showing ${pageInfo.items.length} from one stratum`:''}`;
      window.PdCampaignPreviews?.loadVisible();
      window.dispatchEvent(new CustomEvent('pd-review-updated'));
      return selected.length;
    };
    for(const [key,input] of Object.entries(controls))input.addEventListener(key==='q'?'input':'change',()=>{
      state[key]=input.value;
      if(['q','outcome','approach'].includes(key)){
        state.page=1;state.mission='';
        if(state.q){const first=data.missions.find(m=>matches(m,state));chooseStratum(first?path(first).at(-1):'');}
      }
      render();
    });
    const activate = target => {
      if(target.closest('a,button,input,select,details'))return;
      const group=target.closest('[data-paired-group]');const mission=target.closest('[data-paired-mission]');
      let focusId;
      if(group){const id=group.dataset.pairedGroup;
        if(leafIds.has(id))chooseStratum(opened.has(id)?'':id);
        else if(opened.has(id)){
          branches.filter(b=>ancestors(b.id).includes(id)).forEach(b=>opened.delete(b.id));
          if(ancestors(state.stratum).includes(id))chooseStratum('');
        }else opened.add(id);
        focusId=id;render();root.querySelector(`[data-paired-group="${focusId}"]`)?.focus({preventScroll:true});}
      else if(mission){state.mission=state.mission===mission.dataset.pairedMission?'':mission.dataset.pairedMission;focusId=mission.id;render();document.getElementById(focusId)?.focus({preventScroll:true});}
    };
    root.addEventListener('click',e=>activate(e.target));
    root.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){if(e.target.matches('[data-paired-group],[data-paired-mission]')){e.preventDefault();activate(e.target);}}});
    const focus = (group,approach) => {state.group=group;state.approach=String(approach);state.view=String(approach);state.outcome='all';state.q='';opened.clear();const first=data.missions.find(m=>matches(m,state));chooseStratum(first?path(first).at(-1):'');render();browser.scrollIntoView({block:'start'});root.querySelector(`[data-paired-group="${group}"]`)?.focus({preventScroll:true});};
    document.querySelectorAll('[data-paired-focus]').forEach(button=>button.addEventListener('click',()=>focus(button.dataset.pairedFocus,button.dataset.approach)));
    document.querySelectorAll('[data-review-outcome]').forEach(button=>button.addEventListener('click',()=>{state.outcome=button.dataset.reviewOutcome;state.approach=button.dataset.approach || 'all';if(state.approach!=='all')state.view=state.approach;state.group='';state.q='';chooseStratum('');opened.clear();render();root.scrollIntoView({block:'start'});}));
    document.querySelectorAll('[data-paired-depth]').forEach(button=>button.addEventListener('click',()=>{
      const depth=Number(button.dataset.pairedDepth);opened.clear();chooseStratum('');
      branches.filter(b=>b.depth<Math.min(depth,2)).forEach(b=>opened.add(b.id));
      if(depth>=3){const first=data.missions.find(m=>matches(m,state));chooseStratum(first?path(first).at(-1):'');}
      render();
    }));
    document.querySelectorAll('[data-paired-action]').forEach(button=>button.addEventListener('click',()=>{if(button.dataset.pairedAction==='reset')Object.assign(state,{q:'',outcome:'all',approach:'all',group:''});chooseStratum('');opened.clear();render();}));
    for(const nav of [browser,footer]){
      nav.addEventListener('click',event=>{
        const page=event.target.closest('[data-review-page]');const stratum=event.target.closest('[data-review-stratum]');
        if(page&&!page.disabled){state.mission='';state.page+=page.dataset.reviewPage==='next'?1:-1;}
        else if(stratum&&!stratum.disabled){const index=available.findIndex(b=>b.id===state.stratum);chooseStratum(available[index+(stratum.dataset.reviewStratum==='next'?1:-1)].id);}
        else return;
        render();browser.scrollIntoView({block:'start'});
        browser.querySelector(page?`[data-review-page="${page.dataset.reviewPage}"]`:`[data-review-stratum="${stratum.dataset.reviewStratum}"]`)?.focus({preventScroll:true});
      });
      nav.addEventListener('change',event=>{if(event.target.matches('[data-review-stratum-select]')){chooseStratum(event.target.value);render();browser.querySelector('select').focus({preventScroll:true});}});
    }
    window.PdPairedReview={data,state,render,focus,matches,opened,get pageInfo(){return pageInfo;}};
    render();
    if(state.mission)document.getElementById(state.mission)?.scrollIntoView({block:'start'});
  };
  document.addEventListener('pd-batch-ready',start,{once:true});
})();
