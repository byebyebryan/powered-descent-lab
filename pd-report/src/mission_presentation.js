/* Shared renderer-only mission leaf model and display for single and paired runs. */
(() => {
  const escape = value => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const outcomeLabels = {
    landed:'Verified landing',crashed:'Crashed','timed-out':'Timed out',stopped:'Stopped airborne',
    'not-launched':'Stopped before departure','off-target':'Landed off target',unsupported:'Unsupported',
    failed:'Execution / capture failure',unverified:'Unverified outcome','not-scheduled':'Not scheduled',
    missing:'Missing saved run',unpaired:'Unavailable comparison'
  };
  const formatValue = entry => {
    if (!entry) return 'Unavailable';
    if (entry.text !== null && entry.text !== undefined) return escape(entry.text);
    if (!Number.isFinite(entry.value)) return 'Unavailable';
    const number = Number(entry.value).toPrecision(6).replace(/(?:\.0+|(?:(\.\d*?)0+))(?=e|$)/, '$1');
    return `${escape(number)}${entry.unit ? ` ${escape(entry.unit)}` : ''}`;
  };
  const mode = (data, study) => study ? 'comparison'
    : data.presentation?.schema_version === 1 && data.presentation.mode === 'comparison' ? 'comparison' : 'single';
  const attemptFor = (mission, approach) => mission.attempts.find(a => a.approach === approach) || null;
  const metricEntry = (observation, metric) => observation && Number.isFinite(observation.values?.[metric.id]) ? ({
    key:`study:${metric.id}`,label:metric.label,
    value:observation.values[metric.id],unit:metric.unit,
    text:undefined,study:true
  }) : null;
  const displayEntries = attempt => (attempt?.display_values || []).map(v => ({...v,study:false}));
  const mergeFields = (sides, study, state, studyPair) => {
    const fields = new Map();
    const add = field => {if(!fields.has(field.key))fields.set(field.key,field);};
    for (const side of sides) for (const row of displayEntries(side.attempt))
      add({key:row.key,label:row.label,unit:row.unit,study:false});
    if (study && studyPair) {
      const ids = [state.metric,'coast_s','duration_s'].filter((id,index,list)=>id&&list.indexOf(id)===index);
      for (const id of ids) {
        const metric=study.metrics.find(m=>m.id===id);
        if (!metric) continue;
        const a=studyPair.a?.values?.[id],b=studyPair.b?.values?.[id];
        if (id!=='coast_s'&&id!=='duration_s'&&id!==state.metric&&a===undefined&&b===undefined) continue;
        if (id==='coast_s'||id==='duration_s') {
          if (!Number.isFinite(a)&&!Number.isFinite(b)) continue;
        }
        add({key:`study:${id}`,label:metric.label,unit:metric.unit,study:true,metric});
      }
    }
    return [...fields.values()].map(field=>({...field,values:sides.map((side,index)=>field.study
      ? metricEntry(studyPair?.[index===0?'a':'b'],field.metric)
      : displayEntries(side.attempt).find(v=>v.key===field.key)||null)}));
  };
  const build = (mission,state,data,study) => {
    const viewMode=mode(data,study);
    if (viewMode==='comparison') {
      let aIndex=Number(state.a),bIndex=Number(state.b),studyPair=null,labels;
      if (study) {
        const contrast=study.contrasts.find(c=>c.id===state.contrast)||study.contrasts.find(c=>c.id===study.default_contrast);
        if (contrast) {
          aIndex=study.variants.findIndex(v=>v.id===(state.basis==='earlier'?contrast.candidate:contrast.baseline));
          bIndex=study.variants.findIndex(v=>v.id===contrast.candidate);
        }
        studyPair=globalThis.PdStudyReviewModel?.pair(mission,state,study)||null;
        labels=globalThis.PdComparisonReviewModel?.label(state,study)||{
          a:study.variants[aIndex]?.label||'A',b:study.variants[bIndex]?.label||'B'
        };
      } else {
        labels={a:data.labels[aIndex]||`Run ${aIndex+1}`,b:data.labels[bIndex]||`Run ${bIndex+1}`};
      }
      const observations=studyPair?[studyPair.a,studyPair.b]:[null,null];
      const sides=[aIndex,bIndex].map((approach,index)=>{
        const observation=observations[index];
        const attempt=study?(observation?attemptFor(mission,approach):null):attemptFor(mission,approach);
        return {key:index===0?'a':'b',approach,label:labels[index===0?'a':'b'],attempt,observation,
          outcome:observation?.outcome||attempt?.outcome||'not-scheduled',color:index===0?'#356a96':'#087f73'};
      });
      const metric=study?.metrics.find(m=>m.id===state.metric);
      const fields=mergeFields(sides,study,state,studyPair);
      return {mode:viewMode,sides,fields,studyPair,metric,
        transition:studyPair?.category||null,
        earlierNoOp:state.basis==='earlier'&&Boolean(globalThis.PdComparisonReviewData?.earlier?.same_captures_verified)};
    }
    const view=Number(state.view)||0;
    const candidates=mission.attempts.filter(a=>a.approach===view);
    const selected=state.mission===mission.id&&state.run
      ? candidates.find(a=>String(a.attempt_index)===String(state.run))||candidates[0]
      : candidates[0];
    const label=data.labels[view]||data.labels[0]||'Saved run';
    const side={key:'single',approach:selected?.approach??view,label,attempt:selected||null,
      observation:null,outcome:selected?.outcome||'not-scheduled',color:'#356a96'};
    return {mode:viewMode,sides:[side],fields:mergeFields([side],null,state,null),studyPair:null,metric:null,transition:null,runCandidates:candidates};
  };
  const previewHtml=(side,viewMode,data,detailHref)=>{
    if (!side.attempt?.preview) return `<span class="mission-preview mission-preview-unavailable">${side.outcome==='unsupported'?'No flight recorded for unsupported input.':'Preview unavailable in this saved projection.'}</span>`;
    let path=side.attempt.preview;
    if(viewMode==='single'&&globalThis.PdComparisonReviewData?.single_preview_suffix)path=path.replace(/\.svg$/,PdComparisonReviewData.single_preview_suffix);
    const image=`<img class="mission-preview-image" data-preview-src="${escape(path)}" width="156" height="92" alt="${escape(side.attempt.caption||side.label)}" decoding="async">`;
    const href=detailHref?.(side.attempt);
    return href?`<a class="run-preview mission-preview" href="${href}">${image}</a>`:`<span class="run-preview mission-preview">${image}</span>`;
  };
  const sideCard=(side,viewMode,data,detailHref)=>{
    const missing=!side.attempt;
    const outcome=outcomeLabels[side.outcome]||side.outcome||'Unavailable';
    const title=viewMode==='comparison'?`${side.key.toUpperCase()} · ${side.label}`:side.label;
    const preview=previewHtml(side,viewMode,data,detailHref);
    const caption=side.attempt?.caption?`<p class="preview-caption muted">${escape(side.attempt.caption)}${viewMode==='comparison'?' · shared case frame':''}</p>`:'';
    return `<article class="mission-run-card ${viewMode==='comparison'?'series-'+side.key:'series-single'}" data-run-approach="${side.approach}" data-run-outcome="${escape(side.outcome)}"><div class="mission-run-heading"><strong>${escape(title)}</strong><span class="outcome-badge outcome-${escape(side.outcome)}">${escape(outcome)}</span></div><div class="mission-run-preview">${preview}${caption}</div>${missing&&side.observation?`<p class="mission-run-note">${escape(outcome)}; no saved trajectory is available.</p>`:''}</article>`;
  };
  const renderRows=(model)=>{
    const emptyNote=!model.fields.length?'<p class="mission-run-note">Structured measurements are unavailable in this saved projection.</p>':'';
    const comparison=model.mode==='comparison';
    const headings=`<th scope="col">Run</th><th scope="col">Outcome</th>${model.fields.map(field=>`<th scope="col">${escape(field.label)}${field.unit?` · ${escape(field.unit)}`:''}</th>`).join('')}`;
    const rows=model.sides.map((side,index)=>{
      const identity=comparison?`${side.key.toUpperCase()} · ${side.label}`:side.label;
      const values=model.fields.map(field=>`<td data-label="${escape(field.label)}">${formatValue(field.values[index])}</td>`).join('');
      return `<tr data-run-role="${side.key}" data-run-approach="${side.approach}"><th scope="row" data-label="Run">${escape(identity)}</th><td data-label="Outcome"><span class="outcome-badge outcome-${escape(side.outcome)}">${escape(outcomeLabels[side.outcome]||side.outcome||'Unavailable')}</span></td>${values}</tr>`;
    }).join('');
    return `${emptyNote}<div class="mission-run-values-wrap"><table class="mission-run-values"><thead><tr>${headings}</tr></thead><tbody>${rows}</tbody></table></div>`;
  };
  const render=(mission,state,data,study,{detailHref=()=>'',pairHref='',comparisonReview=globalThis.PdComparisonReviewData}={})=>{
    const model=build(mission,state,data,study),comparison=model.mode==='comparison';
    const cards=model.sides.map(side=>sideCard(side,model.mode,data,detailHref)).join('');
    const outcomeSummary=comparison
      ? `<p class="mission-comparison-summary">${model.transition?`Outcome transition: ${escape(model.transition)} · `:''}${escape(model.sides[0].label)} ${escape(outcomeLabels[model.sides[0].outcome]||model.sides[0].outcome)} → ${escape(model.sides[1].label)} ${escape(outcomeLabels[model.sides[1].outcome]||model.sides[1].outcome)}${comparisonReview?` · native landing target A ${escape(PdComparisonReviewModel.target(model.studyPair?.a||{outcome:model.sides[0].outcome,verified_landing:model.sides[0].outcome==='landed'}))}, B ${escape(PdComparisonReviewModel.target(model.studyPair?.b||{outcome:model.sides[1].outcome,verified_landing:model.sides[1].outcome==='landed'}))}`:''}</p>`
      : '';
    const delta=model.metric?`<p class="mission-selected-delta">Δ (B−A) · ${escape(model.metric.label)}: ${model.studyPair?.eligible?`${Number(model.studyPair.delta).toPrecision(6)} ${escape(model.metric.unit)}`:'Unavailable for this pair'}</p>`:'';
    const noOp=model.earlierNoOp?'<p class="mission-run-note">Pinned earlier edition: same captures and zero scientific changes; no behavioral improvement is claimed.</p>':'';
    const runPicker=model.mode==='single'&&model.runCandidates.length>1?`<label class="mission-run-picker">Run within this occurrence<select data-mission-run="${escape(mission.id)}">${model.runCandidates.map(a=>`<option value="${a.attempt_index}"${String(a.attempt_index)===String(state.run)&&state.mission===mission.id?' selected':''}>Attempt ${a.attempt_index}</option>`).join('')}</select></label>`:'';
    const legacy=model.sides.map(side=>side.attempt?.display_values?.length?'' : side.attempt?.cells?`<details class="legacy-run-summary"><summary>Legacy saved run summary</summary><table class="legacy-run-cells"><tbody><tr>${side.attempt.cells}</tr></tbody></table></details>`:'').join('');
    const pairLink= comparison&&pairHref?`<a class="compare-pair" href="${pairHref}">Open paired inspection page</a>`:'';
    return {model,html:`<section class="mission-presentation mission-presentation-${model.mode}" data-presentation-mode="${model.mode}"><div class="mission-run-grid ${model.mode==='single'?'single':''}">${cards}</div>${outcomeSummary}${renderRows(model)}${delta}${runPicker}${legacy}${noOp}${pairLink}</section>`};
  };
  globalThis.PdMissionPresentation={mode,build,render,formatValue};
})();
