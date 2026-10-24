/* Read-only access to the exact native rich samples. */
(() => {
  const at=(samples,time)=>{
    if(!samples.length)return {sample:null,index:-1,ended:false};
    let lo=0,hi=samples.length;
    while(lo<hi){const mid=(lo+hi)>>1;if(samples[mid].simTimeS<=time)lo=mid+1;else hi=mid;}
    const index=Math.max(0,lo-1);
    return {sample:samples[index],index,ended:time>samples.at(-1).simTimeS,before:time<samples[0].simTimeS};
  };
  const angles=samples=>{
    let previous=null,total=0,previousTarget=null,totalTarget=0;
    return samples.map(s=>{const actual=s.attitudeRad;
      if(previous===null)total=actual;else total+=Math.atan2(Math.sin(actual-previous),Math.cos(actual-previous));previous=actual;
      if(previousTarget===null)totalTarget=total+Math.atan2(Math.sin(s.targetAttitudeRad-actual),Math.cos(s.targetAttitudeRad-actual));
      else totalTarget+=Math.atan2(Math.sin(s.targetAttitudeRad-previousTarget),Math.cos(s.targetAttitudeRad-previousTarget));previousTarget=s.targetAttitudeRad;
      return {time:s.simTimeS,actual:total*180/Math.PI,target:totalTarget*180/Math.PI};});
  };
  const extract=text=>{
    const marker='const reportData'+' = ',match=new RegExp('(?:^|\\n)[ \\t]*'+marker).exec(text);
    if(!match)throw Error('Saved rich payload missing');
    const start=match.index+match[0].length;
    let depth=0,quoted=false,escaped=false;
    for(let i=start;i<text.length;i++){
      const c=text[i];if(quoted){if(escaped)escaped=false;else if(c==='\\')escaped=true;else if(c==='"')quoted=false;continue;}
      if(c==='"')quoted=true;else if(c==='{'||c==='[')depth++;else if(c==='}'||c===']'){if(--depth===0)return JSON.parse(text.slice(start,i+1));}
    }
    throw Error('Saved rich payload incomplete');
  };
  const bounds=records=>{
    let xmin=Infinity,xmax=-Infinity,ymin=Infinity,ymax=-Infinity;
    for(const r of records){const points=r.data.samples.map(s=>[s.xM,s.yM]),pad=r.data.missionDetails.targetPad;
      if(pad)points.push([pad.centerXM-pad.widthM/2,pad.surfaceYM],[pad.centerXM+pad.widthM/2,pad.surfaceYM]);
      for(const goal of r.goals||[])points.push([goal.x,goal.y]);
      for(const [x,y] of points)if(Number.isFinite(x)&&Number.isFinite(y)){xmin=Math.min(xmin,x);xmax=Math.max(xmax,x);ymin=Math.min(ymin,y);ymax=Math.max(ymax,y);}}
    if(!Number.isFinite(xmin))return {xmin:0,xmax:1,ymin:0,ymax:1};
    const mx=Math.max(10,(xmax-xmin)*.08),my=Math.max(10,(ymax-ymin)*.08);
    return {xmin:xmin-mx,xmax:xmax+mx,ymin:ymin-my,ymax:ymax+my};
  };
  globalThis.PdFlightModel={at,angles,extract,bounds};
})();
