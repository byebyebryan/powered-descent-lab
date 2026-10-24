/* Shared paired/single direction histories, independent of the chart CDN. */
(() => {
  const ns='http://www.w3.org/2000/svg',node=(tag,attrs={})=>{const n=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attrs))n.setAttribute(k,v);return n;};
  const text=(parent,x,y,value)=>{const n=node('text',{x,y});n.textContent=value;parent.append(n);return n;};
  const style=()=>{if(document.getElementById('flight-inspection-style'))return;const s=document.createElement('style');s.id='flight-inspection-style';s.textContent='.flight-inspection{padding:16px;border:1px solid #d4d9ce;border-radius:12px;background:#fffdf8;margin-top:16px;min-width:0}.flight-inspection svg{display:block;width:100%;height:auto;touch-action:pan-y}.flight-inspection svg text{font:12px system-ui;fill:#51483d}.flight-inspection h3{margin:4px 0 12px}.flight-inspection input{width:100%;margin:12px 0}.flight-inspection output{display:block;font:13px/1.6 system-ui;overflow-wrap:anywhere}.flight-inspection .history-legend{font:12px/1.5 system-ui;color:#51483d}.flight-pair-panes{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}.flight-pair-panes>section{min-width:0}.flight-pair-panes h3{font-size:17px}@media(max-width:760px){.flight-pair-panes{grid-template-columns:1fr}}';document.head.append(s);};
  function mountHistory(host,records,{onTime=()=>{},slider=true}={}){
    style();host.classList.add('flight-inspection','flight-histories');host.style.maxWidth='1000px';host.style.marginInline='auto';
    const samples=records.flatMap(r=>r.data.samples),start=Math.min(...records.map(r=>r.data.samples[0]?.simTimeS??0)),end=Math.max(start+1e-6,...records.map(r=>r.data.samples.at(-1)?.simTimeS??0));
    const prepared=records.map((r,i)=>({...r,color:r.color||['#356a96','#087f73'][i%2],angles:PdFlightModel.angles(r.data.samples)}));
    const heading=document.createElement('h3');heading.textContent='Direction and command through time';host.append(heading);
    const legend=document.createElement('p');legend.className='history-legend';for(const r of prepared){const span=document.createElement('span');span.style.color=r.color;span.style.marginRight='14px';span.textContent='● '+r.label;legend.append(span);}const explanation=document.createElement('span');explanation.textContent=' · actual attitude solid, target dashed · degrees from upright, positive tilts right · angle unwrapped · commanded throttle 0–1';legend.append(explanation);host.append(legend);
    const cursors=[];
    for(const kind of ['direction','command']){
      const svg=node('svg',{viewBox:'0 0 640 170',role:'img','aria-label':kind==='direction'?'Actual and commanded attitude through mission time':'Commanded throttle through mission time'});
      const values=kind==='direction'?prepared.flatMap(r=>r.angles.flatMap(a=>[a.actual,a.target])):[0,1],min=Math.min(0,...values),max=Math.max(min+1,...values),px=t=>48+570*(t-start)/(end-start),py=v=>130-95*(v-min)/(max-min);
      svg.append(node('path',{d:'M48 28V130H618',fill:'none',stroke:'#cbbda9'}));text(svg,5,18,kind==='direction'?'Attitude · deg':'Throttle command · 0–1');text(svg,5,40,max.toFixed(1));text(svg,5,132,min.toFixed(1));text(svg,48,155,start.toFixed(2)+' s');text(svg,550,155,end.toFixed(2)+' s');
      for(const r of prepared){for(const mode of kind==='direction'?['actual','target']:['command']){
        const points=kind==='direction'?r.angles.map(a=>[a.time,a[mode]]):r.data.samples.map(s=>[s.simTimeS,s.throttleFrac]);
        let d='';points.forEach(([t,v],i)=>{d+=i?(kind==='command'||mode==='target'?`H${px(t)}V${py(v)}`:`L${px(t)},${py(v)}`):`M${px(t)},${py(v)}`;});
        svg.append(node('path',{d,fill:'none',stroke:r.color,'stroke-width':2,'stroke-dasharray':mode==='target'?'5 3':'none'}));
      }}
      for(const r of prepared){const marks=r.data.samples.filter((s,i)=>i&&s.phase!==r.data.samples[i-1].phase).map(s=>({time:s.simTimeS,label:'Phase '+s.phase}));for(const event of r.data.events||[])marks.push({time:event.simTimeS,label:event.label});for(const mark of marks){const line=node('line',{x1:px(mark.time),x2:px(mark.time),y1:28,y2:130,stroke:r.color,'stroke-dasharray':'2 4',opacity:.25});const title=node('title');title.textContent=r.label+' · '+mark.label+' at '+mark.time+' s';line.append(title);svg.append(line);}}
      const line=node('line',{x1:48,x2:48,y1:28,y2:130,stroke:'#b45309','stroke-width':1.5});svg.append(line);cursors.push(line);host.append(svg);
      svg.addEventListener('pointermove',e=>{const rect=svg.getBoundingClientRect(),x=(e.clientX-rect.left)/rect.width*640;seek(start+Math.max(0,Math.min(1,(x-48)/570))*(end-start));});
    }
    const control=document.createElement('input');control.type='range';control.min=start;control.max=end;control.step='any';control.setAttribute('aria-label','Inspect elapsed mission time');if(slider)host.append(control);
    const output=document.createElement('output');host.append(output);
    let current=start;
    const setTime=time=>{current=Math.max(start,Math.min(end,Number(time)));control.value=current;const x=48+570*(current-start)/(end-start);for(const line of cursors){line.setAttribute('x1',x);line.setAttribute('x2',x);}
      const snapshots=prepared.map(r=>({label:r.label,...PdFlightModel.at(r.data.samples,current)}));
      output.textContent=`Cursor ${current.toFixed(3)} s · `+snapshots.map(r=>{const s=r.sample;if(!s)return r.label+': no recorded samples';return `${r.label}: ${r.ended?'ended at '+s.simTimeS.toFixed(3)+' s':r.before?'not started':'sample '+s.simTimeS.toFixed(3)+' s'}; position (${s.xM.toFixed(2)}, ${s.yM.toFixed(2)}) m; attitude ${(s.attitudeRad*180/Math.PI).toFixed(2)}°; command ${(s.throttleFrac*100).toFixed(2)}%`;}).join(' | ');
      output.dataset.snapshots=JSON.stringify(snapshots.map(r=>({label:r.label,index:r.index,ended:r.ended,before:r.before,time:r.sample?.simTimeS,command:r.ended||r.before?null:r.sample?.throttleFrac})));return snapshots;};
    const seek=time=>{const result=setTime(time);onTime(current,result);};control.addEventListener('input',()=>seek(Number(control.value)));setTime(start);
    return {setTime,seek,control,output};
  }
  function mountPair(host,records){
    style();const b=PdFlightModel.bounds(records),scale=Math.min(510/(b.xmax-b.xmin),300/(b.ymax-b.ymin)),px=x=>300+(x-(b.xmin+b.xmax)/2)*scale,py=y=>185-(y-(b.ymin+b.ymax)/2)*scale;
    const panes=document.createElement('div');panes.className='flight-pair-panes';host.append(panes);const cursors=[];
    for(const [i,r] of records.entries()){
      const pane=document.createElement('section');pane.className='flight-inspection';const title=document.createElement('h3');title.textContent=r.label;title.style.color=r.color||['#356a96','#087f73'][i];pane.append(title);const svg=node('svg',{viewBox:'0 0 600 370',role:'img','aria-label':r.label+' recorded trajectory, shared metre scale'});
      const pad=r.data.missionDetails.targetPad;if(pad){svg.append(node('line',{x1:25,x2:575,y1:py(pad.surfaceYM),y2:py(pad.surfaceYM),stroke:'#8a7963'}));svg.append(node('line',{x1:px(pad.centerXM-pad.widthM/2),x2:px(pad.centerXM+pad.widthM/2),y1:py(pad.surfaceYM),y2:py(pad.surfaceYM),stroke:'#2f9e44','stroke-width':4}));}
      for(const goal of r.goals||[]){const x=px(goal.x),y=py(goal.y);svg.append(node('path',{d:`M${x},${y-5}L${x+5},${y}L${x},${y+5}L${x-5},${y}Z`,fill:'#b45309'}));}
      svg.append(node('polyline',{points:r.data.samples.map(s=>`${px(s.xM)},${py(s.yM)}`).join(' '),fill:'none',stroke:r.color||['#356a96','#087f73'][i],'stroke-width':2}));
      const origin=node('circle',{r:4,fill:'#b45309'}),shaft=node('line',{stroke:'#b45309','stroke-width':3}),tip=node('path',{fill:'#b45309'});svg.append(shaft,tip,origin);text(svg,25,350,`Shared scale · ${(1/scale).toFixed(3)} m / viewBox px`);pane.append(svg);panes.append(pane);cursors.push({origin,shaft,tip});
    }
    const history=document.createElement('section');host.append(history);
    const inspector=mountHistory(history,records,{onTime:(_,snapshots)=>snapshots.forEach((r,i)=>{
      const c=cursors[i],s=r.sample;for(const el of Object.values(c))el.setAttribute('opacity',r.ended||r.before?'.25':'1');if(!s)return;
      const x=px(s.xM),y=py(s.yM),u=r.ended||r.before?0:s.throttleFrac,dx=Math.sin(s.attitudeRad)*24*u,dy=-Math.cos(s.attitudeRad)*24*u,len=Math.hypot(dx,dy),nx=len?dx/len:0,ny=len?dy/len:0;
      c.origin.setAttribute('cx',x);c.origin.setAttribute('cy',y);for(const [k,v] of Object.entries({x1:x,y1:y,x2:x+dx,y2:y+dy}))c.shaft.setAttribute(k,v);
      c.tip.setAttribute('d',len?`M${x+dx},${y+dy}L${x+dx-5*nx-3*ny},${y+dy-5*ny+3*nx}L${x+dx-5*nx+3*ny},${y+dy-5*ny-3*nx}Z`:'');
    })});inspector.seek(records[0]?.data.samples[0]?.simTimeS||0);return inspector;
  }
  globalThis.PdFlightInspection={mountHistory,mountPair};
})();
