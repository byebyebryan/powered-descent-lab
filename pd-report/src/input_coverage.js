/* Bounded input inspection. Full-population counts never derive from previews. */
(() => {
  const PAGE_SIZE = 12;
  const groupLabel = key => ({mixed:'Mixed morphology',unclassified:'Unclassified',mountain_crossing:'Mountain crossing',broad_massif:'Broad massif',successive_ridges:'Successive ridges',elevation_transition:'Elevation transition'}[key] || key);
  const keyOf = band => JSON.stringify([band.collection, band.group, band.band]);
  const matches = (mission, state) => (state.direction === 'all' || mission.direction === Number(state.direction)) && (!state.q || `${mission.index} ${mission.physical_sha256}`.includes(state.q.toLowerCase()));
  const pageSelection = (data, state) => {
    const selected = data.previews.filter(m => keyOf(m) === state.band && matches(m, state));
    const pages = Math.max(1, Math.ceil(selected.length / PAGE_SIZE));
    const page = Math.min(pages, Math.max(1, Math.floor(Number(state.page)) || 1));
    return {total:selected.length,pages,page,missions:selected.slice((page-1)*PAGE_SIZE,page*PAGE_SIZE)};
  };
  globalThis.PdInputCoverageModel = {PAGE_SIZE,keyOf,matches,pageSelection};
  if (typeof document === 'undefined') return;
  const dataNode = document.getElementById('input-coverage-data');
  if (!dataNode) return;
  const data = JSON.parse(dataNode.textContent);
  const hierarchy = document.getElementById('input-hierarchy');
  const cards = document.getElementById('input-preview-cards');
  const pager = document.getElementById('input-pager');
  const count = document.getElementById('input-review-count');
  const direction = document.getElementById('input-direction');
  const search = document.getElementById('input-search');
  const esc = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const params = new URLSearchParams(location.search);
  const state = {band:params.get('band') || '',page:params.get('page') || 1,direction:params.get('direction') || 'all',q:params.get('q') || ''};
  if (!['all','1','-1'].includes(state.direction)) state.direction='all';
  if (!data.bands.some(b => keyOf(b) === state.band)) state.band='';
  const updateUrl = () => {
    const p = new URLSearchParams();
    if(state.band)p.set('band',state.band);
    if(state.page>1)p.set('page',String(state.page));
    if(state.direction!=='all')p.set('direction',state.direction);
    if(state.q)p.set('q',state.q);
    // file:// previews remain usable without a server or browser history support.
    try { history.replaceState(null,'',location.pathname+(p.size?'?'+p:'')+location.hash); } catch {}
  };
  const renderCards = () => {
    const band = data.bands.find(b => keyOf(b) === state.band);
    const page = pageSelection(data,state); state.page=page.page;
    direction.value=state.direction;search.value=state.q;
    const collection = data.collections.find(c=>c.id===band?.collection);
    count.textContent=band?`${collection.label} → ${groupLabel(band.group)} → ${band.label}. Full band: ${band.admitted} admitted / ${band.proposed} proposals. Selected previews matching filters: ${page.total}. Page ${page.page} of ${page.pages}.`:'Choose a measured band to inspect its selected previews. No preview images load until a band is opened.';
    pager.innerHTML=band?`<button type="button" data-page="previous" ${page.page===1?'disabled':''}>Previous page</button><span>${page.page} / ${page.pages}</span><button type="button" data-page="next" ${page.page===page.pages?'disabled':''}>Next page</button>`:'';
    cards.innerHTML=page.missions.map(m=>`<article class="input-preview-card"><a href="${esc(m.detail_href)}">Mission ${m.index}</a><p class="section-note">${esc(m.disposition)} · physical travel ${m.direction===1?'right':'left'}</p><a href="${esc(m.detail_href)}"><img src="${esc(m.preview_href)}" loading="lazy" width="640" height="300" alt="Raw and prepared terrain for mission ${m.index}, displayed toward the right"></a><p>${esc(m.metrics)}</p><p class="section-note">Fixed physical scale · source at 0 m · gray raw terrain / teal prepared terrain</p><code>${esc(m.physical_sha256.slice(0,16))}</code></article>`).join('');
    hierarchy.querySelectorAll('[data-band]').forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.band===state.band)));
    updateUrl();
  };
  for (const collection of data.collections) {
    const node = document.createElement('section');node.className='input-collection';
    const bands=data.bands.filter(b=>b.collection===collection.id);
    node.innerHTML=`<h3>${esc(collection.label)} · ${collection.admitted} admitted / ${collection.scheduled} scheduled</h3>`;
    for (const group of [...new Set(bands.map(b=>b.group))]) {
      const children=bands.filter(b=>b.group===group);
      const selected=data.previews.filter(m=>m.collection===collection.id&&m.group===group);
      const details=document.createElement('details');details.className='input-group';
      details.open=children.some(b=>keyOf(b)===state.band);
      details.innerHTML=`<summary>${esc(groupLabel(group))} · ${children.reduce((n,b)=>n+b.admitted,0)} admitted / ${children.reduce((n,b)=>n+b.proposed,0)} proposals · ${selected.length} selected previews</summary><div class="input-band-buttons">${children.map(b=>`<button type="button" data-band="${esc(keyOf(b))}" aria-pressed="false">${esc(b.label)} · ${b.admitted} admitted · ${selected.filter(m=>m.band===b.band).length} previews</button>`).join('')}</div>`;
      node.append(details);
    }
    hierarchy.append(node);
  }
  hierarchy.addEventListener('click',event=>{
    const button=event.target.closest('[data-band]');if(!button)return;
    state.band=button.dataset.band;state.page=1;renderCards();
    count.scrollIntoView({block:'start'});
  });
  pager.addEventListener('click',event=>{
    const button=event.target.closest('[data-page]');if(!button||button.disabled)return;
    state.page+=button.dataset.page==='next'?1:-1;renderCards();pager.scrollIntoView({block:'start'});
  });
  direction.addEventListener('change',()=>{state.direction=direction.value;state.page=1;renderCards();});
  search.addEventListener('input',()=>{state.q=search.value.trim();state.page=1;renderCards();});
  document.getElementById('input-collapse').addEventListener('click',()=>{state.band='';state.page=1;renderCards();});
  globalThis.PdInputCoverage={data,state,render:renderCards,get pageInfo(){return pageSelection(data,state);}};
  renderCards();
})();
