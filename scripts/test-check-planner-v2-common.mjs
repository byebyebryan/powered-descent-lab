import test from 'node:test';
import assert from 'node:assert/strict';
import {assertSharedBatch,assertSharedSummary} from './planner-v2-report-checks.mjs';

const common='<div data-batch-template="common-v1"><h2>Overview</h2><h2>Coverage</h2><h2>Context</h2><h2>Review Tree</h2><table><tr>'+['Selector','Success / Outcome','Fuel Used','Flight Time','Landing Offset','Reference deviation','Preview'].map(h=>`<th>${h}</th>`).join('')+'</tr></table></div>';
test('common template contract requires established sections and columns',()=>{assertSharedBatch(common);for(const section of ['Overview','Coverage','Context','Review Tree'])assert.throws(()=>assertSharedBatch(common.replace(`<h2>${section}</h2>`,'')));assert.throws(()=>assertSharedBatch(common.replace('common-v1','custom-v2')));});
test('canonical report cannot rebase ordinary navigation into evidence',()=>{assert.throws(()=>assertSharedBatch('<base href="/eval/capture/">'+common));assert.throws(()=>assertSharedBatch(common+'Presentation preview'));});
test('template section ordering is part of acceptance, not just shared colors',()=>{assert.throws(()=>assertSharedBatch(common.replace('<h2>Overview</h2><h2>Coverage</h2>','<h2>Coverage</h2><h2>Overview</h2>')));});
test('common summary keeps core and diagnostics separate and exact',()=>{
  const summary={valid_landing_count:36,direct_landing_count:11,corrected_landing_count:25,non_landing_count:0,diagnostic_landing_count:2,diagnostic_non_landing_count:4,unsupported_count:2,integrity_passed_count:44,final_source_replay_passed_count:42};
  const html='36/36 target landings;11 direct · 25 corrected · 0 non-landings;2 landed · 4 supported non-landings · 2 unsupported;44/44 integrity passed;42/42 supported source replays passed';
  assertSharedSummary(html,summary);assert.throws(()=>assertSharedSummary(html.replace('36/36','38/36'),summary));
});
