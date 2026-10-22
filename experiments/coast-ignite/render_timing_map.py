"""Display-only timing maps inside the maintained rich mission report."""
from __future__ import annotations

import argparse
import html
import json
from pathlib import Path

from coast_ignite import NATIVE_ARTIFACT, encoded, native, sha, write_new
from render_stopping import scroll_summary_tables
from timing_analysis import read


def calibrate(root, captures):
    root = Path(root)
    output = root / "campaign/report-cost-pilot"
    native.render_report(encoded({"title": "Timing map report cost pilot",
        "summary": "Display-only pilot; these are captured controls, not independent missions.",
        "grouped": False, "focus_flat": True, "max_bytes": 2**30,
        "rows": [{"capture": r["capture"], "population": "Pilot controls", "label": "Captured control"} for r in captures]}), str(output))
    bundles = [sum((output / (p.stem + suffix)).stat().st_size for suffix in
                   [".html", ".svg", "-evidence.json", "-proof.json"]) for p in output.glob("flight-*.html")]
    result = {"display_only": True, "native_steps": 0, "pilot_rich_views": len(bundles),
              "maximum_rich_bundle_bytes": max(bundles), "total_rich_bytes": sum(bundles)}
    write_new(root / "report-cost-calibration.json", result)
    return result


def capture_groups(captures, roles):
    by_key = {Path(r["capture"]).name: r for r in captures}
    best = {r["id"]: r["capture_key"] for r in roles if r["role"] == "best"}
    used, groups = set(), []
    for cohort, folder, title in [("development", "report", "Development timing maps"),
                                  ("diagnostic", "report-diagnostics", "Known misses and fuel outliers")]:
        subset = []
        for row in captures:
            key = Path(row["capture"]).name
            if row["cohort"] == cohort and best.get(row["id"]) == key:
                subset.append(by_key[key]); used.add(key)
        groups.append((folder, title, subset, True))
    extra = [r for r in captures if Path(r["capture"]).name not in used]
    if extra:
        groups.append(("report-probes", "Additional captured policies and timing probes", extra, False))
    if sum(len(g[2]) for g in groups) != len(captures):
        raise ValueError("rich report capture assignment is not unique")
    return groups


def compact_rows(cases, summaries, rows, links):
    by_id = {s["id"]: s for s in summaries}
    result = []
    for case in cases:
        points = []
        for r in rows:
            if r["id"] != case["id"]:
                continue
            m = r["metrics"]
            admission = r["diagnostics"]["admission"]
            points.append({"slot": r["slot"], "success": r["native_success"],
                "fuel": m["fuel_used_kg"], "ignition_tick": m["ignition_tick"],
                "stop_s": m["stop_tick"] / 120, "contact_residual_s": m["contact_residual_s"],
                "admitted": m["admitted"], "contact": m["contact"],
                "admission": admission, "capture": links.get((r["id"], r["slot"]))})
        result.append({"id": case["id"], "cohort": case["cohort"], "arc": case["arc"],
                       "reference_end_s": read(case["task_path"])["reference_end"] / 120,
                       "summary": by_id[case["id"]], "points": points})
    return result


STYLE = """<style>
.timing-study{min-width:0}.timing-study select{max-width:100%;padding:.55rem;background:#142235;color:#edf2f7;border:1px solid #64748b;border-radius:6px}
.timing-study .timing-scroll{max-width:100%;min-width:0;overflow-x:auto}.timing-strip{display:flex;gap:2px;width:max-content;padding:12px 0}
.timing-slot{width:10px;height:30px;padding:0;border:1px solid transparent;border-radius:2px;cursor:pointer;background:#64748b}
.timing-slot.selected{outline:2px solid #fff;outline-offset:2px}.timing-slot[data-state=landed]{background:#22c55e}.timing-slot[data-state=rejected]{background:#f59e0b}.timing-slot[data-state=crashed]{background:#ef4444}
.timing-study svg{width:100%;max-width:900px;display:block;color:#cbd5e1}.timing-study .timing-metrics{line-height:1.7;padding:12px;border:1px solid #475569;border-radius:6px}
.timing-study .timing-caption{color:#cbd5e1;font-size:.92rem}.timing-study button{color:#edf2f7}.timing-study a{color:#7dd3fc}
</style>"""


