// Load previews near the viewport, including after opening a tree/disclosure.
(() => {
  const root = document.getElementById('review-tree-root');
  if (!root) return;
  let scheduled = false;
  const watched = new Set();
  const load = image => {
    if (!image.isConnected || !image.dataset.previewSrc || !image.getClientRects().length) return;
    image.loading = 'lazy';
    image.src = image.dataset.previewSrc;
    image.removeAttribute('data-preview-src');
    observer?.unobserve(image);
    watched.delete(image);
  };
  const observer = typeof IntersectionObserver === 'undefined' ? null : new IntersectionObserver(entries => {
    entries.forEach(entry => {if (entry.isIntersecting) load(entry.target);});
  }, {rootMargin: '200px 0px'});
  const update = () => {
    scheduled = false;
    watched.forEach(image => {
      if (!image.isConnected || !image.dataset.previewSrc) {observer?.unobserve(image);watched.delete(image);}
    });
    root.querySelectorAll('img[data-preview-src]').forEach(image => {
      if (observer) {
        if (!watched.has(image)) {watched.add(image);observer.observe(image);}
      } else {
        const box = image.getBoundingClientRect();
        if (box.bottom >= -200 && box.top <= innerHeight + 200 && box.right > 0 && box.left < innerWidth) load(image);
      }
    });
  };
  const schedule = () => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(update);
  };
  new MutationObserver(schedule).observe(root, {
    subtree: true, childList: true, attributes: true, attributeFilter: ['hidden', 'aria-expanded', 'open'],
  });
  window.PdCampaignPreviews = {loadVisible: schedule};
  if (!observer) {window.addEventListener('scroll', schedule, {passive:true});window.addEventListener('resize', schedule);}
  document.addEventListener('pd-batch-ready', () => {
    if (document.getElementById('paired-review-data')) return;
    const tree = root.__pdBatchTreeApi;
    const rows = [...root.querySelectorAll('tr.campaign-row')];
    const direction = document.getElementById('campaign-direction');
    const selected = new URLSearchParams(location.search).get('direction') || 'all';
    if ([...direction.options].some(o=>o.value===selected)) direction.value=selected;
    const updateFilter = () => {
      rows.forEach(row=>row.classList.toggle('review-direction-hidden',direction.value!=='all'&&row.dataset.direction!==direction.value));
      const count=rows.filter(row=>!row.classList.contains('review-direction-hidden')&&!row.classList.contains('review-filter-hidden')).length;
      document.getElementById('campaign-filter-count').textContent=`${count} / ${rows.length} scheduled attempts match the filters. Summary totals cover the full collection.`;
      if(direction.value!=='all')direction.closest('details').open=true;
      schedule();
    };
    direction.addEventListener('change',()=>{const url=new URL(location.href);if(direction.value==='all')url.searchParams.delete('direction');else url.searchParams.set('direction',direction.value);history.replaceState(null,'',url.pathname+url.search+url.hash);updateFilter();});
    updateFilter();
    document.querySelectorAll('[data-review-outcome]').forEach(button => button.addEventListener('click', () => {
      rows.forEach(row => row.classList.toggle('review-filter-hidden', row.dataset.outcome !== button.dataset.reviewOutcome));
      updateFilter();
      tree?.expandLeaves();root.scrollIntoView({block:'start'});
    }));
    document.querySelectorAll('[data-campaign-reset]').forEach(button => button.addEventListener('click', () => {rows.forEach(r=>r.classList.remove('review-filter-hidden'));direction.value='all';direction.dispatchEvent(new Event('change'));tree?.collapseGroups(0);}));
  }, {once:true});
  schedule();
})();
