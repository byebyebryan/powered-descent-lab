/* Mission-led navigation layered onto the maintained catalogue and its filters. */
(() => {
  const families = [
    {id:'terminal',label:'Terminal control',note:'Synthetic arrivals, conditions, arc and velocity cases.'},
    {id:'transfer',label:'Direct transfers',note:'Synthetic source-to-target transfers, route angles and radii.'},
    {id:'procedural',label:'Complete procedural terrain',note:'Complete missions with generated terrain, grouped by family and terrain stratum.'},
    {id:'waypoint',label:'Authored waypoint routes',note:'Approach-specific route and handoff probes.'},
    {id:'planner',label:'Planner probes and terrain studies',note:'Planner-specific diagnostics and retained terrain campaigns.'},
    {id:'other',label:'Other reports',note:'Reports without a recorded mission family.'},
  ];
  // Classify known collection/pack identities, never scores or controller labels.
  const family = (href, topic='') => {
    const p=(href || '').split(/[?#]/)[0].toLowerCase();
    if (/\/(?:procedural-learning|procedural-missions)\//.test(p)) return 'procedural';
    if (/\/(?:terminal[^/]*)(?:\/|$)/.test(p)) return 'terminal';
    if (/\/(?:transfer[^/]*|waypoint[^/]*)(?:\/|$)/.test(p) && /waypoint/.test(p)) return 'waypoint';
    if (/\/transfer[^/]*(?:\/|$)/.test(p)) return 'transfer';
    if (/\/planner[^/]*(?:\/|$)/.test(p) || topic==='waypoint-planning') return 'planner';
    return 'other';
  };
  globalThis.PdMissionNavigationModel={families,family};
  if(typeof document==='undefined')return;
  const esc = s => String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const subjects = document.querySelector('.subject-grid');
  if(subjects){
    const section=subjects.closest('section');
    const navigation=document.createElement('section');navigation.id='mission-navigation';
    const cards = list => list.map(f=>`<article class="subject"><a href="/reports/library/index.html?mission=${f.id}"><h2>${esc(f.label)}</h2><p>${esc(f.note)}</p><span class="action">Browse these missions</span></a></article>`).join('');
    navigation.innerHTML='<h2>Browse missions</h2><p class="library-note">Choose the mission family first, then its collection or capture. Controller comparisons remain inside the mission reports.</p><div class="subject-grid mission-core-grid">'+cards(families.slice(0,3))+'</div><details class="subject-secondary"><summary>Approach-specific mission probes</summary><div class="subject-grid">'+cards(families.slice(3,5))+'</div></details>';
    const hero=document.querySelector('.hero');hero.after(navigation);
    const secondary=document.createElement('details');secondary.className='subject-secondary';secondary.innerHTML='<summary>Browse by approach / subject</summary>';
    section.before(secondary);secondary.append(section);
  }
  // Topic pages preserve their curated hierarchy before chronological discovery.
  const recent=document.getElementById('latest-work');
  const entries=document.querySelector('.entry-list');
  if(recent&&!subjects&&entries&&!entries.closest('details'))entries.after(recent);
  const library=document.getElementById('report-library');
  if(!library)return;
  const intro=document.querySelector('.hero > p');
  if(intro)intro.textContent='Browse mission families, then their saved collections and captures.';
  const notes=document.querySelector('.page > p.library-note');
  if(notes){
    const disclosure=document.createElement('details');disclosure.className='subject-secondary';
    disclosure.innerHTML='<summary>Catalogue notes and evidence labels</summary>';
    notes.replaceWith(disclosure);disclosure.append(notes);library.append(disclosure);
  }
  const container=library.querySelector('.report-rows');
  const rows=[...container.querySelectorAll('.report-row')];
  rows.forEach(row=>{row.dataset.mission=family(row.querySelector('a.report-title')?.getAttribute('href'),row.dataset.topic);});
  const label=document.createElement('label');label.innerHTML='Mission family<select id="mission-filter"><option value="all">All mission families</option>'+families.map(f=>`<option value="${f.id}">${esc(f.label)}</option>`).join('')+'</select>';
  const filters=library.querySelector('.filters');filters.querySelector('label').after(label);
  const select=label.querySelector('select');
  const selected=new URLSearchParams(location.search).get('mission');if(families.some(f=>f.id===selected))select.value=selected;
  const sections=new Map();
  for(const f of families){
    const group=document.createElement('details');group.className='mission-report-group';group.dataset.missionFamily=f.id;
    group.innerHTML=`<summary>${esc(f.label)} <span class="mission-report-count"></span></summary><p class="library-note">${esc(f.note)}</p><div class="report-rows"></div>`;
    if(select.value===f.id)group.open=true;
    sections.set(f.id,group);container.append(group);
  }
  const regroup = () => {
    const topic=document.getElementById('topic-filter').value;
    const type=document.getElementById('type-filter').value;
    const q=document.getElementById('report-search').value.trim().toLowerCase();
    const availability=document.getElementById('availability-filter')?.value || 'all';
    const order=document.getElementById('order-filter')?.value || 'name';
    const ordered=[...rows].sort((a,b)=>order==='newest'?Number(b.dataset.updated||0)-Number(a.dataset.updated||0)||String(a.dataset.name||a.dataset.search).localeCompare(String(b.dataset.name||b.dataset.search)):String(a.dataset.name||a.dataset.search).localeCompare(String(b.dataset.name||b.dataset.search)));
    let total=0;
    for(const row of ordered){
      row.hidden=!(select.value==='all'||row.dataset.mission===select.value)||!(topic==='all'||row.dataset.topic===topic)||!(type==='all'||row.dataset.type===type)||!!(q&&!row.dataset.search.includes(q))||(availability!=='all'&&row.dataset.available!=='true');
      if(!row.hidden)total++;
      sections.get(row.dataset.mission).querySelector('.report-rows').append(row);
    }
    for(const group of sections.values()){
      const n=[...group.querySelectorAll('.report-row')].filter(row=>!row.hidden).length;
      group.hidden=n===0;group.querySelector('.mission-report-count').textContent=`${n} reports`;
      if(select.value!=='all'&&!group.hidden)group.open=true;
      // Restore headings after the maintained order handler temporarily moves rows.
      container.append(group);
    }
    document.getElementById('report-count').textContent=`${total} reports`;
    document.getElementById('report-empty').hidden=total!==0;
  };
  select.addEventListener('change',regroup);
  filters.querySelectorAll('input,select').forEach(input=>{if(input!==select)input.addEventListener(input.tagName==='INPUT'?'input':'change',regroup);});
  regroup();
})();
