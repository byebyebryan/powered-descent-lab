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
    subtree: true, attributes: true, attributeFilter: ['hidden', 'aria-expanded'],
  });
  schedule();
})();
