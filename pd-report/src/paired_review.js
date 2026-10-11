/* Compact saved projections only. Branches and images materialize on demand. */
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
    const controls = {q:document.getElementById('review-search'), outcome:document.getElementById('review-outcome'), approach:document.getElementById('review-approach'), a:document.getElementById('review-a'), b:document.getElementById('review-b')};
    const params = new URLSearchParams(location.search);
    const state = {q:params.get('q') || '',outcome:params.get('outcome') || 'all',approach:params.get('approach') || 'all',group:params.get('group') || '',mission:params.get('mission') || location.hash.slice(1),a:params.get('a') || '0',b:params.get('b') || String(data.labels.length-1)};
    for (const key of ['outcome','approach','a','b']) if (![...controls[key].options].some(o=>o.value===state[key])) state[key]=key==='outcome'||key==='approach'?'all':key==='a'?'0':String(data.labels.length-1);
    if (!data.groups.some(g=>g.id===state.group)) state.group='';
    const opened = new Set((params.get('open') || '').split(',').filter(id=>data.groups.some(g=>g.id===id)));
    if (state.mission) { const m=data.missions.find(m=>m.id===state.mission);if(m)opened.add(m.group);else state.mission=''; }
    const updateUrl = () => {
      const p = new URLSearchParams();
      for (const key of ['q','outcome','approach','group','mission','a','b']) if(state[key]&&state[key]!=='all')p.set(key,state[key]);
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
    const pairCard = (attempt, mission) => {
      if (!attempt) return '';
      const image=attempt.preview?`<img data-preview-src="${esc(attempt.preview)}" width="156" height="92" alt="${esc(attempt.caption)}" decoding="async">`:'<p class="muted">Preview unavailable in this saved projection</p>';
      const link=detailHref(attempt,mission);
      return `<article class="pair-card"><strong>${esc(data.labels[attempt.approach])}</strong><span class="outcome-badge outcome-${esc(attempt.outcome)}">${esc(controls.outcome.querySelector(`option[value="${attempt.outcome}"]`)?.textContent || attempt.outcome)}</span>${link?`<a class="run-preview" href="${link}">${image}</a>`:image}<p class="preview-caption muted">${esc(attempt.caption)}</p></article>`;
    };
    const render = (save=true) => {
      for (const [key,input] of Object.entries(controls))input.value=state[key];
      if(save)updateUrl();
      const selected=data.missions.filter(m=>matches(m,state));
      const bins=new Map(data.groups.map(g=>[g.id,[]]));selected.forEach(m=>bins.get(m.group).push(m));
      let html='';
      for(const group of data.groups){const missions=bins.get(group.id);if(!missions.length)continue;
        const expanded=opened.has(group.id);
        html+=`<tr class="paired-group-row" data-paired-group="${group.id}" tabindex="0" aria-expanded="${expanded}"><td colspan="4"><span class="expander">${expanded?'−':'+'}</span> ${esc(group.label)} <span class="muted">${missions.length} matched missions</span></td></tr>`;
        if(!expanded)continue;
        for(const mission of missions){const open=state.mission===mission.id;const badges=mission.attempts.map(a=>`<span class="mini-outcome outcome-${esc(a.outcome)}" title="${esc(data.labels[a.approach]+': '+a.outcome)}">${esc(a.outcome==='landed'?'L':a.outcome==='unsupported'?'U':a.outcome==='timed-out'?'T':a.outcome==='not-launched'?'N':a.outcome==='crashed'?'C':a.outcome==='failed'?'F':a.outcome==='unverified'?'?':a.outcome==='off-target'?'O':'S')}</span>`).join('');
          html+=`<tr class="paired-mission-row" id="${mission.id}" data-paired-mission="${mission.id}" tabindex="0" aria-expanded="${open}"><td><span class="expander">${open?'−':'+'}</span> Mission ${mission.mission_index}${mission.occurrence?` · repeat ${mission.occurrence+1}`:''}</td><td colspan="3">${badges}</td></tr>`;
          if(!open)continue;
          html+=`<tr class="paired-preview-row"><td colspan="4"><div class="paired-previews">${pairCard(mission.attempts[Number(state.a)],mission)}${state.a!==state.b?pairCard(mission.attempts[Number(state.b)],mission):''}</div><details class="row-evidence"><summary>Physical input and pairing</summary><p><code>${esc(mission.physical_sha256)}</code> · occurrence ${mission.occurrence+1}. Pairing uses physical identity and scheduled occurrence, without best-repeat selection.</p></details></td></tr>`;
          for(const attempt of mission.attempts){const href=detailHref(attempt,mission);html+=`<tr class="paired-attempt-row" data-approach="${attempt.approach}" data-outcome="${esc(attempt.outcome)}"><td>${href?`<a class="paired-detail" href="${href}">${esc(data.labels[attempt.approach])}</a>`:esc(data.labels[attempt.approach])}<span class="row-note">Attempt ${attempt.attempt_index}</span></td>${attempt.cells}</tr>`;}
        }
      }
      tbody.innerHTML=html||'<tr><td colspan="4">No missions match these filters. Reset to review every scheduled input.</td></tr>';
      document.getElementById('review-count').textContent=`${selected.length} / ${data.missions.length} scheduled mission occurrences matched · ${data.labels.length} approaches per mission`;
      window.PdCampaignPreviews?.loadVisible();
      window.dispatchEvent(new CustomEvent('pd-review-updated'));
      return selected.length;
    };
    for(const [key,input] of Object.entries(controls))input.addEventListener(key==='q'?'input':'change',()=>{state[key]=input.value;render();});
    const activate = target => {
      if(target.closest('a,button,input,select,details'))return;
      const group=target.closest('[data-paired-group]');const mission=target.closest('[data-paired-mission]');
      if(group){const id=group.dataset.pairedGroup;if(opened.has(id))opened.delete(id);else opened.add(id);render();}
      else if(mission){state.mission=state.mission===mission.dataset.pairedMission?'':mission.dataset.pairedMission;render();document.getElementById(state.mission)?.focus({preventScroll:true});}
    };
    root.addEventListener('click',e=>activate(e.target));
    root.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){if(e.target.matches('[data-paired-group],[data-paired-mission]')){e.preventDefault();activate(e.target);}}});
    const focus = (group,approach) => {state.group=group;state.approach=String(approach);state.outcome='all';state.q='';state.mission='';opened.clear();opened.add(group);render();root.scrollIntoView({block:'start'});root.querySelector('[data-paired-group]')?.focus({preventScroll:true});};
    document.querySelectorAll('[data-paired-focus]').forEach(button=>button.addEventListener('click',()=>focus(button.dataset.pairedFocus,button.dataset.approach)));
    document.querySelectorAll('[data-review-outcome]').forEach(button=>button.addEventListener('click',()=>{state.outcome=button.dataset.reviewOutcome;state.approach=button.dataset.approach || 'all';state.group='';state.q='';state.mission='';opened.clear();render();root.scrollIntoView({block:'start'});}));
    document.querySelectorAll('[data-paired-action]').forEach(button=>button.addEventListener('click',()=>{if(button.dataset.pairedAction==='reset'){Object.assign(state,{q:'',outcome:'all',approach:'all',group:'',mission:''});}else state.mission='';opened.clear();render();}));
    window.PdPairedReview={data,state,render,focus,matches,opened};
    render(false);
    if(state.mission)document.getElementById(state.mission)?.scrollIntoView({block:'start'});
  };
  document.addEventListener('pd-batch-ready',start,{once:true});
})();
