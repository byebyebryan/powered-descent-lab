(() => {
  const matches = (mission, state) => {
    if (state.group && mission.group !== state.group && !mission.path?.includes(state.group)) return false;
    const q = (state.q || '').trim().toLowerCase();
    if (q && (/^\d+$/.test(q) ? mission.mission_index !== Number(q) : !`${mission.mission_index} ${mission.label} ${mission.physical_sha256}`.toLowerCase().includes(q))) return false;
    const attempts = state.approach === 'all' ? mission.attempts : mission.attempts.filter(a => String(a.approach) === state.approach);
    return attempts.some(a => state.outcome === 'all' || (state.outcome === 'misses' ? a.outcome !== 'landed' : a.outcome === state.outcome));
  };
  globalThis.PdPairedReviewModel = {matches};
})();