SCRIPT = r"""<script>
(()=>{const data=TIMING_DATA, select=document.getElementById('timing-case'), axis=document.getElementById('timing-axis');
const summary=document.getElementById('timing-case-summary'), strip=document.getElementById('timing-strip'), plot=document.getElementById('timing-plot'), detail=document.getElementById('timing-point');
let selected=null; const esc=x=>String(x).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const num=(x,n=3)=>x===null||x===undefined?'—':Number(x).toFixed(n);
const label=p=>p.slot===null?'Never ignite':(p.slot/10).toFixed(1)+' s';
function showPoint(p){selected=p.slot;strip.querySelectorAll('button').forEach(b=>b.classList.toggle('selected',b.dataset.slot===String(p.slot)));
 const c=p.contact,a=p.admission,proof=p.capture?'<a data-timing-capture href="'+esc(p.capture)+'">Open replay-verified rich flight</a>':'Summary only: no full flight capture for this point.';
 detail.innerHTML='<strong>'+esc(label(p))+'</strong> · native Summary outcome: '+(p.success?'target success':'no target success')+'<br>Fuel used '+num(p.fuel)+' kg · actual stop '+num(p.stop_s)+' s · contact minus reference end '+num(p.contact_residual_s)+' s<br>'+(a?esc(a.kind+(a.reason?': '+a.reason:'')):'No ignition request before native stop')+(c?'<br>Normal speed '+num(c.normal_speed_mps)+' m/s · tangential '+num(c.tangential_speed_mps)+' m/s · attitude '+num(c.attitude_error_rad,4)+' rad<br>Foot clearances '+num(c.min_touchdown_clearance_m,4)+' to '+num(c.max_touchdown_clearance_m,4)+' m':'')+'<br>'+proof; }
function draw(){const d=data[Number(select.value)], s=d.summary;strip.replaceChildren();
 summary.textContent=d.id+' · reference end '+num(d.reference_end_s)+' s · '+s.successful_powered_schedules+'/81 successful admitted requests · '+s.successful_powered_components.length+' powered success components. Best tested fuel '+num(s.best_fuel_kg)+' kg. M '+(s.policies.M.native_success?'succeeds':'misses')+', G '+(s.policies.G.native_success?'succeeds':'misses')+'.';
 for(const p of d.points){const b=document.createElement('button');b.type='button';b.className='timing-slot';b.dataset.slot=String(p.slot);b.dataset.state=p.success&&p.admitted?'landed':p.admission&&p.admission.kind==='no_plan'?'rejected':p.admitted?'crashed':'passive';b.title=label(p)+' · '+(p.success?'native success':'no success')+' · '+num(p.fuel)+' kg';b.setAttribute('aria-label',b.title);b.addEventListener('click',()=>showPoint(p));strip.appendChild(b);}
 const pts=d.points.filter(p=>p.success&&p.slot!==null), flight=axis.value==='flight';
 const xs=pts.map(p=>flight?p.stop_s:p.slot/10), ys=pts.map(p=>p.fuel);let xmin=flight?Math.min(...xs):0,xmax=flight?Math.max(...xs):8,ymin=Math.min(...ys),ymax=Math.max(...ys);
 if(!pts.length){plot.innerHTML='<p>No native-successful requested timing on this map.</p>';showPoint(d.points[0]);return;}
 if(xmin===xmax){xmin-=.1;xmax+=.1}if(ymin===ymax){ymin-=.1;ymax+=.1}const pad=Math.max(.05,(ymax-ymin)*.08);ymin-=pad;ymax+=pad;
 const x=p=>60+640*((flight?p.stop_s:p.slot/10)-xmin)/(xmax-xmin),y=p=>190-160*(p.fuel-ymin)/(ymax-ymin);
 let svg='<svg viewBox="0 0 760 240" role="img" aria-label="Successful timing fuel costs"><path d="M60 20V190H710" stroke="#94a3b8" fill="none"/>';
 svg+='<text x="5" y="28" fill="currentColor">'+num(ymax,1)+'</text><text x="5" y="193" fill="currentColor">'+num(ymin,1)+'</text><text x="60" y="215" fill="currentColor">'+num(xmin,1)+' s</text><text x="665" y="215" fill="currentColor">'+num(xmax,1)+' s</text><text x="230" y="237" fill="currentColor">'+(flight?'Actual flight duration':'Requested ignition time')+' · fuel used in kg</text>';
 for(const p of pts){let color='#94a3b8',radius=3;if(p.slot===s.best_slot){color='#22c55e';radius=6}if(p.slot===s.policies.G.slot){color='#38bdf8';radius=6}if(p.slot===s.policies.M.slot){color='#fb923c';radius=5}svg+='<circle data-point="'+p.slot+'" cx="'+x(p)+'" cy="'+y(p)+'" r="'+radius+'" fill="'+color+'" tabindex="0" role="button" aria-label="'+esc(label(p))+'"><title>'+esc(label(p))+' · '+num(p.fuel)+' kg</title></circle>';}
 plot.innerHTML=svg+'</svg>';plot.querySelectorAll('[data-point]').forEach(el=>{const f=()=>showPoint(d.points.find(p=>p.slot===Number(el.dataset.point)));el.addEventListener('click',f);el.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();f()}})});
 showPoint(d.points.find(p=>p.slot===s.best_slot)||d.points[0]);}
select.addEventListener('change',draw);axis.addEventListener('change',draw);draw();})();
</script>"""


