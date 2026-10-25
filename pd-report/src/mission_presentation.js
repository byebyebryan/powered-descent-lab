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
    const raw=Number(entry.value);
    const number = raw===0?'0':raw.toPrecision(6).replace(/(?:\.0+|(?:(\.\d*?)0+))(?=e|$)/, '$1');
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
  const compactLabels={
    'study:fuel_used_kg':'Fuel used','study:coast_s':'Coast','study:duration_s':'Burn',
    'study:contact_speed':'Contact speed','study:contact_speed_mps':'Contact speed',
    'study:flight_time':'Time','study:fuel_remaining':'Fuel left','study:endpoint_speed':'End speed',
    fuel_used_kg:'Fuel used',coast_s:'Coast',duration_s:'Burn',
    flight_time:'Time',fuel_remaining:'Fuel left',contact_speed:'Contact speed',
    contact_speed_mps:'Contact speed',endpoint_speed:'End speed'
  };
  const sourceLabel=field=>`${field.label}${field.unit?` · ${field.unit}`:''}`;
  const overviewFields=model=>{
    const keys=model.metric
      ? [`study:${model.metric.id}`,'study:coast_s','study:duration_s']
      : ['flight_time','fuel_remaining','endpoint_speed'];
    return keys.map(key=>model.fields.find(field=>field.key===key)).filter(Boolean).slice(0,3);
  };
  const summarizeGroup=(summary,metric)=>{
    const deltas=(summary?.pairs||[]).map(pair=>pair.delta).filter(Number.isFinite);
    if(!metric||!deltas.length)return {metricId:metric?.id||'',unit:metric?.unit||'',eligible:0,mean:null,min:null,max:null};
    return {
      metricId:metric.id,unit:metric.unit,eligible:deltas.length,
      mean:deltas.reduce((total,value)=>total+value,0)/deltas.length,
      min:Math.min(...deltas),max:Math.max(...deltas)
    };
  };
  const summarizeGroupOutcomes=(missions,state,study)=>{
    const result={total:0,aAvailable:0,bAvailable:0,aLandings:0,bLandings:0};
    if(!globalThis.PdStudyReviewModel?.pair)return result;
    const isLanding=record=>globalThis.PdComparisonReviewModel?.target
      ? globalThis.PdComparisonReviewModel.target(record)==='met'
      : Boolean(record?.verified_landing);
    for(const mission of missions){
      const pair=globalThis.PdStudyReviewModel.pair(mission,state,study);
      if(!pair.inCohort)continue;
      result.total++;
      if(pair.a){result.aAvailable++;if(isLanding(pair.a))result.aLandings++;}
      if(pair.b){result.bAvailable++;if(isLanding(pair.b))result.bLandings++;}
    }
    return result;
  };
  const fieldHeading=(field,compact)=>{
    const full=sourceLabel(field),label=compact?(compactLabels[field.key]||field.label):field.label;
    return `<th scope="col" title="${escape(full)}" aria-label="${escape(full)}">${escape(label)}${field.unit?` · ${escape(field.unit)}`:''}</th>`;
  };
  const previewHtml=(side,viewMode,data,detailHref)=>{
    if (!side.attempt?.preview) return `<span class="mission-preview mission-preview-unavailable">${side.outcome==='unsupported'?'No flight recorded for unsupported input.':'Preview unavailable in this saved projection.'}</span>`;
    let path=side.attempt.preview;
    if(viewMode==='single'&&globalThis.PdComparisonReviewData?.single_preview_suffix)path=path.replace(/\.svg$/,PdComparisonReviewData.single_preview_suffix);
    const image=`<img class="mission-preview-image" data-preview-src="${escape(path)}" width="156" height="92" alt="${escape(side.attempt.caption||side.label)}" decoding="async">`;
    const href=detailHref?.(side.attempt);
    return href?`<a class="run-preview mission-preview" href="${href}" aria-label="Open saved run · ${escape(side.label)}">${image}</a>`:`<span class="run-preview mission-preview">${image}</span>`;
  };
  const sideCard=(side,viewMode,data,detailHref)=>{
    const tag=viewMode==='comparison'?side.key.toUpperCase():'Saved run';
    const accessibleLabel=viewMode==='comparison'?`${tag} · ${side.label}`:`Saved run · ${side.label}`;
    const preview=previewHtml(side,viewMode,data,detailHref);
    return `<article class="mission-run-card ${viewMode==='comparison'?'series-'+side.key:'series-single'}" data-run-approach="${side.approach}" data-run-outcome="${escape(side.outcome)}"><div class="mission-run-heading"><strong title="${escape(side.label)}" aria-label="${escape(accessibleLabel)}">${tag}</strong></div><div class="mission-run-preview">${preview}</div></article>`;
  };
  const renderRows=(model,fields,{compact=false,overview=false}={})=>{
    const emptyNote=!fields.length?'<p class="mission-run-note">Structured measurements are unavailable in this saved projection.</p>':'';
    const comparison=model.mode==='comparison';
    const headings=`<th scope="col">Run</th><th scope="col">Outcome</th>${fields.map(field=>fieldHeading(field,compact)).join('')}`;
    const rows=model.sides.map((side,index)=>{
      const identity=comparison?side.key.toUpperCase():'Saved run';
      const accessible=comparison?`${side.key.toUpperCase()} · ${side.label}`:`Saved run · ${side.label}`;
      const values=fields.map(field=>`<td data-label="${escape(compact?(compactLabels[field.key]||field.label):field.label)}" title="${escape(sourceLabel(field))}" data-value-key="${escape(field.key)}">${formatValue(field.values[index])}</td>`).join('');
      return `<tr${overview?` data-run-role="${side.key}"`:''} data-run-approach="${side.approach}"><th scope="row" data-label="Run" title="${escape(side.label)}" aria-label="${escape(accessible)}">${identity}</th><td data-label="Outcome"><span class="outcome-badge outcome-${escape(side.outcome)}">${escape(outcomeLabels[side.outcome]||side.outcome||'Unavailable')}</span></td>${values}</tr>`;
    }).join('');
    const tableClass=overview?'mission-run-values mission-run-values-overview':'mission-run-values mission-run-values-full';
    return `${emptyNote}<div class="mission-run-values-wrap"><table class="${tableClass}"><thead><tr>${headings}</tr></thead><tbody>${rows}</tbody></table></div>`;
  };
  const transitionLabels={preserved:'Preserved',gained:'Gained',lost:'Lost',missed:'Neither landed',unpaired:'Unavailable comparison'};
  const resultLine=(model)=>{
    if(model.mode!=='comparison')return '';
    if(!model.metric&&!model.transition)return '';
    const transition=model.transition
      ? transitionLabels[model.transition]||model.transition
      : `${outcomeLabels[model.sides[0].outcome]||model.sides[0].outcome} → ${outcomeLabels[model.sides[1].outcome]||model.sides[1].outcome}`;
    const delta=model.metric
      ? model.studyPair?.eligible
        ? ` · Δ ${compactLabels[`study:${model.metric.id}`]||model.metric.label} (B−A) ${formatValue({value:model.studyPair.delta,unit:model.metric.unit})}`
        : ` · Δ ${compactLabels[`study:${model.metric.id}`]||model.metric.label} (B−A) unavailable`
      : '';
    return `<p class="mission-result-line">${escape(transition)}${delta}</p>`;
  };
  const targetLabel=(side,observation,comparisonReview)=>{
    if(!comparisonReview||!globalThis.PdComparisonReviewModel)return '';
    const result=globalThis.PdComparisonReviewModel.target(observation||{
      outcome:side.outcome,verified_landing:side.outcome==='landed'
    });
    return ` · native landing target ${escape(result)}`;
  };
  const detailFields=(model,study)=>{
    const fields=new Map(model.fields.map(field=>[field.key,field]));
    if(!study||!model.studyPair)return [...fields.values()];
    for(const metric of study.metrics||[]){
      const key=`study:${metric.id}`;
      if(fields.has(key))continue;
      const a=metricEntry(model.studyPair.a,metric),b=metricEntry(model.studyPair.b,metric);
      if(metric.id!==model.metric?.id&&!a&&!b)continue;
      fields.set(key,{
        key,label:metric.label,unit:metric.unit,study:true,metric,
        values:model.sides.map((_,index)=>metricEntry(model.studyPair[index===0?'a':'b'],metric))
      });
    }
    return [...fields.values()];
  };
  const renderDetails=(model,{runPicker='',pairHref='',comparisonReview=null,study=null}={})=>{
    const comparison=model.mode==='comparison';
    const context=model.sides.map((side,index)=>{
      const identity=comparison?side.key.toUpperCase():'Saved run';
      const label=comparison?`${identity} · ${side.label}`:`Saved run · ${side.label}`;
      const observation=model.studyPair?.[index===0?'a':'b'];
      return `<p class="mission-full-outcome"><strong title="${escape(side.label)}">${escape(label)}</strong> · ${escape(outcomeLabels[side.outcome]||side.outcome||'Unavailable')}${targetLabel(side,observation,comparisonReview)}</p>`;
    }).join('');
    const captions=model.sides.map(side=>side.attempt?.caption
      ? `<p class="mission-full-caption"><strong>${comparison?side.key.toUpperCase():'Saved run'}</strong> · ${escape(side.attempt.caption)}</p>`
      : '').join('');
    const legacy=model.sides.map(side=>side.attempt?.cells
      ? `<details class="legacy-run-summary"><summary>${comparison?side.key.toUpperCase()+' · ':''}Legacy saved run summary</summary><table class="legacy-run-cells"><tbody><tr>${side.attempt.cells}</tr></tbody></table></details>`
      : '').join('');
    const pairLink=comparison&&pairHref?`<a class="compare-pair" href="${pairHref}">Open paired inspection page</a>`:'';
    return `<section class="mission-full-details"><h3>Full recorded values and context</h3>${context}${renderRows(model,detailFields(model,study))}${captions}${legacy}${runPicker}${pairLink}</section>`;
  };
  const render=(mission,state,data,study,{detailHref=()=>'',pairHref='',comparisonReview=globalThis.PdComparisonReviewData}={})=>{
    const model=build(mission,state,data,study);
    const cards=model.sides.map(side=>sideCard(side,model.mode,data,detailHref)).join('');
    const compact=overviewFields(model);
    const summary=resultLine(model);
    const noOp=model.earlierNoOp?'<p class="mission-earlier-note">Pinned earlier edition: same captures and zero scientific changes; no behavioral improvement is claimed.</p>':'';
    const runPicker=model.mode==='single'&&model.runCandidates.length>1?`<label class="mission-run-picker">Run within this occurrence<select data-mission-run="${escape(mission.id)}">${model.runCandidates.map(a=>`<option value="${a.attempt_index}"${String(a.attempt_index)===String(state.run)&&state.mission===mission.id?' selected':''}>Attempt ${a.attempt_index}</option>`).join('')}</select></label>`:'';
    return {
      model,
      html:`<section class="mission-presentation mission-presentation-${model.mode}" data-presentation-mode="${model.mode}"><div class="mission-run-grid ${model.mode==='single'?'single':''}">${cards}</div><div class="mission-results">${renderRows(model,compact,{compact:true,overview:true})}${summary}${noOp}</div></section>`,
      detailsHtml:renderDetails(model,{runPicker,pairHref,comparisonReview,study})
    };
  };
  globalThis.PdMissionPresentation={mode,build,render,formatValue,summarizeGroup,summarizeGroupOutcomes};
})();
