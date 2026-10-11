#!/usr/bin/env node
// Optional real-browser acceptance for a supplied create-only report edition.
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseArgs} from 'node:util';
import {cdpClient,waitForBrowserPage,checkFlightInteractions} from './planner-v2-browser-helpers.mjs';
import {validateBrowserEndpoints,resolveReportPath} from './check-planner-v2-browser.mjs';

export async function checkPresentation({rootUrl,cdpUrl,editionPath,outputDir}) {
  const {root,cdp}=validateBrowserEndpoints(rootUrl,cdpUrl);
  assert(editionPath.endsWith('/'),'edition path must end with a slash');
  const base=resolveReportPath(root,editionPath);
  const getJson=async name=>{const r=await fetch(new URL(name,base),{redirect:'error',signal:AbortSignal.timeout(20000)});assert(r.ok);return r.json();};
  const receipt=await getJson('render.json');const projection=await getJson('comparison-input.json');
  assert.equal(receipt.new_flights,0);assert.equal(receipt.new_replays,0);assert.equal(receipt.saved_rich_payloads_preserved,true);
  assert.equal(receipt.scheduled_attempts,receipt.native_details+receipt.status_details);
  const expected=projection.approaches.map(a=>{const good=a.campaigns.every(c=>c.page.integrity_passed);const rows=a.campaigns.flatMap(c=>c.page.attempts);return {label:a.label,total:rows.length,landed:rows.filter(r=>good&&r.verified_target_landing&&r.execution_valid&&r.integrity_passed&&r.physical_replay_verified&&r.complete_replay_verified&&r.stop==='mission_terminal'&&r.physical_outcome==='landed_on_target'&&r.mission_outcome==='success').length};});
  const response=await fetch(new URL('/json/new?about:blank',cdp),{method:'PUT'});assert(response.ok);const tab=await response.json();
  const ws=new WebSocket(tab.webSocketDebuggerUrl);await new Promise((ok,bad)=>{ws.addEventListener('open',ok,{once:true});ws.addEventListener('error',bad,{once:true});});
  const client=cdpClient(ws);const checks=[];const screenshots=[];const timings=[];
  const navigate=async url=>{await client.send('Page.navigate',{url});await waitForBrowserPage(client,url);};
  const wait=async expression=>{for(let i=0;i<300;i++){if(await client.evaluate(expression))return;await new Promise(r=>setTimeout(r,50));}throw Error(`Browser readiness: ${expression}`);};
  const screenshot=async name=>{if(!outputDir)return;const {data}=await client.send('Page.captureScreenshot',{format:'png'});writeFileSync(resolve(outputDir,name),Buffer.from(data,'base64'),{flag:'wx'});screenshots.push(name);};
  const noOverflow=async label=>{const width=await client.evaluate('({client:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth})');assert(width.scroll<=width.client+1,`${label}: document overflow ${JSON.stringify(width)}`);};
  try {
    for(const domain of ['Page','Runtime','Log','Network'])await client.send(`${domain}.enable`);
    for(const size of [{name:'desktop',width:1440,height:900,mobile:false},{name:'mobile',width:390,height:844,mobile:true}]) {
      await client.send('Emulation.setDeviceMetricsOverride',{...size,deviceScaleFactor:1});
      const report=new URL('report.html',base).href;
      await navigate(report);await wait('!!window.PdPairedReview');await noOverflow(size.name);
      const initial=await client.evaluate('({missions:PdPairedReview.data.missions.length,groups:PdPairedReview.data.groups.length,rows:document.querySelectorAll("#review-tree-root tbody tr").length,images:document.querySelectorAll("#review-tree-root img[src]").length,cards:[...document.querySelectorAll(".outcome-card")].map(c=>c.innerText),order:[...document.querySelectorAll("#report-page > section,#report-page > details")].map(e=>e.className)})');
      assert.equal(initial.missions,receipt.scheduled_missions);assert.equal(initial.rows,initial.groups);assert.equal(initial.images,0);
      if(!size.mobile) assert(await client.evaluate('[...document.querySelectorAll(".outcome-card")].every(c=>c.getBoundingClientRect().bottom<=innerHeight)'), 'all approach outcomes should fit above the desktop fold');
      assert(initial.order.findIndex(c=>c.includes('review-tree'))<initial.order.findIndex(c=>c.includes('paired-coverage')));
      expected.forEach((e,i)=>assert(initial.cards[i].includes(`${e.landed}/${e.total}`),`accounting: ${e.label}`));
      await screenshot(`${size.name}-outcomes.png`);
      const focus=await client.evaluate('(()=>{const b=document.querySelector("[data-paired-focus]");b.closest("details").open=true;b.click();return {group:b.dataset.pairedFocus,approach:b.dataset.approach,expected:PdPairedReview.data.missions.filter(m=>m.group===b.dataset.pairedFocus).length}})()');
      const actual=await client.evaluate('({group:PdPairedReview.state.group,approach:PdPairedReview.state.approach,groups:document.querySelectorAll("[data-paired-group]").length,missions:document.querySelectorAll("[data-paired-mission]").length})');
      assert.equal(actual.group,focus.group);assert.equal(actual.approach,focus.approach);assert.equal(actual.groups,1);assert.equal(actual.missions,focus.expected);
      await client.evaluate('document.querySelector("[data-paired-mission]").click();document.querySelector(".paired-preview-row").scrollIntoView({block:"start"})');
      await wait('[...document.querySelectorAll(".pair-card img")].length===2&&[...document.querySelectorAll(".pair-card img")].every(i=>i.complete&&i.naturalWidth>0)');
      assert.equal(await client.evaluate('document.querySelectorAll("tr.paired-attempt-row").length'),expected.length);await noOverflow(`${size.name}/pair`);
      await screenshot(`${size.name}-paired-mission.png`);
      const before=await client.evaluate('({href:document.querySelector(".paired-detail").href,query:location.search,mission:PdPairedReview.state.mission})');
      await navigate(before.href);await wait('document.querySelector(".review-detail-nav")?.dataset.contextReady==="true"');
      await noOverflow(`${size.name}/detail`);
      const charts=await client.evaluate('({ready:typeof Plotly!=="undefined"&&!!document.getElementById("chart-spatial")?._fullLayout,handoffs:reportData.flightAnnotations?.corrections?.length||0,payload:!!reportData.samples.length,staticVisible:!document.getElementById("static-flight-fallback").hidden,back:document.querySelector("[data-back-review]").href,next:document.querySelector("[data-review-next]").hidden?null:document.querySelector("[data-review-next]").href})');
      assert(charts.payload);assert(charts.ready,'live rich charts unavailable in ordinary check');
      await checkFlightInteractions(client,{correction_count:charts.handoffs},`${size.name}/saved-detail`);
      await screenshot(`${size.name}-rich-detail.png`);
      if(charts.next){await navigate(charts.next);await wait('document.querySelector(".review-detail-nav")?.dataset.contextReady==="true"');const next=await client.evaluate('({mission:document.body.dataset.reviewMission,back:document.querySelector("[data-back-review]").href})');assert.notEqual(next.mission,before.mission);assert.equal(new URL(next.back).searchParams.get('group'),focus.group);}
      await navigate(charts.back);await wait('!!window.PdPairedReview');
      assert.equal(await client.evaluate('PdPairedReview.state.mission'),before.mission);assert.equal(await client.evaluate('location.search'),before.query);
      const duration=await client.evaluate('(async()=>{const paint=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const results=[];for(let i=0;i<5;i++){let t=performance.now();PdPairedReview.focus(PdPairedReview.data.groups[i%PdPairedReview.data.groups.length].id,"0");await paint();const expand=performance.now()-t;t=performance.now();PdPairedReview.state.outcome="timed-out";PdPairedReview.state.approach="0";PdPairedReview.render();await paint();results.push({expand_ms:expand,filter_ms:performance.now()-t});}return results})()');const median=key=>duration.map(t=>t[key]).sort((a,b)=>a-b)[2];assert(median('expand_ms')<250&&median('filter_ms')<250,'warm painted interactions exceed 250 ms target');timings.push({viewport:size.name,trials:duration,includes_two_animation_frames:true,median_expand_ms:median('expand_ms'),median_filter_ms:median('filter_ms')});
      await client.evaluate('document.querySelector("[data-paired-action=reset]").click();document.getElementById("review-search").value="0";document.getElementById("review-search").dispatchEvent(new Event("input"))');
      assert.equal(await client.evaluate('PdPairedReview.data.missions.filter(m=>PdPairedReview.matches(m,PdPairedReview.state)).every(m=>m.mission_index===0)'),true);
      const unsupported=await client.evaluate('(()=>{const m=PdPairedReview.data.missions.find(m=>m.attempts.some(a=>a.outcome==="unsupported"));if(!m)return null;return {id:m.id,href:new URL(m.attempts.find(a=>a.outcome==="unsupported").href,location.href).href}})()');
      if(unsupported){await navigate(unsupported.href);assert(await client.evaluate('!document.getElementById("chart-spatial")&&document.body.innerText.includes("No simulator trajectory")'));await noOverflow(`${size.name}/unsupported`);await screenshot(`${size.name}-unsupported.png`);}
      const campaignUrl=new URL(projection.approaches[0].campaigns[0].report_href,base);campaignUrl.searchParams.set('presentation-check',size.name);await navigate(campaignUrl.href);
      assert(await client.evaluate('!!document.querySelector("[data-batch-template=common-v1]")&&document.body.innerText.includes("Back to paired comparison")'));
      const projectionLink=await client.evaluate('[...document.querySelectorAll(".hero-actions a")].find(a=>a.textContent==="Report projection")?.href');assert(projectionLink&&projectionLink.includes('campaign-a0-c0-input.json'));
      assert.equal(await client.evaluate('document.querySelectorAll("img[src]").length'),0);
      await client.evaluate('document.querySelector("tr.summary-row").click();document.querySelector("tr.campaign-row:not([hidden])").scrollIntoView({block:"center"})');
      await wait('[...document.querySelectorAll("tr.campaign-row:not([hidden]) img")].some(i=>i.complete&&i.naturalWidth>0)');await noOverflow(`${size.name}/campaign`);await screenshot(`${size.name}-campaign.png`);
      const camp=await client.evaluate('({previewVisible:[...document.querySelectorAll("tr.campaign-row:not([hidden]) img")].some(i=>i.naturalWidth&&i.getBoundingClientRect().left>=0&&i.getBoundingClientRect().right<=document.documentElement.clientWidth),detail:document.querySelector("tr.campaign-row:not([hidden]) .mission-link").href})');assert(camp.previewVisible);assert(new URL(camp.detail).pathname.startsWith(base.pathname+'details/'));
      await client.evaluate('document.querySelector("[data-review-outcome]").click()');
      assert(await client.evaluate('(()=>{const selected=document.querySelector("[data-review-outcome]").dataset.reviewOutcome;return [...document.querySelectorAll("tr.campaign-row:not(.review-filter-hidden)")].every(r=>r.dataset.outcome===selected)})()'));
      checks.push({viewport:size.name,initial,coverage:actual,pair_previews_loaded:2,context_roundtrip:true,rich_modes_hover_and_handoffs:true,unsupported:!!unsupported,campaign_companion_preview_links_and_filter:true});
    }
    assert.deepEqual(client.errors,[],'ordinary browser errors');
    const homeNavigation=[];
    for(const path of ['/', '/reports/']) {
      await navigate(new URL(path,root).href);
      const link=await client.evaluate(`[...document.querySelectorAll('a')].find(a=>a.href===${JSON.stringify(new URL('report.html',base).href)})?.href`);
      assert(link,'completed edition missing from report home '+path);
      await navigate(link);await wait('!!window.PdPairedReview');homeNavigation.push(path);
    }
    const ordinaryErrors=client.errors.length;
    await client.send('Network.setBlockedURLs',{urls:['*cdn.plot.ly*']});
    await client.send('Network.setCacheDisabled',{cacheDisabled:true});
    await navigate(new URL(receipt.captures[0].mission?`details/${receipt.captures[0].mission}-a${receipt.captures[0].approach}.html`:'',base).href);
    const fallback=await client.evaluate('({plotly:typeof Plotly,visible:!document.getElementById("static-flight-fallback").hidden,svg:document.querySelectorAll("#static-flight-fallback svg,#static-metrics-fallback svg").length,text:document.querySelector(".detail-outcome")?.innerText,title:document.getElementById("scenario-title").textContent})');
    assert.equal(fallback.plotly,'undefined');assert(fallback.visible&&fallback.svg===3&&fallback.title&&fallback.text);await noOverflow('blocked-Plotly');await screenshot('mobile-static-fallback.png');
    const blockedErrors=client.errors.slice(ordinaryErrors);assert(blockedErrors.every(e=>e.kind!=='javascript-exception'),'fallback raised a JavaScript exception');
    return {schema_version:1,passed:true,edition_path:editionPath,scheduled_missions:receipt.scheduled_missions,scheduled_attempts:receipt.scheduled_attempts,expected_outcomes:expected,checks,timings,static_fallback:fallback,intentionally_blocked_chart_diagnostics:blockedErrors,screenshots,home_navigation:homeNavigation,browser_errors:client.errors.slice(0,ordinaryErrors)};
  } finally {ws.close();await fetch(new URL(`/json/close/${encodeURIComponent(tab.id)}`,cdp));}
}

async function main(){const {values}=parseArgs({options:{'root-url':{type:'string',default:'http://127.0.0.1:8000/'},'cdp-url':{type:'string'},'edition-path':{type:'string'},'output-dir':{type:'string'}}});assert(values['cdp-url']&&values['edition-path']);const outputDir=values['output-dir']&&resolve(values['output-dir']);if(outputDir)mkdirSync(outputDir);const result=await checkPresentation({rootUrl:values['root-url'],cdpUrl:values['cdp-url'],editionPath:values['edition-path'],outputDir});if(outputDir)writeFileSync(resolve(outputDir,'browser-check.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});process.stdout.write(JSON.stringify(result,null,2)+'\n');}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){try{await main();}catch(error){process.stderr.write(`${error.stack??error}\n`);process.exitCode=1;}}
