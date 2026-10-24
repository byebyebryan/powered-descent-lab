// Browser acceptance through a caller-owned tab/client; no browser discovery,
// server lifecycle, flight execution or report mutation.
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {join} from 'node:path';

async function ready(client, href, rich = false) {
  for (let attempt = 0; attempt < 300; attempt++) {
    const condition = rich
      ? 'typeof Plotly!=="undefined"&&Boolean(document.getElementById("chart-spatial")?._fullLayout)&&Boolean(document.getElementById("chart-metrics")?._fullLayout)'
      : 'Boolean(window.PdPairedReview)';
    if (await client.evaluate(`location.pathname===${JSON.stringify(new URL(href).pathname)}&&document.readyState==='complete'&&${condition}`)) {
      await client.evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
      return;
    }
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error('Report readiness: ' + href);
}

async function screenshot(client, path) {
  const {data} = await client.send('Page.captureScreenshot', {format: 'png', captureBeyondViewport: false});
  writeFileSync(path, Buffer.from(data, 'base64'), {flag: 'wx'});
}

export async function checkTimingViewport(client, {url, output, name, width}) {
  await client.send('Page.navigate', {url});
  await ready(client, url);
  const counts = await client.evaluate('({missions:PdPairedReview.data.missions.length,groups:PdPairedReview.data.groups,options:document.getElementById("timing-state").options.length})');
  assert.equal(counts.missions, 366);
  assert.equal(counts.options, 60);
  assert.equal(await client.evaluate('document.querySelectorAll("#timing-results tr").length'), 6);
  await client.evaluate('document.getElementById("timing-state").value="composition-confirmation-027";document.getElementById("timing-state").dispatchEvent(new Event("change"))');
  assert.equal(await client.evaluate('document.querySelectorAll("#timing-results a").length'), 6);
  assert(await client.evaluate('document.body.textContent.includes("20.45%")&&document.body.textContent.includes("0.21%")&&document.body.textContent.includes("exposed")'));
  assert(await client.evaluate(`document.querySelector('img[src="timing-goal-comparison.svg"]').naturalWidth>0`));
  assert(await client.evaluate(`document.documentElement.scrollWidth<=${width + 2}`));
  await screenshot(client, join(output, name + '-overview.png'));
  await client.evaluate(`document.querySelector('[data-paired-depth="3"]').click()`);
  const seen = [];
  for (const group of counts.groups) {
    await client.evaluate(`document.querySelector('.mission-browser [data-review-stratum-select]').value=${JSON.stringify(group.id)};document.querySelector('.mission-browser [data-review-stratum-select]').dispatchEvent(new Event('change',{bubbles:true}))`);
    seen.push(...await client.evaluate('(()=>{const ids=[];for(;;){ids.push(...[...document.querySelectorAll("[data-paired-mission]")].map(e=>e.dataset.pairedMission));const next=document.querySelector(".mission-browser [data-review-page=next]");if(next.disabled)break;next.click()}return ids})()'));
  }
  assert.equal(seen.length, 366);
  assert.equal(new Set(seen).size, 366);
  const visible = await client.evaluate('document.querySelectorAll("[data-paired-mission] img").length');
  assert(visible > 0);
  for (let index = 0; index < visible; index++) {
    const loaded = await client.evaluate(`(async()=>{const img=document.querySelectorAll('[data-paired-mission] img')[${index}];img.scrollIntoView({block:'center'});for(let n=0;n<100;n++){if(img.naturalWidth>0&&!img.currentSrc.startsWith('data:')&&img.currentSrc.includes('flight-')){await img.decode();await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true}await new Promise(r=>setTimeout(r,50))}return false})()`);
    assert(loaded, 'Visible preview did not load');
  }
  await screenshot(client, join(output, name + '-previews.png'));
  const details = await client.evaluate('PdPairedReview.data.missions.flatMap(m=>m.attempts.map(a=>a.href)).filter(Boolean)');
  assert.equal(new Set(details).size, 366);
  for (const index of [0, 1, 3, 5, 91, 181, 365]) {
    const href = new URL(details[index], url).href;
    await client.send('Page.navigate', {url: href});
    await ready(client, href, true);
    // Browser connector results can cross JavaScript realms. Check the exact
    // toolbar values in the page instead of comparing transport prototypes.
    assert(await client.evaluate(`JSON.stringify([...document.querySelectorAll('[data-mode]')].map(b=>b.dataset.mode))===JSON.stringify(['mission','guidance','speed','throttle','vectors'])&&(reportData.flightAnnotations?.corrections||[]).length===0&&!document.getElementById('flight-handoffs-visible')`));
    assert(await client.evaluate('document.body.textContent.includes("Incoming native contact")&&document.body.textContent.includes("Arrival and landing execution")'));
    for (const mode of ['throttle', 'guidance', 'vectors', 'mission']) {
      await client.evaluate(`document.querySelector('[data-mode="${mode}"]').click()`);
    }
    assert(await client.evaluate(`document.documentElement.scrollWidth<=${width + 2}`));
    assert(await client.evaluate('Boolean(document.querySelector("[data-back-review]"))'));
    if (index === 1) await screenshot(client, join(output, name + '-fuel-ranked-flight.png'));
  }
  const first = new URL(details[0], url).href;
  await client.send('Page.navigate', {url: first});
  await ready(client, first, true);
  assert(await client.evaluate('!document.querySelector("[data-review-next]").hidden'));
  const next = await client.evaluate('document.querySelector("[data-review-next]").href');
  await client.evaluate('document.querySelector("[data-review-next]").click()');
  await ready(client, next, true);
  await client.evaluate('document.querySelector("[data-back-review]").click()');
  await ready(client, url);
  return {viewport: name, missions: 366, comparison_states: 60, compared_choices: 6,
    all_occurrences_browsed: true, visible_previews_loaded: visible, rich_details_exercised: 7,
    modes_and_next_back: true, overflow: false};
}
