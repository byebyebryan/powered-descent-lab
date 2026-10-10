// Additive common-rich-report diagnostics. No reportData mutation or planning.
(() => {
  const data = JSON.parse(document.getElementById('planning-cycle-data').textContent);
  const feedback = data.provenance.startsWith('Ballistic feedback candidate');
  const select = document.getElementById('planning-cycle-select');
  const toggle = document.getElementById('planning-cycle-visible');
  const queryToggle = document.getElementById('planning-cycle-query-visible');
  const plot = document.getElementById('chart-spatial');
  const status = document.getElementById('planning-cycle-plot-status');
  const fmt = v => Number(v).toFixed(2);
  const boundaryText = s => `t=${fmt(s.simTimeS)}s · position (${fmt(s.positionM.x)}, ${fmt(s.positionM.y)})m · velocity (${fmt(s.velocityMps.x)}, ${fmt(s.velocityMps.y)})m/s · attitude ${fmt(s.attitudeRad * 180 / Math.PI)}° · fuel ${fmt(s.fuelKg)}kg`;
  const pathTrace = (name, points, color, dash, width = 2) => ({
    type: 'scatter', mode: 'lines', name,
    x: points.map(p => p.xM), y: points.map(p => p.yM),
    customdata: points.map(p => p.timeS),
    hovertemplate: `${name}<br>t=%{customdata:.2f}s · (%{x:.2f}, %{y:.2f})m<extra></extra>`,
    line: {color, dash, width}, visible: toggle.checked,
  });
  const markerTrace = (name, state, color, symbol) => ({
    type: 'scatter', mode: 'markers', name,
    x: state ? [state.positionM.x] : [], y: state ? [state.positionM.y] : [],
    text: state ? [boundaryText(state)] : [],
    hovertemplate: `${name}<br>%{text}<extra></extra>`,
    marker: {color, symbol, size: 13, line: {width: 2}}, visible: toggle.checked,
  });
  const traces = cycle => [
    pathTrace(feedback ? 'Selected refresh · desired ballistic aim' : 'Selected cycle · recorded nominal', cycle.nominal, '#7950b5', 'dash', 2.5),
    pathTrace(feedback ? 'Selected refresh · short held-command prediction' : 'Selected cycle · unaudited terrain-blind extension', cycle.nominalExtension, '#b9a7d0', 'dot', 1.5),
    pathTrace('Selected cycle · current-state ballistic', cycle.ballistic, '#cf7b00', 'dot', 1.5),
    pathTrace(feedback ? 'Selected refresh · actually flown interval' : 'Selected cycle · executed E → H', cycle.executed, '#2f9e44', 'solid', 3),
    markerTrace('Selected cycle · actual start', cycle.current, '#1864ab', 'diamond-open'),
    markerTrace('Selected cycle · proposed terrain conflict', cycle.conflict, '#c92a2a', 'x'),
    ...(!feedback ? [] : [{
      type: 'scatter', mode: 'markers', name: cycle.goalLabel || 'Active goal',
      x: cycle.activeGoal ? [cycle.activeGoal.x] : [], y: cycle.activeGoal ? [cycle.activeGoal.y] : [],
      marker: {color: '#7950b5', symbol: 'star-open', size: 15}, visible: toggle.checked,
    }, {
      type: 'scatter', mode: 'markers', name: 'Previous goal · not a handoff',
      x: cycle.previousGoal ? [cycle.previousGoal.x] : [], y: cycle.previousGoal ? [cycle.previousGoal.y] : [],
      marker: {color: '#888', symbol: 'star-open', size: 12}, visible: toggle.checked,
    }, {
      type: 'scatter', mode: 'markers', name: 'Blocking ridge crest · terrain feature, not a handoff',
      x: cycle.clearingCrest ? [cycle.clearingCrest.x] : [], y: cycle.clearingCrest ? [cycle.clearingCrest.y] : [],
      hovertemplate: 'Blocking ridge crest<br>(%{x:.2f}, %{y:.2f})m<extra></extra>',
      marker: {color: '#a61e4d', symbol: 'triangle-up-open', size: 14}, visible: toggle.checked,
    }, pathTrace('Predicted cutoff coast · not flown', cycle.cutoffCoast || [], '#087f8c', 'dashdot', 2)]),
    ...[0, 1].flatMap(i => {
      const q = cycle.queries[i];
      return [pathTrace(`Rejected query ${i + 1} · ${q?.rowId || 'none'} · not flown`, q?.path || [], '#777', i ? 'dashdot' : 'dot', 1),
        markerTrace(`Rejected query ${i + 1} · failed guard`, q?.rejectionState, '#777', 'square-open')]
        .map(trace => ({...trace, visible: toggle.checked && queryToggle.checked}));
    }),
  ];
  let traceIndices = null;
  let updating = false;
  let pending = false;
  const updatePlot = async () => {
    if (updating) { pending = true; return; }
    if (!traceIndices) return;
    updating = true;
    try {
      const values = traces(data.cycles[Number(select.value)]);
      await Plotly.restyle(plot, {
        x: values.map(v => v.x), y: values.map(v => v.y), text: values.map(v => v.text || []),
        customdata: values.map(v => v.customdata || []), name: values.map(v => v.name),
        hovertemplate: values.map(v => v.hovertemplate), visible: values.map(v => v.visible),
      }, traceIndices);
    } finally {
      updating = false;
      if (pending) { pending = false; updatePlot(); }
    }
  };
  const update = () => {
    const cycle = data.cycles[Number(select.value)];
    document.getElementById('planning-cycle-state').textContent = boundaryText(cycle.current);
    document.getElementById('planning-cycle-summary').textContent = cycle.summary;
    document.getElementById('planning-cycle-reasons').textContent = JSON.stringify({
      boundaryChecks: cycle.boundaryCounts, nominalRejections: cycle.nominalRejections,
      nominalIdentity: cycle.nominalIdentity, proposedConflictPhase: cycle.conflictPhase,
    }, null, 2);
    const container = document.getElementById('planning-cycle-queries');
    container.replaceChildren();
    for (const q of cycle.queries) {
      const p = document.createElement('p');
      p.textContent = `${q.rowId}: progress requires x ≥ ${fmt(q.progressXM)}m. ${q.rejectionStatus || 'No supported boundary recorded'}${q.handoff ? ` at query H: ${boundaryText(q.handoff)}` : ''}. ${q.rejectionState ? `Failed guard: ${boundaryText(q.rejectionState)}. ` : ''}${q.failedPredicates.join('; ')}. Saved reason: ${q.rejectionReason || q.stopReason}. Trace end: ${q.stopReason}. This query was not executed or accepted.`;
      container.append(p);
    }
    if (!cycle.queries.length) {
      const p = document.createElement('p');
      p.textContent = 'No rejected-query reconstruction for this cycle; aggregate reasons above are saved evidence.';
      container.append(p);
    }
    updatePlot();
  };
  for (const cycle of data.cycles) {
    const option = document.createElement('option');
    option.value = cycle.index;
    option.textContent = `${feedback ? `Refresh ${cycle.index} · t=${fmt(cycle.current.simTimeS)}s` : cycle.index ? `After H${cycle.index}` : 'Launch'} · ${cycle.decision}${cycle.index === data.cycles.length - 1 ? ' · final cycle' : ''}`;
    select.append(option);
  }
  select.value = String(data.cycles.length - 1);
  select.addEventListener('change', update);
  toggle.addEventListener('change', updatePlot);
  queryToggle.addEventListener('change', updatePlot);
  document.getElementById('planning-cycle-provenance').textContent = data.provenance;
  if (feedback) {
    document.getElementById('planning-decision-nav').hidden = false;
    const decisions = document.getElementById('planning-decision-select');
    const placeholder = document.createElement('option');
    placeholder.value = '';
    placeholder.textContent = 'Choose waypoint, obstruction or terrain recovery…';
    decisions.append(placeholder);
    for (const cycle of data.cycles.filter(c => /obstruction|blocked|waypoint_selected|waypoint_replaced|waypoint_reacquired|destination_reacquired_before_waypoint|construction_miss|terrain_recovery_started|terrain_recovery_resumed|coast_through_|coast_terminal_entry|maintained_landing_entry/.test(c.decision))) {
      const option = document.createElement('option');
      option.value = cycle.index;
      option.textContent = `t=${fmt(cycle.current.simTimeS)}s · ${cycle.decision} · ${cycle.goalLabel || 'goal'}`;
      decisions.append(option);
    }
    decisions.value = '';
    decisions.addEventListener('change', () => {
      if (decisions.value === '') return;
      select.value = decisions.value;
      update();
    });
    const panel = document.getElementById('planning-cycle-review');
    panel.querySelector('h2').textContent = 'Ballistic aim · correction refreshes';
    panel.querySelector('p.muted').textContent = 'Dashed purple: desired ideal ballistic aim, not the illustrative launch reference and not flown. Teal: estimated coast after the recorded finite burn cutoff, not flown. Pale purple: the recorded proposed command prediction (may have been rejected), not an acquisition/landing certificate. Orange: current unpowered motion. Green: actually flown interval. Purple star: selected refresh goal; gray star: previous goal before this decision. Red ×: queried obstruction, not an actual crash. Same-goal reacquisition refreshes correction without adding a waypoint. Proposals, replacements and routine refreshes are not waypoint handoffs.';
  }
  update();
  let attempts = 0;
  const attach = async () => {
    if (!globalThis.Plotly || !plot?.data || !plot._fullLayout) {
      if (++attempts < 100) { setTimeout(attach, 100); return; }
      status.textContent = 'Planning data is available above; spatial overlays require the existing Plotly chart to load.';
      return;
    }
    try {
      const start = plot.data.length;
      const initial = traces(data.cycles[Number(select.value)]);
      await Plotly.addTraces(plot, initial);
      traceIndices = initial.map((_v, i) => start + i);
      // Common view presets send visibility for their original traces only.
      // Restore this separately owned overlay after a preset, not after ourselves.
      document.getElementById('spatial-mode-toolbar')?.addEventListener('click', () => setTimeout(updatePlot, 0));
      status.textContent = 'Cycle overlays enabled on the existing trajectory plot. All original views remain available.';
      await updatePlot();
    } catch (error) {
      status.textContent = `Planning overlays unavailable: ${error.message}. Diagnostic data remains available above.`;
    }
  };
  attach();
})();
