"""Display-only projection into the maintained mission tree and rich flights."""
import argparse
import html
import json
from pathlib import Path

from arrival import encoded, findings, native, read, sha, write_new


def overview(results, root):
    allrows = []
    sections = []
    for name, result in results:
        f = result["findings"]
        if name != "Post-hoc proof probes":
            sections.append(f'<tr><td>{html.escape(name)}</td><td>{f["original_cases"]}</td><td>{f["direct_native_successes"]}</td>'
                        f'<td>{f["arrival_reached"]}/{f["arrival_attempts"]}</td><td>{f["arrival_reached_and_landed"]}</td>'
                        f'<td>{len(f["recovered_cases"])}</td><td>{len(f["fuel_improvements"])}</td></tr>')
        allrows.extend({**r, "population": name} for r in result["rows"])
    data = encoded(allrows).replace("<", "\\u003c")
    audit = read(root / "actuation-audit.json")
    timing = read(root / "timing-grid-audit.json")
    best = min((r for r in timing["rows"] if r["contact"] == {"stable_touchdown": {"on_target": True}}), key=lambda r:r["fuel_used_kg"])
    diagnosis = f'''<section><h2>The interface works; controller qualification remains open</h2>
<p><strong>The existing waypoint kernel can be exposed as live airborne arrival, but these requested arrival states are not reliably fulfilled.</strong> Native landing success alone hides many failed instructions. The 36-input core records 34 verified direct target landings, 21 fulfilled arrivals among 96 goal attempts, and 19 native Summary successes after fulfilled arrivals. The paired reflection checks and four-input cost pilot remain separate below.</p>
<p>The 24 fresh inputs give 21 verified direct landings and four native Summary successes after fulfilled arrivals. None of the tested arrivals recovers a direct miss. One fresh fulfilled arrival, case 302, saves 285.35 kg (749.23 vs 1,034.57 kg) against first-feasible direct selection. Its full proof is in the post-hoc probes. The direct-grid audit already predicts a cheaper direct candidate at {best["fuel_used_kg"]:.2f} kg, coasting {best["coast_ticks"]/120:g} s then burning {best["duration_ticks"]/120:g} s. That candidate is forecast evidence, not a sealed Episode landing claim. The apparent fuel opportunity needs a stronger timing baseline before it can be attributed to arc shaping.</p>
<h3>A confirmed command-mapping mismatch</h3><p>In a vertical, unclipped case the shared waypoint allocator requests {audit["requested_force_fraction"]*100:.3f}% thrust and emits that value as the raw engine setting. The native engine applies minimum + setting × (1 − minimum), producing {audit["traces"][0]["applied_force_fraction"]*100:.3f}% thrust. An inverse-only diagnostic produces the requested thrust. This confirms a mismatch for intermediate engine settings with nonzero minimum thrust; it does not establish how many full-flight failures it causes. Poly Landing already uses its paired native inverse and is unchanged.</p>
<h3>What the next pass should resolve</h3><ol><li>Define and test force-to-engine-setting allocation, including requests below minimum thrust, without silently changing legacy routes.</li><li>Recheck arrival position/velocity tracking and expose bounded feasibility and time information. The 30 s operation limit currently stops execution; the reused local horizon search is not constrained by that deadline.</li><li>Strengthen direct coast/duration selection before measuring fuel benefit from intermediate goals. Keep first-feasible and fuel-ranked objectives explicit.</li><li>Then redesign the goal bank around demonstrated reachable states and measure useful choices again before RL training.</li></ol>
<p>The 0.72 rad angle limit saturates in many shallow entries, but vertical failures with zero tilt saturation show that removing it cannot be the only answer. This bank is a narrow position/velocity probe, with fixed vehicle/fuel, upright starts and no arrival-attitude contract. It does not qualify arbitrary state control or general high-level learning.</p>
<p><a href="http://starship:8000/coast-ignite-timing/loop-20261022T225427Z/campaign/report-final/report.html">Earlier fixed-end timing study</a> · <a href="http://starship:8787/reports/#project=powered-descent-lab">Project report catalog</a></p></section>'''
    return diagnosis + '''<style>#arrival-case{display:block;width:100%;max-width:640px}#arrival-comparison td{white-space:nowrap}</style><section><h2>Arrival execution and landing are separate gates</h2>
<p>The planner supplies one optional world position and velocity envelope. Existing waypoint guidance owns local commands, horizon search and its 0.72 rad tilt limit. A new deterministic native-forecast selector then chooses one coast/duration pair for the unchanged checked Poly Landing burn.</p>
<p>Every scheduled input and choice is retained. The table below groups native Summary observations by physical input. Each mission-tree occurrence is one selected captured execution, labelled by its actual choice, with physical replay, controller reconstruction, input-goal-rule reconstruction and Summary/Captured equality. A landing after a missed arrival does not satisfy that arrival instruction.</p>
<div style="overflow:auto"><table><thead><tr><th>Population</th><th>Physical inputs</th><th>Direct landings</th><th>Arrival reached / attempts</th><th>Reached + landed</th><th>Inputs rescued by bank</th><th>Inputs with fuel headroom</th></tr></thead><tbody>''' + "".join(sections) + '''</tbody></table></div>
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
    args.output = args.output.resolve()
    results = []
    for path in args.results:
        result = read(path)
        if path.parent.name == "full":
            allocation = read(path.parent.parent / "full-allocation.json")
            for population, label in [("core", "Core factorial"), ("mirror_diagnostic", "Paired reflections")]:
                keys = {Path(c["path"]).stem for c in allocation["cases"] if c["population"] == population}
                part = {"rows": [r for r in result["rows"] if r["case"] in keys],
                        "captures": [c for c in result["captures"] if c["case"] in keys]}
                part["findings"] = findings(part["rows"])
                results.append((label, part))
        else:
            result["findings"] = findings(result["rows"])
            label = {"fresh": "Fresh frozen-rule comparison", "pilot-b": "Cost pilot", "proof-probes": "Post-hoc proof probes"}.get(path.parent.name, path.parent.name)
            results.append((label, result))
    rows = []
    for name, result in results:
        for c in result["captures"]:
            info = read(Path(c["capture"]) / "decisions.json")
            rows.append({"capture": c["capture"], "population": name, "label": c["choice"], "context_html": context(info)})
    spec = {"title": "Airborne arrival → Poly Landing · controller and decision headroom",
            "subtitle": "Flat airborne missions · live waypoint kernel · explicit Poly continuation",
            "summary": "Arrival fulfillment, deterministic continuation and finite-bank headroom are measured separately.",
            "rows": rows, "grouped": False, "focus_flat": True, "max_bytes": 2 * 1024**3,
            "overview_html": overview(results, args.output.parent)}
    write_new(args.output.parent / (args.output.name + "-spec.json"), spec)
    native.render_report(encoded(spec), str(args.output))
    write_new(args.output / "presentation-receipt.json", {"renderer_sha256": sha(__file__),
              "result_inputs": [{"path": str(p.resolve()), "sha256": sha(p)} for p in args.results],
              "scope": "display-only; native source epoch and captures unchanged"})


if __name__ == "__main__":
    main()
