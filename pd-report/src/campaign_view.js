// Load a preview only after its existing review-tree row becomes visible.
(() => {
  const root = document.getElementById('review-tree-root');
  if (!root) return;
  let scheduled = false;
  const update = () => {
    scheduled = false;
    root.querySelectorAll('img[data-preview-src]').forEach(image => {
      if (image.closest('tr')?.hidden || !image.getClientRects().length) return;
      image.loading = 'lazy';
      image.src = image.dataset.previewSrc;
      image.removeAttribute('data-preview-src');
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
