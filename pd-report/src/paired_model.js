(() => {
  const matches = (mission, state) => {
    if (globalThis.PdStudyProjection && !globalThis.PdStudyReviewModel.matches(mission,state,globalThis.PdStudyProjection)) return false;
    if (state.group && mission.group !== state.group && !mission.path?.includes(state.group)) return false;
    if (state.direction && state.direction !== 'all' && (mission.direction_unavailable ? 'unavailable' : String(mission.direction)) !== state.direction) return false;
    const q = (state.q || '').trim().toLowerCase();
    if (q && (/^\d+$/.test(q) ? mission.mission_index !== Number(q) : !`${mission.mission_index} ${mission.label} ${mission.physical_sha256}`.toLowerCase().includes(q))) return false;
    const attempts = state.approach === 'all' ? mission.attempts : mission.attempts.filter(a => String(a.approach) === state.approach);
    return attempts.some(a => state.outcome === 'all' || (state.outcome === 'misses' ? a.outcome !== 'landed' : a.outcome === state.outcome));
  };
  // Old coverage selectors encode direction. Preserve their exact subset while
  // exposing its merged stratum and direction as independent view controls.
  const normalizeScope = (state, groups = []) => {
    const coverage = groups.find(g => g.id === state.group && g.branch);
    return coverage ? {...state, group:coverage.branch,
      direction:state.direction && state.direction !== 'all' ? state.direction : coverage.direction_unavailable ? 'unavailable' : String(coverage.direction)} : {...state};
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
  // Browsing keeps the active hierarchy visible without stacking its siblings.
  // The full tree remains available for deliberately choosing another group.
  const visibleBranches = (branches, activeId = '', showTree = false) => {
    if (!activeId || showTree) return branches;
    const byId = new Map(branches.map(b => [b.id, b]));
    if (!byId.has(activeId)) return branches;
    const active = new Set();
    for (let b = byId.get(activeId); b && !active.has(b.id); b = byId.get(b.parent)) active.add(b.id);
    return branches.filter(b => active.has(b.id));
  };
  // Closing a branch removes only that branch and its descendants. Parents and
  // independently expanded siblings retain their state in either tree view.
  const collapseBranch = (branches, opened, id) => {
    const byId = new Map(branches.map(b => [b.id, b]));
    return new Set([...opened].filter(openId => {
      const visited = new Set();
      for (let b = byId.get(openId); b && !visited.has(b.id); b = byId.get(b.parent)) {
        if (b.id === id) return false;
        visited.add(b.id);
      }
      return true;
    }));
  };
  globalThis.PdPairedReviewModel = {matches, normalizeScope, pageMissions, pageSize, visibleBranches, collapseBranch};
})();
