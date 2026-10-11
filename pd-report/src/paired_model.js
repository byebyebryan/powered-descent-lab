(() => {
  const matches = (mission, state) => {
    if (state.group && mission.group !== state.group && !mission.path?.includes(state.group)) return false;
    const q = (state.q || '').trim().toLowerCase();
    if (q && (/^\d+$/.test(q) ? mission.mission_index !== Number(q) : !`${mission.mission_index} ${mission.label} ${mission.physical_sha256}`.toLowerCase().includes(q))) return false;
    const attempts = state.approach === 'all' ? mission.attempts : mission.attempts.filter(a => String(a.approach) === state.approach);
    return attempts.some(a => state.outcome === 'all' || (state.outcome === 'misses' ? a.outcome !== 'landed' : a.outcome === state.outcome));
  };
  const pageSize = 12;
  // Page a presentation slice without filtering or deduplicating the inventory.
  // A detail return may point to a mission on a different page of this stratum.
  const pageMissions = (missions, requested = 1, selectedId = '') => {
    const pages = Math.ceil(missions.length / pageSize);
    const selected = selectedId ? missions.findIndex(m => m.id === selectedId) : -1;
    const parsed = Number(requested);
    const page = selected >= 0 ? Math.floor(selected / pageSize) + 1
      : Math.min(Math.max(Number.isFinite(parsed) ? Math.trunc(parsed) : 1, 1), Math.max(pages, 1));
    const offset = (page - 1) * pageSize;
    return {page, pages, total: missions.length, start: missions.length ? offset + 1 : 0,
      end: Math.min(offset + pageSize, missions.length), items: missions.slice(offset, offset + pageSize)};
  };
  globalThis.PdPairedReviewModel = {matches, pageMissions, pageSize};
})();
