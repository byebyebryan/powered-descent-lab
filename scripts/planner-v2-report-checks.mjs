// Stable read-only checks shared by the maintained Planner V2 workflow.
import assert from 'node:assert/strict';
import {isDeepStrictEqual} from 'node:util';

const TIMING_POINTERS = [
  '/timings/planning_s',
  '/timings/execution_s',
  '/timings/replay_s',
];
// These are the only historically observed output-location fields. They are
// ignored only at the root of flight.json and only when both values are paths.
const PATH_POINTERS = ['/scenario_path', '/output_dir'];

const isRecord = value => value !== null && typeof value === 'object' && !Array.isArray(value);

function fail(message) {
  throw new Error(message);
}

function check(condition, message) {
  if (!condition) fail(message);
}

function firstDifference(actual, expected, path = '$') {
  if (Object.is(actual, expected)) return null;
  if (Array.isArray(actual) || Array.isArray(expected)) {
    if (!Array.isArray(actual) || !Array.isArray(expected)) return `${path}: array/value type differs`;
    if (actual.length !== expected.length) return `${path}: array length ${actual.length} != ${expected.length}`;
    for (let index = 0; index < actual.length; index += 1) {
      const difference = firstDifference(actual[index], expected[index], `${path}[${index}]`);
      if (difference) return difference;
    }
    return null;
  }
  if (isRecord(actual) || isRecord(expected)) {
    if (!isRecord(actual) || !isRecord(expected)) return `${path}: object/value type differs`;
    const actualKeys = Object.keys(actual).sort();
    const expectedKeys = Object.keys(expected).sort();
    if (!isDeepStrictEqual(actualKeys, expectedKeys)) {
      const missing = expectedKeys.filter(key => !Object.hasOwn(actual, key));
      const extra = actualKeys.filter(key => !Object.hasOwn(expected, key));
      return `${path}: object keys differ (missing ${missing.join(', ') || 'none'}; extra ${extra.join(', ') || 'none'})`;
    }
    for (const key of actualKeys) {
      const difference = firstDifference(actual[key], expected[key], `${path}.${key}`);
      if (difference) return difference;
    }
    return null;
  }
  return `${path}: ${String(actual)} != ${String(expected)}`;
}

export function assertTypedEqual(actual, expected, label) {
  if (!isDeepStrictEqual(actual, expected)) {
    fail(`${label}: exact typed JSON differs at ${firstDifference(actual, expected)}`);
  }
}

function removePointer(object, pointer, label, removed) {
  const parts = pointer.split('/').slice(1);
  let parent = object;
  for (const part of parts.slice(0, -1)) {
    if (!isRecord(parent) || !Object.hasOwn(parent, part)) return;
    parent = parent[part];
  }
  const leaf = parts.at(-1);
  if (isRecord(parent) && Object.hasOwn(parent, leaf)) {
    removed.push({pointer, value: parent[leaf]});
    delete parent[leaf];
  }
}

export function comparableFlight(flight, counterpart, label) {
  check(isRecord(flight) && isRecord(counterpart), `${label}: both flight payloads must be objects`);
  const actual = structuredClone(flight);
  const expected = structuredClone(counterpart);
  const ignored = [];
  for (const pointer of TIMING_POINTERS) {
    const actualHas = pointerExists(actual, pointer);
    const expectedHas = pointerExists(expected, pointer);
    check(actualHas === expectedHas, `${label}: timing field presence differs at ${pointer}`);
    if (actualHas) {
      check(Number.isFinite(pointerValue(actual, pointer)) && Number.isFinite(pointerValue(expected, pointer)), `${label}: nonnumeric timing exclusion at ${pointer}`);
      removePointer(actual, pointer, label, ignored);
      removePointer(expected, pointer, label, []);
    }
  }
  for (const pointer of PATH_POINTERS) {
    const actualHas = pointerExists(actual, pointer);
    const expectedHas = pointerExists(expected, pointer);
    check(actualHas === expectedHas, `${label}: path field presence differs at ${pointer}`);
    if (actualHas) {
      const actualPath = pointerValue(actual, pointer);
      const expectedPath = pointerValue(expected, pointer);
      check(typeof actualPath === 'string' && typeof expectedPath === 'string', `${label}: observed path field is not a string at ${pointer}`);
      removePointer(actual, pointer, label, ignored);
      removePointer(expected, pointer, label, []);
    }
  }
  assertTypedEqual(actual, expected, label);
  return ignored.map(item => item.pointer);
}

function pointerExists(object, pointer) {
  let value = object;
  for (const part of pointer.split('/').slice(1)) {
    if (!isRecord(value) || !Object.hasOwn(value, part)) return false;
    value = value[part];
  }
  return true;
}

function pointerValue(object, pointer) {
  let value = object;
  for (const part of pointer.split('/').slice(1)) value = value[part];
  return value;
}