def map_widget(data):
    options = ''.join(f'<option value="{i}">{html.escape(d["id"])} · original arc {d["arc"]}</option>' for i, d in enumerate(data))
    return (STYLE + '<section class="timing-study" id="timing-study"><h2>Every requested handoff</h2>'
        '<p>All 81 ignition times plus never. Green: admitted native success; amber: rejected; red: admitted without success; grey: passive or censored. The strip contains Summary observations; links identify full proof captures.</p>'
        '<label>Mission <select id="timing-case">' + options + '</select></label><p id="timing-case-summary"></p>'
        '<div class="timing-scroll"><div class="timing-strip" id="timing-strip"></div></div>'
        '<label>Fuel plot <select id="timing-axis"><option value="ignition">By ignition time</option><option value="flight">By actual flight duration</option></select></label>'
        '<p class="timing-caption">Grey: successful tested points · green: best observed fuel · blue: G · orange: M. Overlapping markers share a point. Every dot is selectable.</p>'
        '<div id="timing-plot"></div><div id="timing-point" class="timing-metrics"></div></section>'
        + SCRIPT.replace('TIMING_DATA', encoded(data).replace('<', '\\u003c')))


def overview(result, cohort):
    stat = result["cohorts"][cohort]
    rows = ''.join(f'<tr><th>{r}</th><td>{stat[r]["native_successes"]}/{stat["arcs"]}</td><td>{stat[r]["avoidable_misses"]}</td><td>{stat[r]["fuel_regret_kg_on_success"].get("median",0):.3f} kg</td><td>{100*stat[r]["fuel_regret_fraction_on_success"].get("median",0):.3f}%</td><td>{stat[r]["at_least_one_percent_headroom"]}</td></tr>' for r in ["M", "G"])
    return ('<section><h2>Fixed-end timing and efficiency</h2><p>Unchanged Poly Landing, frozen models, zero training. '
        'These exposed inputs characterize timing; they are not fresh policy qualification. Best tested timing is a hindsight controller/grid comparator.</p>'
        f'<p>{stat["arcs"]} arcs · {stat["with_successful_grid_point"]} with a native-successful tested schedule · '
        f'{stat["with_disconnected_powered_windows"]} with disconnected powered success sets · '
        f'{stat["best_with_successful_neighbors"]} best points with successful adjacent 0.1 s requests.</p>'
        '<table><thead><tr><th>Frozen rule</th><th>Native successes</th><th>Misses with another successful point</th><th>Median fuel regret on success</th><th>Median relative regret</th><th>Successful arcs with ≥1% headroom</th></tr></thead><tbody>'
        + rows + '</tbody></table><p>Reference end is a construction instruction; native safe first contact can end a flight earlier. Fuel savings, actual duration and contact margins stay separate. Neighbor success is discrete timing sensitivity, not perturbation robustness.</p></section>')


