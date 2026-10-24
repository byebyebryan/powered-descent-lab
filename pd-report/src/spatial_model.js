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
    const powered=[];
    for(let i=0;i<samples.length;i++){
      const s=samples[i];
      if(![s.xM,s.yM,s.attitudeRad,s.throttleFrac].every(finite)||s.throttleFrac<=0.015)continue;
      const p=project(s.xM,s.yM);
      if(p&&p.every(finite))powered.push({i,p});
    }
    if(!powered.length)return [];
    const changes=powered.slice(1).map((v,j)=>({j:j+1,score:Math.abs(Math.atan2(Math.sin(samples[v.i].attitudeRad-samples[powered[j].i].attitudeRad),Math.cos(samples[v.i].attitudeRad-samples[powered[j].i].attitudeRad)))+Math.abs(samples[v.i].throttleFrac-samples[powered[j].i].throttleFrac)})).sort((a,b)=>b.score-a.score);
    const candidates=[0,powered.length-1,...changes.filter(v=>v.score>.01).slice(0,4).map(v=>v.j)];
    for(let j=1;j<maximum-1;j++)candidates.push(Math.round(j*(powered.length-1)/(maximum-1)));
    for(let j=1;j<maximum*4;j++)candidates.push(Math.round(j*(powered.length-1)/(maximum*4)));
    const chosen=[];
    for(const j of candidates){const v=powered[j];if(chosen.some(q=>Math.hypot(v.p[0]-q.p[0],v.p[1]-q.p[1])<spacing))continue;chosen.push(v);if(chosen.length>=maximum)break;}
    return chosen.map(v=>v.i).sort((a,b)=>a-b);
  };
  const vectorAnnotation=(sample,layout,{color='#356a96',width=1.7}={})=>{
    const x=layout.xaxis,y=layout.yaxis,u=sample.throttleFrac;
    if(![sample.xM,sample.yM,sample.attitudeRad,u].every(finite)||u<=.015)return null;
    const px=x.l2p(sample.xM),py=y.l2p(sample.yM),dx=Math.sin(sample.attitudeRad)*24*u,dy=-Math.cos(sample.attitudeRad)*24*u;
    return {x:x.p2l(px+dx),y:y.p2l(py+dy),ax:-dx,ay:-dy,xref:'x',yref:'y',axref:'pixel',ayref:'pixel',text:'',showarrow:true,arrowhead:3,arrowsize:.9,arrowwidth:width,arrowcolor:color};
  };
  globalThis.PdSpatialModel={extent,scalarSegments,vectorIndices,vectorAnnotation};
})();