export function extractReportData(html, label = 'report') {
  const marker = 'const reportData = ';
  const start = html.indexOf(marker);
  check(start >= 0, `${label}: missing const reportData payload`);
  const payloadStart = start + marker.length;
  const payloadEnd = html.indexOf(';\n', payloadStart);
  check(payloadEnd > payloadStart, `${label}: unterminated reportData payload`);
  try {
    const value = JSON.parse(html.slice(payloadStart, payloadEnd));
    check(isRecord(value), `${label}: reportData must be a JSON object`);
    return value;
  } catch (error) {
    throw new Error(`${label}: invalid reportData JSON: ${error.message}`);
  }
}

function boundarySnapshot(state) {
  check(isRecord(state), 'missing local-correction segment boundary state');
  return {
    physicsStep: state.physics_step,
    simTimeS: state.sim_time_s,
    positionM: state.position_m,
    velocityMps: state.velocity_mps,
    attitudeRad: state.attitude_rad,
    fuelKg: state.fuel_kg,
  };
}

export function verifyCorrectionAnnotations(flight, reportData, label) {
  const annotations = reportData.flightAnnotations;
  check(isRecord(annotations) && Array.isArray(annotations.corrections), `${label}: annotated reportData.flightAnnotations is missing`);
  const corrections = annotations.corrections;
  const segments = flight.segments.filter(segment => segment.kind === 'local_correction');
  check(corrections.length === flight.correction_count, `${label}: annotation count differs from flight correction_count`);
  check(segments.length === flight.correction_count, `${label}: raw LocalCorrection segment count differs from correction_count`);
  for (let index = 0; index < corrections.length; index += 1) {
    const annotation = corrections[index];
    const segment = segments[index];
    check(annotation.number === index + 1, `${label}: handoff numbers are not consecutive`);
    assertTypedEqual(annotation.entry, boundarySnapshot(segment.entry_state), `${label} H${index + 1} exact entry snapshot`);
    assertTypedEqual(annotation.handoff, boundarySnapshot(segment.end_state), `${label} H${index + 1} exact handoff snapshot`);
    check(typeof annotation.reason === 'string' && annotation.reason.trim().length > 0, `${label} H${index + 1}: missing correction reason`);
    check(typeof annotation.afterHandoff === 'string' && annotation.afterHandoff.trim().length > 0, `${label} H${index + 1}: missing post-handoff result`);
  }
  return corrections.length;
}

export function verifyOrdinaryExecution(flight, id) {
  const ordinary = flight.ordinary_flight;
  check(Array.isArray(ordinary?.actions), `${id}: simulated result has no raw action array`);
  check(Array.isArray(ordinary?.samples) && ordinary.samples.length > 0, `${id}: simulated result has no raw samples`);
  if (ordinary.actions.length === 0) {
    check(flight.planning_stop === 'no_clearing' && flight.physical_outcome === 'flying'
      && flight.mission_outcome === 'in_progress', `${id}: empty commands are not a landing`);
    check(flight.manifest.physics_steps === 0 && flight.manifest.controller_updates === 0
      && flight.manifest.sim_time_s === 0, `${id}: empty commands have nonzero execution coverage`);
    check(ordinary.samples.length === 1 && ordinary.samples[0].physics_step === 0,
      `${id}: zero-command stop must retain only its initial sample`);
  }
}

function parseAttributes(source) {
  const attributes = {};
  const pattern = /([^\s=]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/g;
  let match;
  while ((match = pattern.exec(source))) attributes[match[1].toLowerCase()] = decodeHtml(match[2] ?? match[3] ?? match[4] ?? '');
  return attributes;
}

function decodeHtml(value) {
  return value.replaceAll('&amp;', '&').replaceAll('&quot;', '"').replaceAll('&#39;', "'")
    .replaceAll('&lt;', '<').replaceAll('&gt;', '>');
}

function visibleText(html) {
  return decodeHtml(html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, ' ')
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, ' ')
    .replace(/<[^>]+>/g, ' ')).replace(/\s+/g, ' ').trim();
}

export function parseAnchors(html, pageUrl) {
  const baseMatch = /<base\b([^>]*)>/i.exec(html);
  const baseHref = baseMatch ? parseAttributes(baseMatch[1]).href : null;
  const baseUrl = baseHref ? new URL(baseHref, pageUrl) : new URL(pageUrl);
  const anchors = [];
  const pattern = /<a\b([^>]*)>([\s\S]*?)<\/a\s*>/gi;
  let match;
  while ((match = pattern.exec(html))) {
    const attributes = parseAttributes(match[1]);
    if (!attributes.href) continue;
    let url;
    try { url = new URL(attributes.href, baseUrl); } catch { continue; }
    anchors.push({href: attributes.href, url: url.href, pathname: url.pathname, text: visibleText(match[2]), attributes});
  }
  return {baseHref, baseUrl: baseUrl.href, anchors};
}