def capture_context(row):
    folder = Path(row["capture"])
    proof = read(folder / "timing-proof.json")
    for name, key in [("evaluation.json", "capture_sha256"), ("decisions.json", "decisions_sha256"),
                      ("diagnostics.json", "diagnostics_sha256"), ("timing.json", "timing_sha256")]:
        if sha(folder / name) != proof[key]:
            raise ValueError("capture/sidecar proof mismatch")
    m = row["metrics"]
    label = "Never" if row["slot"] is None else f'{row["slot"]/10:.1f} s'
    return (f'<section><h2>Timing and cost</h2><p>Requested timing: {label}. Fuel used: {m["fuel_used_kg"]:.6f} kg. '
        f'Actual stop: {m["stop_tick"]/120:.6f} s. Contact-time residual: {m["contact_residual_s"]} s.</p>'
        f'<p>Reconstruction: {html.escape(proof["planner_kind"])}. Complete Summary equality, physical replay and controller reconstruction passed.</p>'
        '<p>Use the report timing map to compare every legal request; this rich view is one captured occurrence.</p></section>')


def render(root):
    root = Path(root).resolve(); campaign = root / "campaign"
    allocation = read(root / "execution-allocation.json")
    result = read(campaign / "result.json")
    maps = read(campaign / "map-summary.json")["maps"]
    rows = [json.loads(line) for line in (campaign / "results.jsonl").read_text().splitlines()]
    captures, roles = read(campaign / "report-captures.json"), read(campaign / "capture-roles.json")
    groups = capture_groups(captures, roles)
    links = {}
    for folder, _, subset, _ in groups:
        for index, r in enumerate(subset):
            links.setdefault((r["id"], r["slot"]), f'../{folder}/flight-{index}.html')
    data = compact_rows(allocation["cases"], maps, rows, links)
    common_links = '<p>Captured mission trees: ' + ' · '.join(f'<a href="../{folder}/report.html">{html.escape(title)}</a>' for folder, title, _, _ in groups) + '</p>'
    outputs = []
    for folder, title, subset, grouped in groups:
        cohort = "development" if folder == "report" else "diagnostic"
        intro = overview(result, cohort) if folder != "report-probes" else '<section><h2>Additional replay-verified occurrences</h2><p>Preservation, pilot, policy and neighboring-point captures. Separate from the best-point mission trees; each row is a captured probe.</p></section>'
        if folder != "report-probes":
            intro += map_widget([d for d in data if d["cohort"] == cohort])
        intro += common_links + '<p><a href="../map-summary.json">Complete per-arc summaries</a> · <a href="../results.jsonl">Every native Summary occurrence</a> · <a href="../result.json">Execution result</a> · <a href="../paired-efficiency.json">Earlier paired fuel reduction</a></p>'
        spec = {"title": "Coast ignition · " + title, "summary": "Fixed-end timing characterization with complete retained outcomes.",
            "grouped": grouped, "focus_flat": True, "max_bytes": 2 * 2**30,
            "overview_html": scroll_summary_tables(intro),
            "rows": [{"capture": r["capture"], "population": "Development" if r["cohort"] == "development" else "Known diagnostics",
                "label": "Best observed successful timing" if grouped else r["id"] + " · " + (r["rule"] or ("never" if r["slot"] is None else f'{r["slot"]/10:.1f} s')),
                "context_html": capture_context(r)} for r in subset]}
        write_new(campaign / (folder + "-spec.json"), spec)
        target = campaign / folder
        native.render_report(encoded(spec), str(target))
        review = read(target / "report-input.json")
        if any([a["approach"] for a in m["attempts"]] != list(range(len(review["labels"]))) for m in review["missions"]):
            raise ValueError("report has sparse or mislabeled approach slots")
        if sum(len(m["attempts"]) for m in review["missions"]) != len(subset):
            raise ValueError("rich capture inventory differs from projection")
        files = [{"file": str(p.relative_to(target)), "bytes": p.stat().st_size, "sha256": sha(p)} for p in sorted(target.iterdir())]
        write_new(target / "presentation-receipt.json", {"display_only": True, "native_steps": 0,
            "captured_occurrences": len(subset), "dense_slots_verified": True,
            "native_artifact_sha256": sha(NATIVE_ARTIFACT), "presentation_source_sha256": sha(__file__), "files": files})
        outputs.append({"folder": folder, "title": title, "rich_views": len(subset), "report": str(target / "report.html")})
        print(encoded(outputs[-1]), flush=True)
    write_new(campaign / "report-delivery.json", {"display_only": True, "native_steps": 0,
        "unique_captures": len(captures), "reports": outputs})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("--root", required=True)
    render(parser.parse_args().root)
