/* Mission hierarchy owns navigation; controller attempts are mission children. */
(() => {
  const {matches} = globalThis.PdPairedReviewModel;
  if (typeof document === 'undefined') return;
  const start = () => {
    const dataNode = document.getElementById('paired-review-data');
    if (!dataNode) return;
    const data = JSON.parse(dataNode.textContent);
    const root = document.getElementById('review-tree-root');
    const tbody = root.querySelector('tbody');
    const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    const controls = Object.fromEntries(['q','outcome','approach','view','a','b'].map(key => [key,document.getElementById(key==='q'?'review-search':'review-'+key)]));
    const params = new URLSearchParams(location.search);
    const state = {q:params.get('q') || '',outcome:params.get('outcome') || 'all',approach:params.get('approach') || 'all',group:params.get('group') || '',mission:params.get('mission') || location.hash.slice(1),view:params.get('view') || '0',a:params.get('a') || '0',b:params.get('b') || String(data.labels.length-1)};
    for (const key of ['outcome','approach','view','a','b']) if (![...controls[key].options].some(o=>o.value===state[key])) state[key]=key==='outcome'||key==='approach'?'all':key==='b'?String(data.labels.length-1):'0';
    const branches = data.branches || data.groups.map(g=>({...g,parent:null,depth:0,kind:'coverage'}));
    const byId = new Map(branches.map(b=>[b.id,b]));
    const path = mission => mission.path || [mission.group];
    const ancestors = id => {const result=[];for(let b=byId.get(id);b;b=byId.get(b.parent))result.unshift(b.id);return result;};
    if (!byId.has(state.group)) state.group='';
    const opened = new Set((params.get('open') || '').split(',').filter(id=>byId.has(id)));
    if (state.mission) {const m=data.missions.find(m=>m.id===state.mission);if(m)path(m).forEach(id=>opened.add(id));else state.mission='';}
    const updateUrl = () => {
      const p = new URLSearchParams();
      for (const key of ['q','outcome','approach','group','mission','view','a','b']) if(state[key]&&state[key]!=='all')p.set(key,state[key]);
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
    const render = (save=true) => {
      for (const [key,input] of Object.entries(controls))input.value=state[key];
      if(save)updateUrl();
      const selected=data.missions.filter(m=>matches(m,state));
      const bins=new Map(branches.map(b=>[b.id,[]]));selected.forEach(m=>path(m).forEach(id=>bins.get(id)?.push(m)));
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
        if(expanded)html+=branches.some(b=>b.parent===branch.id)?branchRows(branch.id):missions.map(missionRow).join('');
        return html;
      }).join('');
      tbody.innerHTML=branchRows(null)||'<tr><td colspan="4">No missions match these filters. Reset to review every scheduled input.</td></tr>';
      document.getElementById('review-count').textContent=`${selected.length} / ${data.missions.length} scheduled mission occurrences matched`;
      window.PdCampaignPreviews?.loadVisible();
      window.dispatchEvent(new CustomEvent('pd-review-updated'));
      return selected.length;
    };
    for(const [key,input] of Object.entries(controls))input.addEventListener(key==='q'?'input':'change',()=>{state[key]=input.value;render();});
    const activate = target => {
      if(target.closest('a,button,input,select,details'))return;
      const group=target.closest('[data-paired-group]');const mission=target.closest('[data-paired-mission]');
      let focusId;
      if(group){const id=group.dataset.pairedGroup;if(opened.has(id))opened.delete(id);else opened.add(id);focusId=id;render();root.querySelector(`[data-paired-group="${focusId}"]`)?.focus({preventScroll:true});}
      else if(mission){state.mission=state.mission===mission.dataset.pairedMission?'':mission.dataset.pairedMission;focusId=mission.id;render();document.getElementById(focusId)?.focus({preventScroll:true});}
    };
    root.addEventListener('click',e=>activate(e.target));
    root.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){if(e.target.matches('[data-paired-group],[data-paired-mission]')){e.preventDefault();activate(e.target);}}});
    const focus = (group,approach) => {state.group=group;state.approach=String(approach);state.view=String(approach);state.outcome='all';state.q='';state.mission='';opened.clear();ancestors(group).forEach(id=>opened.add(id));render();root.scrollIntoView({block:'start'});root.querySelector(`[data-paired-group="${group}"]`)?.focus({preventScroll:true});};
    document.querySelectorAll('[data-paired-focus]').forEach(button=>button.addEventListener('click',()=>focus(button.dataset.pairedFocus,button.dataset.approach)));
    document.querySelectorAll('[data-review-outcome]').forEach(button=>button.addEventListener('click',()=>{state.outcome=button.dataset.reviewOutcome;state.approach=button.dataset.approach || 'all';if(state.approach!=='all')state.view=state.approach;state.group='';state.q='';state.mission='';opened.clear();render();root.scrollIntoView({block:'start'});}));
    document.querySelectorAll('[data-paired-depth]').forEach(button=>button.addEventListener('click',()=>{opened.clear();branches.filter(b=>b.depth<Number(button.dataset.pairedDepth)).forEach(b=>opened.add(b.id));state.mission='';render();}));
    document.querySelectorAll('[data-paired-action]').forEach(button=>button.addEventListener('click',()=>{if(button.dataset.pairedAction==='reset')Object.assign(state,{q:'',outcome:'all',approach:'all',group:'',mission:''});else state.mission='';opened.clear();render();}));
    window.PdPairedReview={data,state,render,focus,matches,opened};
    render(false);
    if(state.mission)document.getElementById(state.mission)?.scrollIntoView({block:'start'});
  };
  document.addEventListener('pd-batch-ready',start,{once:true});
})();
