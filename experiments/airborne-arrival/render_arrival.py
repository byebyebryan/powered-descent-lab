"""Display-only projection into the maintained mission tree and rich flights."""
import argparse
import html
import json
from pathlib import Path

from arrival import encoded, native, read, write_new


def overview(results):
    allrows = []
    sections = []
    for name, result in results:
        f = result["findings"]
        sections.append(f'<tr><td>{html.escape(name)}</td><td>{f["original_cases"]}</td><td>{f["direct_native_successes"]}</td>'
                        f'<td>{f["arrival_reached"]}/{f["arrival_attempts"]}</td><td>{f["arrival_reached_and_landed"]}</td>'
                        f'<td>{len(f["recovered_cases"])}</td><td>{len(f["fuel_improvements"])}</td></tr>')
        allrows.extend({**r, "population": name} for r in result["rows"])
    data = encoded(allrows).replace("<", "\\u003c")
    return '''<section><h2>Arrival execution and landing are separate gates</h2>
<p>The planner supplies one optional world position and velocity envelope. Existing waypoint guidance owns local commands, horizon search and its 0.72 rad tilt limit. A new deterministic native-forecast selector then chooses one coast/duration pair for the unchanged checked Poly Landing burn.</p>
<p>Every scheduled input and choice is retained. The table below contains native Summary observations; the mission tree contains the selected fully captured executions with physical replay, controller reconstruction, input-goal-rule reconstruction and Summary/Captured equality. A landing after a missed arrival does not satisfy that arrival instruction.</p>
<table><thead><tr><th>Population</th><th>Physical inputs</th><th>Direct landings</th><th>Arrival reached / attempts</th><th>Reached + landed</th><th>Inputs rescued by bank</th><th>Inputs with fuel headroom</th></tr></thead><tbody>''' + "".join(sections) + '''</tbody></table>
<p>“Rescued” and fuel headroom use hindsight selection among tested choices, with arrival fulfillment required. They do not establish an online policy, global optimum or general controller reliability. Mirrors are paired checks, not independent samples. No optimizer ran.</p>
<details><summary>Read the fixed choice bank and timing selector</summary><p>Arrival positions are 35% or 60% of the straight displacement from entry to the pad. Arrival speed is 40%, 70% or 100% of entry speed, with a 12 m/s floor, directed along that displacement. Radius and cross-track tolerance: 30 m. Speed tolerance: ±15%; heading: 0.25 rad; cross velocity: 25%; vertical speed tolerance: ±15% of speed. Absolute arrival deadline: 30 s. All missions start upright and retain their original 90 s deadline.</p><p>Landing selection uses first feasible native touchdown, coast-major order: coasts 0, 0.5, 1, 2, 3, 4, 6 s; durations 6, 8, 10, 12, 16, 20, 24, 30, 36 s. Engine-off coast prepares the first burn attitude. The finite grid is not a feasibility certificate outside its choices or a fuel optimizer.</p></details>
</section><section><h2>Inspect every scheduled choice</h2><label>Physical input <select id="arrival-case"></select></label><div style="overflow:auto"><table><thead><tr><th>Choice</th><th>Arrival</th><th>Physical landing</th><th>Fuel used kg</th><th>Flight s</th><th>Tilt-limited ticks</th><th>Replans</th><th>Landing grid</th></tr></thead><tbody id="arrival-comparison"></tbody></table></div><p id="arrival-metadata"></p></section>
<script>(()=>{const rows=''' + data + ''';const select=document.getElementById('arrival-case'),body=document.getElementById('arrival-comparison');
const keys=[...new Set(rows.map(r=>r.population+' / '+r.case))];for(const k of keys){const o=document.createElement('option');o.value=k;o.textContent=k;select.append(o)}
function render(){body.replaceChildren();for(const r of rows.filter(r=>r.population+' / '+r.case===select.value)){const tr=document.createElement('tr'),v=r.result;for(const t of [r.choice,v.arrival_status,v.success?'native target success':'no target landing',v.fuel_used_kg.toFixed(2),v.stop_s.toFixed(2),v.tilt_saturated_updates+' / '+v.arrival_updates,v.replans,v.no_plan?'no tested plan':v.candidates+' candidates']){const td=document.createElement('td');td.textContent=t;tr.append(td)}body.append(tr)}}select.addEventListener('change',render);render()})();</script>'''


def context(info):
    a = info["arrival"]
    if not a:
        return "<section><h2>Direct landing control</h2><p>No intermediate arrival request.</p></section>"
    w, anchor = a["request"]["waypoint"], a["anchor"]
    resolution = a["resolution"]
    text = f'<section><h2>Requested airborne arrival</h2><p>Status: <strong>{html.escape(a["status"])}</strong>. '
    text += f'Goal ({w["position_m"]["x"]:.1f}, {w["position_m"]["y"]:.1f}) m; radius {w["capture_radius_m"]:.1f} m; speed {w["min_speed_mps"]:.1f}–{w["max_speed_mps"]:.1f} m/s. '
    text += f'Window entry tick: {a["window_entry_physics_step"]}; resolution tick: {resolution["physics_step"] if resolution else "none"}. Replans: {a["replans"]}; tilt-limited updates: {a["tilt_saturated_updates"]}/{a["command_updates"]}.</p>'
    text += '<p>The full rich flight below retains native samples, attitude, engine setting, fuel, phase and arrival metrics. The saved Poly reference overlay starts at its actual later burn activation.</p></section>'
    return text


def main():
    p = argparse.ArgumentParser()
    p.add_argument("output", type=Path)
    p.add_argument("results", nargs="+", type=Path)
    args = p.parse_args()
    results = [(path.parent.name, read(path)) for path in args.results]
    rows = []
    for name, result in results:
        for c in result["captures"]:
            info = read(Path(c["capture"]) / "decisions.json")
            rows.append({"capture": c["capture"], "population": name, "label": c["choice"], "context_html": context(info)})
    spec = {"title": "Airborne arrival → Poly Landing · controller and decision headroom",
            "subtitle": "Flat airborne missions · live waypoint kernel · explicit Poly continuation",
            "summary": "Arrival fulfillment, deterministic continuation and finite-bank headroom are measured separately.",
            "rows": rows, "grouped": True, "focus_flat": True, "max_bytes": 2 * 1024**3,
            "overview_html": overview(results)}
    write_new(args.output.parent / (args.output.name + "-spec.json"), spec)
    native.render_report(encoded(spec), str(args.output))


if __name__ == "__main__":
    main()
