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
    document.querySelectorAll('[data-review-outcome]').forEach(button => button.addEventListener('click', () => {
      rows.forEach(row => row.classList.toggle('review-filter-hidden', row.dataset.outcome !== button.dataset.reviewOutcome));
      tree?.expandLeaves();root.scrollIntoView({block:'start'});
    }));
    document.querySelectorAll('[data-campaign-reset]').forEach(button => button.addEventListener('click', () => {rows.forEach(r=>r.classList.remove('review-filter-hidden'));tree?.collapseGroups(0);}));
  }, {once:true});
  schedule();
})();
