/* Display geometry only. Original observations and hover samples are untouched. */
(() => {
  const finite = v => typeof v === 'number' && Number.isFinite(v);
  const extent = values => {
    const valid = values.filter(finite);
    if (!valid.length) return {min:0,max:1};
    const min=Math.min(...valid),max=Math.max(...valid);
    return {min,max:max>min?max:min+1};
  };
  const scalarSegments = (xs,ys,values,domain,bins=32) => {
    const groups=Array.from({length:bins},(_,bin)=>({bin,x:[],y:[]}));
    for(let i=1;i<Math.min(xs.length,ys.length,values.length);i++){
      if(![xs[i-1],xs[i],ys[i-1],ys[i],values[i-1],values[i]].every(finite))continue;
      const t=Math.max(0,Math.min(1,((values[i-1]+values[i])/2-domain.min)/(domain.max-domain.min)));
      const g=groups[Math.min(bins-1,Math.floor(t*bins))];
      // Explicit separators retain every segment without joining unrelated runs.
      g.x.push(xs[i-1],xs[i],null);g.y.push(ys[i-1],ys[i],null);
    }
    return groups.filter(g=>g.x.length);
  };
  const vectorIndices = (samples,project,{spacing=42,maximum=18}={}) => {
    const chosen=[],points=[];
    for(let i=0;i<samples.length;i++){
      const s=samples[i];
      if(![s.xM,s.yM,s.attitudeRad,s.throttleFrac].every(finite)||s.throttleFrac<=0.015)continue;
      const p=project(s.xM,s.yM);
      if(!p||!p.every(finite)||points.some(q=>Math.hypot(p[0]-q[0],p[1]-q[1])<spacing))continue;
      chosen.push(i);points.push(p);if(chosen.length>=maximum)break;
    }
    return chosen;
  };
  globalThis.PdSpatialModel={extent,scalarSegments,vectorIndices};
})();
