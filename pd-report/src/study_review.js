/* The study is a lens over the maintained mission browser, not another inventory. */
(() => {
  const esc=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const fmt=n=>Number.isFinite(n)?(n!==0&&Math.abs(n)<.001?n.toPrecision(4):n.toLocaleString(undefined,{maximumFractionDigits:3})):'Unavailable';
  const quantile=(v,p)=>{const a=[...v].sort((a,b)=>a-b),x=(a.length-1)*p,i=Math.floor(x);return a[i]+(a[Math.ceil(x)]-a[i])*(x-i);};
  let initialized=false,request=0;const evidenceCache=new Map();
  const changeLabels={preserved:'Preserved landings',gained:'Gained landings',lost:'Lost landings',missed:'Neither landed',unpaired:'Unavailable comparison'};
  const selected=()=>{const r=window.PdPairedReview;return r.data.missions.filter(m=>r.matches(m,r.state));};
  const apply=(patch,open=false,missionId='')=>{
    const r=window.PdPairedReview,s=window.PdStudyProjection;
    Object.assign(r.state,patch,{q:'',group:'',outcome:'all',approach:'all',direction:'all',stratum:'',page:1,mission:''});r.opened.clear();
    Object.assign(r.state,PdStudyReviewModel.normalize(r.state,s));
    const contrast=s.contrasts.find(c=>c.id===r.state.contrast);
    r.state.a=String(s.variants.findIndex(v=>v.id===contrast.baseline));r.state.b=String(s.variants.findIndex(v=>v.id===contrast.candidate));r.state.view=r.state.b;
    if(missionId){const m=r.data.missions.find(m=>m.id===missionId);if(m){r.state.q=String(m.mission_index);r.state.mission=m.id;}}
    if(open){const first=r.data.missions.find(m=>r.matches(m,r.state));if(first){r.state.stratum=(first.path||[first.group]).at(-1);(first.path||[first.group]).forEach(id=>r.opened.add(id));}}
    r.render();if(open)document.getElementById('review-tree-root').scrollIntoView({block:'start'});
  };
  const initialize=()=>{
    if(initialized)return;initialized=true;
    const s=window.PdStudyProjection,controls=document.getElementById('study-controls');
    const select=(id,label,items)=>`<label>${label}<select id="study-${id}">${items.map(o=>`<option value="${esc(o.id)}">${esc(o.label)}</option>`).join('')}</select></label>`;
    controls.innerHTML=select('cohort','Population / role',s.cohorts)+select('contrast','Baseline → candidate',s.contrasts)+select('metric','Difference metric',s.metrics)+`<label class="study-range">Δ minimum<input id="study-lo" type="number" step="any" aria-label="Minimum metric difference"></label><label class="study-range">Δ maximum<input id="study-hi" type="number" step="any" aria-label="Maximum metric difference"></label><button type="button" id="study-range">Select range</button><button type="button" id="study-clear">Clear selection</button><button type="button" id="study-browse">Browse selected missions</button>`;
    for(const key of ['cohort','contrast','metric'])document.getElementById('study-'+key).addEventListener('change',event=>apply({[key]:event.target.value,change:'all',lo:'',hi:'',cell:''}));
    document.getElementById('study-range').addEventListener('click',()=>{const lo=document.getElementById('study-lo').value,hi=document.getElementById('study-hi').value;if(lo!==''&&hi!==''&&Number(lo)>Number(hi)){document.getElementById('study-lo').setCustomValidity('Minimum must not exceed maximum');document.getElementById('study-lo').reportValidity();return;}document.getElementById('study-lo').setCustomValidity('');apply({lo,hi},true);});
    document.getElementById('study-clear').addEventListener('click',()=>apply({change:'all',lo:'',hi:'',cell:''}));
    document.getElementById('study-browse').addEventListener('click',()=>apply({},true));
    document.getElementById('study-overview').addEventListener('click',event=>{
      const change=event.target.closest('[data-study-change]'),cell=event.target.closest('[data-study-cell]'),mission=event.target.closest('[data-study-mission]');
      if(change)apply({change:change.dataset.studyChange,lo:'',hi:'',cell:''},true);
      if(cell)apply({cell:cell.dataset.studyCell,change:'all',lo:'',hi:''},true);
      if(mission)apply({change:'all',lo:'',hi:'',cell:''},true,mission.dataset.studyMission);
    });
    document.getElementById('study-overview').addEventListener('keydown',event=>{if((event.key==='Enter'||event.key===' ')&&event.target.matches('circle[data-study-mission]')){event.preventDefault();apply({change:'all',lo:'',hi:'',cell:''},true,event.target.dataset.studyMission);}});
    document.getElementById('study-definitions').innerHTML=`<div class="study-definitions">${s.metrics.map(m=>`<p><strong>${esc(m.label)} · ${esc(m.unit)}</strong>: ${esc(m.measurement)}. Difference is candidate minus baseline; ${m.lower_is_better===null?'direction is descriptive, without a favourable ordering':(m.lower_is_better?'lower':'higher')+' is favourable'}. ${m.successful_pairs_only?'Only pairs with two verified landings are eligible.':'Requires recorded values on both sides.'}</p>`).join('')}${s.cohorts.map(c=>`<p><strong>${esc(c.label)}</strong>: ${c.originals} original scheduled occurrences · ${esc(c.role)}.</p>`).join('')}</div>`;
    window.PdStudyReview={apply,selected};
  };
  const render=()=>{
    const r=window.PdPairedReview,s=window.PdStudyProjection;if(!r||!s)return;initialize();
    const state=r.state,cohort=s.cohorts.find(c=>c.id===state.cohort),contrast=s.contrasts.find(c=>c.id===state.contrast),metric=s.metrics.find(m=>m.id===state.metric);
    for(const key of ['cohort','contrast','metric','lo','hi'])document.getElementById('study-'+key).value=state[key]||'';
    const all=PdStudyReviewModel.summary(r.data.missions,state,s),visible=selected(),subset=PdStudyReviewModel.summary(visible,state,s);
    const base=s.variants.find(v=>v.id===contrast.baseline).label,candidate=s.variants.find(v=>v.id===contrast.candidate).label;
    document.getElementById('study-scope').textContent=`${cohort.label} · ${base} → ${candidate} · ${visible.length}/${all.total} original occurrences selected · ${subset.pairs.length} eligible metric pairs. Δ = candidate − baseline (${metric.unit}). ${state.change!=='all'?changeLabels[state.change]+'. ':''}${state.cell?'Condition '+state.cell+'. ':''}Exploratory subset; frozen claims above remain unchanged.`;
    document.querySelectorAll('[data-study-tree-context]').forEach(node=>{node.textContent=`${cohort.label} · ${base} → ${candidate} · ${state.change==='all'?'all outcomes':changeLabels[state.change]} · ${visible.length}/${all.total} originals · ${metric.label} (${metric.unit})`;});
    document.getElementById('study-outcomes').innerHTML=`<p class="study-mini">Full selected cohort: ${all.total} originals. Select a cell to inspect its exact missions.</p><div class="study-matrix">${['preserved','lost','gained','missed'].map(k=>`<button type="button" class="${k}" data-study-change="${k}" aria-pressed="${state.change===k}"><span>${changeLabels[k]}</span><strong>${all.counts[k]}</strong></button>`).join('')}</div><p class="study-mini"><button type="button" data-study-change="unpaired" aria-pressed="${state.change==='unpaired'}">Unavailable / unscheduled comparison: ${all.counts.unpaired}</button></p>`;
    document.getElementById('study-metric-title').textContent=`${metric.label}: how large is the change?`;
    const bound=Math.max(1e-6,...all.pairs.map(p=>Math.abs(p.delta)))*1.08,x=d=>250+205*d/bound;
    const chosen=new Set(visible.map(m=>m.id));
    const points=all.pairs.map(p=>{const good=metric.lower_is_better?p.delta<0:p.delta>0,color=p.delta===0?'#827a70':metric.lower_is_better===null?'#356a96':good?'#087f73':'#ae382d';const y=45+(p.mission.mission_index*37%137);return `<circle cx="${x(p.delta).toFixed(3)}" cy="${y}" r="3.5" fill="${color}" opacity="${chosen.has(p.mission.id)?'.8':'.12'}" tabindex="0" role="button" data-study-mission="${esc(p.mission.id)}" aria-label="Original ${p.mission.mission_index}, difference ${esc(fmt(p.delta))} ${esc(metric.unit)}"><title>Original ${p.mission.mission_index}: Δ ${esc(fmt(p.delta))} ${esc(metric.unit)}; A ${esc(fmt(p.a.values[state.metric]))}, B ${esc(fmt(p.b.values[state.metric]))}</title></circle>`;}).join('');
    const effect=s.effects.find(e=>e.cohort===state.cohort&&e.contrast===state.contrast&&e.metric===state.metric);
    const effectHtml=effect?`<p class="study-effect">Retained full-cohort mean Δ ${fmt(effect.estimate)} ${esc(metric.unit)} · 95% interval [${fmt(effect.low)}, ${fmt(effect.high)}] · ${effect.pairs} pairs. ${esc(effect.method)}. No subset interval is recomputed.</p>`:'';
    document.getElementById('study-differences').innerHTML=all.pairs.length?`<svg viewBox="0 0 500 225" role="img" aria-label="Paired original metric differences; negative is lower, positive is higher"><path d="M45 195H455M250 25V195" fill="none" stroke="#b6b0a3" stroke-dasharray="3 3"/><text x="45" y="214">${esc(fmt(-bound))}</text><text x="244" y="214">0</text><text x="440" y="214">${esc(fmt(bound))}</text>${points}</svg><p class="study-mini">Each point is one eligible original. Choose it for paired details. ${all.pairs.filter(p=>p.delta===0).length} exact ties; values retain precision before classification. ${subset.pairs.length}/${all.pairs.length} eligible pairs selected.</p>${effectHtml}`:`<p>No eligible metric pairs in this cohort. Both declared variants and their required recorded measurements must be available. Missing data are not zero.</p>`;
    const axes=s.condition_axes,rows=new Set(),cols=new Set(),cells=new Map();
    for(const m of r.data.missions){const p=PdStudyReviewModel.pair(m,state,s);if(!p.inCohort)continue;const o=p.a||p.b,a=o.attributes[axes[0]]||'Unavailable',b=o.attributes[axes[1]]||'All',key=a+'|'+b;rows.add(a);cols.add(b);if(!cells.has(key))cells.set(key,[]);cells.get(key).push({mission:m,...p});}
    const rs=[...rows].sort(),cs=[...cols].sort();
    document.getElementById('study-conditions').innerHTML=`<p class="study-mini">${esc(axes.join(' × ').replaceAll('_',' '))} · counts and descriptive eligible-pair means. No per-cell qualification claim.</p><div class="study-conditions"><table><thead><tr><th>Condition</th>${cs.map(c=>`<th>${esc(c)}</th>`).join('')}</tr></thead><tbody>${rs.map(a=>`<tr><th>${esc(a.replaceAll('_',' '))}</th>${cs.map(b=>{const key=a+'|'+b,ps=cells.get(key)||[],eligible=ps.filter(p=>p.eligible),mean=eligible.length?eligible.reduce((n,p)=>n+p.delta,0)/eligible.length:null,loss=ps.filter(p=>p.category==='lost').length;return `<td><button type="button" data-study-cell="${esc(key)}" aria-pressed="${state.cell===key}" style="background:${loss?'#ffe8df':mean===null?'#f0ede7':(metric.lower_is_better!==null&&(metric.lower_is_better?mean<0:mean>0))?'#e6f3ed':'#f3f0e9'}"><strong>${ps.length} originals</strong><br>Δ ${mean===null?'unavailable':esc(fmt(mean))+' '+esc(metric.unit)}<br>${eligible.length} pairs · ${loss} lost</button></td>`;}).join('')}</tr>`).join('')}</tbody></table></div>`;
    const perf=s.performance.filter(p=>p.variant===contrast.baseline||p.variant===contrast.candidate);
    if(!perf.length)document.getElementById('study-performance').innerHTML='<p>No isolated planning-latency measurements are declared for this study. Whole-episode timings are not substituted.</p>';
    else {
      const max=Math.max(...perf.flatMap(p=>p.values))*1.08,px=v=>150+300*v/max;
      document.getElementById('study-performance').innerHTML=`<svg viewBox="0 0 500 ${perf.length*85+30}" role="img" aria-label="Separate measured selection latency distributions">${perf.map((p,i)=>{const y=45+i*85,color=p.variant===contrast.candidate?'#087f73':'#356a96';return `<text x="5" y="${y-12}">${esc(p.label)}</text><line x1="${px(quantile(p.values,.05))}" x2="${px(quantile(p.values,.95))}" y1="${y}" y2="${y}" stroke="${color}" stroke-width="5"/>${p.values.map((v,j)=>`<circle cx="${px(v)}" cy="${y+(j*7%19)-9}" r="2" fill="${color}" opacity=".4"><title>${fmt(v)} ${esc(p.unit)}</title></circle>`).join('')}<line x1="${px(quantile(p.values,.5))}" x2="${px(quantile(p.values,.5))}" y1="${y-13}" y2="${y+13}" stroke="#20211e" stroke-width="2"/><text x="150" y="${y+30}">Median ${fmt(quantile(p.values,.5))} ${esc(p.unit)} · P95 ${fmt(quantile(p.values,.95))}</text>`;}).join('')}</svg><p class="study-mini">${perf.map(p=>`${esc(p.label)}: ${p.originals} originals × ${p.measured_repeats} measured repeats = ${p.values.length} timings`).join('; ')}. Whisker P05–P95, line median; repeats are not independent originals. ${esc(perf[0].scope)} These measurements do not change with mission selections.</p>`;
    }
    renderExplanation(r,s,state);
  };
  const renderExplanation=async(r,s,state)=>{
    const panel=document.getElementById('study-explanation'),token=++request;
    if(!s.timing_explanation||!state.mission){panel.hidden=true;return;}
    const mission=r.data.missions.find(m=>m.id===state.mission),pair=PdStudyReviewModel.pair(mission,state,s),record=pair.b||pair.a;
    panel.hidden=false;panel.innerHTML='<h3>Why this timing?</h3><p>Loading the selected original’s recorded native forecasts…</p>';
    try {
      if(!evidenceCache.has(record.evidence_href)){
        const url=new URL(record.evidence_href,location.href);if(url.origin!==location.origin)throw Error('Evidence origin differs');
        const response=await fetch(url,{redirect:'error'});if(!response.ok)throw Error('Evidence unavailable');
        const text=await response.text();if(text.length>2*1024*1024)throw Error('Evidence display limit');evidenceCache.set(record.evidence_href,JSON.parse(text));
      }
      if(token!==request)return;
      const e=evidenceCache.get(record.evidence_href),search=e.policy_decisions?.landing_search,forecast=search?.candidates||[];
      if(!forecast.length){panel.innerHTML='<h3>Why this timing?</h3><p>No candidate-forecast record is available for this selected execution.</p>';return;}
      const coasts=[...new Set(forecast.map(c=>c.coast_ticks/120))].sort((a,b)=>a-b),durations=[...new Set(forecast.map(c=>c.duration_ticks/120))].sort((a,b)=>a-b),passing=forecast.filter(c=>c.result==='predicted_target_landing'),fuel=passing.map(c=>c.remaining_fuel_kg),min=Math.min(...fuel),max=Math.max(...fuel);
      const cells=forecast.map(c=>{const x=70+coasts.indexOf(c.coast_ticks/120)*44,y=35+durations.indexOf(c.duration_ticks/120)*25,ok=c.result==='predicted_target_landing',level=ok?(c.remaining_fuel_kg-min)/Math.max(1e-9,max-min):0,fill=ok?`hsl(165 45% ${88-level*45}%)`:'#e8e4dc',chosen=c.coast_ticks===search.chosen_coast_ticks&&c.duration_ticks===search.chosen_duration_ticks;return `<rect x="${x}" y="${y}" width="39" height="21" fill="${fill}" stroke="${chosen?'#b45309':'#fffdf8'}" stroke-width="${chosen?3:1}"><title>Coast ${c.coast_ticks/120}s; burn ${c.duration_ticks/120}s; ${esc(c.result)}${ok?'; forecast remaining fuel '+fmt(c.remaining_fuel_kg)+' kg':''}${chosen?'; chosen actual execution':''}</title></rect>`;}).join('');
      panel.innerHTML=`<h3>Original ${mission.mission_index}: why this timing?</h3><p>Selected coast/preparation ${search.chosen_coast_ticks/120} s; Poly duration ${search.chosen_duration_ticks/120} s. These are recorded native forecasts; the selected captured flight carries its own proof.</p><div class="study-explanation-grid"><svg viewBox="0 0 ${Math.max(600,coasts.length*44+90)} ${durations.length*25+70}" role="img" aria-label="Native forecast grid, coast seconds horizontally and burn duration seconds vertically; selected timing outlined">${coasts.map((v,i)=>`<text x="${76+i*44}" y="22">${v}s</text>`).join('')}${durations.map((v,i)=>`<text x="10" y="${50+i*25}">${v}s</text>`).join('')}${cells}<text x="70" y="${durations.length*25+56}">Green: forecast target contact, darker: more fuel remaining. Grey: rejected forecast. Amber: selected.</text></svg></div><p class="study-mini">Full Cartesian candidate bank; no new query or simulation is executed. Coast and duration change jointly, so this plot does not isolate either factor’s causal contribution.</p>`;
    } catch(error){if(token===request)panel.innerHTML=`<h3>Why this timing?</h3><p>Recorded explanation unavailable: ${esc(error.message)}. The original flight and proof remain accessible from the mission tree.</p>`;}
  };
  window.addEventListener('pd-review-updated',render);
})();