export function verifyHttpContent(contentType, body, label, kind = 'html') {
  if (kind === 'json') {
    check(/application\/json/i.test(contentType), `${label}: expected JSON, got ${contentType || 'no content type'}`);
    JSON.parse(body);
  } else {
    check(/text\/html|application\/xhtml\+xml/i.test(contentType), `${label}: expected HTML, got ${contentType || 'no content type'}`);
  }
}

export function isEvidenceAnchor(anchor) {
  return new URL(anchor.url).pathname.endsWith('.json')
    || /source|original|scenario\.json|flight\.json|summary\.json|input/i.test(anchor.text);
}

export function assertSharedBatch(html) {
  assert(html.includes('data-batch-template="common-v1"'),'must render through the shared full batch template');
  const headings=[...html.matchAll(/<h2[^>]*>(.*?)<\/h2>/gs)].map(m=>m[1].replace(/<[^>]+>/g,'').trim());
  for(const name of ['Overview','Coverage','Context','Review Tree'])assert(headings.includes(name),`missing common section ${name}`);
  assert(headings.indexOf('Overview')<headings.indexOf('Coverage')&&headings.indexOf('Coverage')<headings.indexOf('Context')&&headings.indexOf('Context')<headings.indexOf('Review Tree'));
  assert(!html.includes('<base '),'canonical reader navigation must not be rebased into raw captures');
  assert(!html.includes('preview_final')&&!html.includes('Presentation preview'));
  for(const header of ['Selector','Success / Outcome','Fuel Used','Flight Time','Landing Offset','Reference deviation','Preview'])assert(html.includes(`<th>${header}</th>`),`missing established tree column ${header}`);
}
export function assertSharedSummary(html,summary){
  for(const text of [`${summary.valid_landing_count}/36 target landings`,`${summary.direct_landing_count} direct · ${summary.corrected_landing_count} corrected · ${summary.non_landing_count} non-landings`,`${summary.diagnostic_landing_count} landed · ${summary.diagnostic_non_landing_count} supported non-landings · ${summary.unsupported_count} unsupported`,`${summary.integrity_passed_count}/44 integrity passed`,`${summary.final_source_replay_passed_count}/42 supported source replays passed`])assert(html.includes(text),`incorrect common summary: ${text}`);
}
export function expectedProjection(record, flight, scenario) {
  const departed = Boolean((flight.manifest && flight.manifest.physics_steps > 0) || flight.ordinary_flight?.samples.some(s=>s.physics_step>0));
  const landed = flight.planning_stop === 'landed' && flight.physical_outcome === 'landed_on_target'
    && flight.mission_outcome === 'success' && flight.integrity_passed && flight.final_source_replay_passed;
  const unsupported = flight.planning_stop === 'unsupported';
  return {
    case_id:record.case_id, group:record.group, family:record.family,
    label:record.case_id.replace(/^(?:v2_|fresh_)/,'').replaceAll('_',' '),
    detail_href:record.annotated_report_path,
    display_outcome:unsupported ? 'Unsupported — not simulated' : !departed ? `Not departed — ${record.planning_stop ?? 'planning stop'}`
      : landed ? 'Landed on target' : `${record.physical_outcome ?? 'unverified'} — ${record.planning_stop ?? 'missing stop'}`,
    departed, landed:Boolean(landed), unsupported,
    integrity_passed:flight.integrity_passed, replay_passed:unsupported ? null : flight.final_source_replay_passed,
    correction_count:flight.correction_count,
    fuel_used_pct:departed && flight.manifest && scenario.vehicle.max_fuel_kg > 0 ? 100 * flight.manifest.summary.fuel_used_kg / scenario.vehicle.max_fuel_kg : null,
    flight_s:departed && flight.manifest ? flight.manifest.sim_time_s : null,
    landing_offset_m:departed && landed && flight.manifest?.summary.landing ? Math.abs(flight.manifest.summary.landing.touchdown_center_offset_m) : null,
    planning_s:unsupported ? null : record.planning_s,
    handoffs:flight.segments.filter(s=>s.kind==='local_correction').map(s=>s.end_state.position_m),
  };
}

export function extractProjection(html) {
  const match = html.match(/const batchTreeData = (.*?);<\/script>/s);
  assert(match, 'missing tree projection');
  return JSON.parse(match[1]);
}

export function assertSummaryText(html, summary) {
  for (const text of [
    `${summary.integrity_passed_count}/44 integrity passed`,
    `${summary.final_source_replay_passed_count}/42 supported source replays passed`,
    `<strong>${summary.valid_landing_count}/36</strong>`,
    `${summary.direct_landing_count} direct · ${summary.corrected_landing_count} corrected · ${summary.non_landing_count} non-landings`,
    `${summary.diagnostic_landing_count} landed · ${summary.diagnostic_non_landing_count} supported non-landings · ${summary.unsupported_count} unsupported`,
    `${summary.crash_count} crashes.`,
  ]) assert(html.includes(text), `missing or inaccurate page-level summary: ${text}`);
}
