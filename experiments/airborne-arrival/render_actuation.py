"""Display-only calibration comparison using the established rich report tree."""
import argparse
import collections
import html
from pathlib import Path
from actuation_study import compact, encoded, native, read, sha, write_new


def aggregate(rows):
    groups = collections.defaultdict(list)
    for row in rows:
        if row["phase"] == "pilot":
            continue
        groups[(row["population"], row["mode"])].append(row["result"])
        if "legacy" in row and row["mode"] == "native_pair_v2":
            groups[(row["population"], "retained legacy")].append(row["legacy"])
    result = []
    for (population, mode), group in sorted(groups.items()):
        arrived = [r for r in group if r["arrival_status"] != "direct"]
        result.append(f'<tr><td>{html.escape(population)}</td><td>{html.escape(mode)}</td><td>{len(group)}</td>'
            f'<td>{sum(r["arrival_status"] == "reached" for r in arrived)}/{len(arrived)}</td>'
            f'<td>{sum(r["success"] for r in group)}</td><td>{sum(r["success"] and r["arrival_status"] == "reached" for r in group)}</td></tr>')
    return "".join(result)


def context(info, mode):
    a = info["arrival"]
    if not a:
        return '<p>Direct Poly continuation control. No waypoint allocation is used.</p>'
    text = f'<section><h2>Local arrival outcome</h2><p>Mode: {html.escape(mode)}. Status: <strong>{a["status"]}</strong>. '
    text += f'Window entry tick {a["window_entry_physics_step"]}; resolution tick {a["resolution"]["physics_step"] if a["resolution"] else "none"}. '
    text += f'Replans {a["replans"]}; tilt-limited updates {a["tilt_saturated_updates"]}/{a["command_updates"]}.</p>'
    if "witness_recipe" in info:
        text += '<p>This independent native command recipe demonstrates goal reachability. Engine-off observation continues afterward to the original native end. A later tail crash is separate from local witness fulfillment.</p>'
    elif a.get("actuation"):
        v = a["actuation"]
        text += f'<p>Allocation regimes: {html.escape(str(v["regime_updates"]))}. Engine switches: {v["engine_switches"]}. Last local horizon remaining {a["deadline_remaining_s"]:.3f} s; operation time remaining {v["operation_remaining_s"]:.3f} s. '
        text += f'Last ideal contract prediction: {v["ideal_contract_pass"]}; actuator prediction: {v["actuator_forecast_contract_pass"]}; earlier ideal pass: {v["ideal_pass_ever"]}. These are diagnostic predictions.</p>'
    text += '<p>Physical landing, local instruction fulfillment and evidence integrity are separate claims. Per-update CPU time is uninstrumented; campaign steps and phase wall costs are the cost evidence.</p></section>'
    return text


def findings(analysis):
    old, new = analysis["overall"]["legacy"], analysis["overall"]["result"]
    witness_rows = "".join(f'<tr><td>{html.escape(p.removeprefix("witness_").replace("_", " "))}</td>'
        f'<td>{v["witness"]["reached"]}/{v["witness"]["arrival_attempts"]}</td>'
        f'<td>{v["legacy_v1"]["reached"]}/{v["legacy_v1"]["arrival_attempts"]}</td>'
        f'<td>{v["native_pair_v2"]["reached"]}/{v["native_pair_v2"]["arrival_attempts"]}</td></tr>'
        for p, v in analysis["witness_families"].items())
    angle_rows = "".join(f'<tr><td>{angle}°</td><td>{v["legacy"]["reached"]}/24</td>'
        f'<td>{v["result"]["reached"]}/24</td><td>{v["legacy"]["arrival_native_success"]} → '
        f'{v["result"]["arrival_native_success"]}</td></tr>' for angle, v in analysis["core_angles_degrees"].items())
    losses = [d for d in analysis["corrected_diagnostics"] if d["case"] in analysis["lost_landings"]]
    loss_rows = "".join(f'<tr><td>{html.escape(d["case"])}</td><td>{d["fuel_used_kg"]:.2f}</td>'
        f'<td>{html.escape(str(d["regimes"]))}</td><td>{d["operation_time_remaining_s"]:.2f} s</td></tr>' for d in losses)
    return f'''<section><h2>What this pass establishes</h2>
<p><strong>Calibrated allocation and bounded local arrival tracking pass. Broad arrival-plus-landing qualification does not pass.</strong>
The 504 native command-pair checks pass. All 21 declared native witness recipes are admitted without exclusions; the corrected controller fulfills 21/21 goals versus 13/21 for legacy.</p>
<p>The unchanged retained panel improves from {old["reached"]}/{old["arrival_attempts"]} to {new["reached"]}/{new["arrival_attempts"]} fulfilled arrival instructions.
Native target landing observations on those arrival programs fall from {old["arrival_native_success"]} to {new["arrival_native_success"]}: nine lost and one gained.
Fourteen arrivals are gained and one is lost. The direct Poly controls remain exactly unchanged ({new["direct_native_success"]}/{new["direct_attempts"]} native target successes).
The legacy default stays selected; there is no RL training or default promotion.</p>
<div style="overflow:auto"><table><thead><tr><th>Witness family</th><th>Independent native witness</th><th>Legacy controller</th><th>Corrected controller</th></tr></thead><tbody>{witness_rows}</tbody></table></div>
<p>These are six-second airborne diagnostics with 30 m capture radius, ±15% speed bounds, 0.25 rad heading bound and bounded cross-speed. The target is derived from an independent native recipe; execution and proof use the unchanged first-window/plane predicate. Reflected recipes are paired diagnostics. This is a small operating envelope, not generalized state control or landing qualification. Landing is disabled for this cohort; later engine-off tail crashes are expected.</p>
<div style="overflow:auto"><table><thead><tr><th>Core entry angle</th><th>Legacy arrivals</th><th>Corrected arrivals</th><th>Native landing observations, legacy → corrected</th></tr></thead><tbody>{angle_rows}</tbody></table></div>
<p>The 60° and 80° core entries still fulfill zero arrival instructions. The 96 core arrival programs include several goals on shared initial states; they are not independent mission samples. Two corrected arrivals succeed locally but find no Poly continuation; local fulfillment alone does not establish a safe handoff.</p></section>
<section><h2>Regressions and the next boundary</h2>
<p>All nine newly lost native landings resolve the arrival as a plane miss and the unchanged finite continuation search returns no plan. Eight spend zero fuel: every arrival update selects <code>gap_off</code>. The ninth mixes minimum, off and tracking.</p>
<p>The legacy geometry projection clips downward demand and lateral authority. For a large lateral request with downward vertical demand, its surviving projected magnitude can be only 0.1754 m/s², below the minimum-thrust gap. Legacy interprets that small nonzero fraction as an engine setting and produces minimum thrust; corrected allocation selects off as the nearer scalar demand. That removes incidental braking. This is an interaction exposed by the physical correction, not proof that either entire trajectory was optimal.</p>
<div style="overflow:auto"><table><thead><tr><th>Newly lost native landing (Summary)</th><th>Fuel kg</th><th>Corrected allocation updates</th><th>Operation time remaining at final command</th></tr></thead><tbody>{loss_rows}</tbody></table></div>
<p>The remaining gaps are geometric projection and minimum-thrust policy, finite horizon/recovery, and continuation-aware handoff selection. The final diagnostics show 88 plane misses after an earlier ideal pass, and two expirations while the last predictions still pass. They identify a model/recovery limitation; they do not isolate its cause. The 0.72 rad tilt limit has not been relaxed or proven responsible.</p>
<p><strong>Recommended next pass:</strong> investigate a native-validated feasible-goal and continuation screen, plus allocation-aware recovery for the retained failures. Declare the operating envelope before training an RL planner. Preserve these failed cases and tolerances. Broader goal discovery, timing/fuel optimization, angle-limit changes and RL remain separate work.</p>
<p>Cost: 65,416,717 measured native/model/forecast/replay steps, 60 complete captures, zero optimizer steps. All 19 maintained development checks pass. The independent read-only audit verifies source/input bindings, all selected proofs, unchanged direct/legacy controls and the fly-through outcome/tick from captured native observations. Per-update CPU timing remains uninstrumented.</p></section>'''


def overview(rows, analysis):
    data = encoded(rows).replace("<", "\\u003c")
    return findings(analysis) + '''<section><h2>Actuator calibration and arrival qualification</h2>
<p>The new native-pair allocation is opt-in. Established defaults, Poly Landing, the 0.72 rad tilt limit, local horizon/recovery rules, waypoint-plane termination and operation deadlines remain fixed. Below minimum thrust, the allocator explicitly chooses the closer aligned acceleration from off or minimum; it records the resulting allocation error and switching.</p>
<p><img src="actuation-calibration.svg" alt="Native applied acceleration versus projected demand for legacy and corrected allocation" style="max-width:100%;height:auto"></p>
<p>The calibrated inverse applies to aligned scalar thrust. Rotation is still limited by native physics. The local forecast keeps its 60 Hz approximation and finite horizon; a forecast is not a complete feasibility certificate.</p>
<div style="overflow:auto"><table><thead><tr><th>Population</th><th>Mode</th><th>Occurrences</th><th>Arrival reached / attempts</th><th>Native target successes</th><th>Reached + native success</th></tr></thead><tbody>''' + aggregate(rows) + '''</tbody></table></div>
<p>Retained inputs are exposed regression data. Reflections are paired diagnostics, not independent samples. Witnesses use independently generated native command recipes and the unchanged fly-through predicate. Witness and controller observation tails run to the original native end. Summary target successes are provisional observations; rich captured flights carry the complete physical/mission/integrity/replay evidence.</p>
<p><a href="http://starship:8000/airborne-arrival/loop-20261023T222023Z/report-delivery/report.html">Previous arrival characterization</a></p></section>
<section><h2>Inspect the paired outcomes</h2><label>Program <select id="actuation-case" style="display:block;max-width:100%;width:680px"></select></label><div style="overflow:auto"><table><thead><tr><th>Mode</th><th>Arrival</th><th>Native success</th><th>Fuel used kg</th><th>Time s</th><th>Replans</th></tr></thead><tbody id="actuation-results"></tbody></table></div></section>
<script>(()=>{const rows=''' + data + ''';const s=document.getElementById('actuation-case'),b=document.getElementById('actuation-results');
for(const id of [...new Set(rows.map(r=>r.case))]){const o=document.createElement('option');o.value=id;o.textContent=id;s.append(o)}
function render(){b.replaceChildren();const selected=rows.filter(r=>r.case===s.value);const values=selected.map(r=>[r.mode,r.result]);if(selected[0]?.legacy)values.unshift(['retained legacy',selected[0].legacy]);for(const [mode,v] of values){const tr=document.createElement('tr');for(const value of [mode,v.arrival_status,v.success?'yes':'no',v.fuel_used_kg.toFixed(2),v.stop_s.toFixed(2),v.replans]){const td=document.createElement('td');td.textContent=value;tr.append(td)}b.append(tr)}}s.addEventListener('change',render);render()})();</script>'''


def main():
    p = argparse.ArgumentParser()
    p.add_argument("output", type=Path)
    p.add_argument("results", nargs="+", type=Path)
    args = p.parse_args()
    args.output = args.output.resolve()
    results = [read(path) for path in args.results]
    rows = [dict(row, phase=path.parent.name) for path, result in zip(args.results, results) for row in result["rows"]]
    # Pilot repeats remain in the rich evidence tree, but neither the aggregate
    # comparison nor the program picker counts a repeated corrected Summary.
    rows = list({(row["case"], row["mode"]): row for row in rows}.values())
    captures = [dict(c, phase=path.parent.name) for path, result in zip(args.results, results) for c in result["captures"]]
    analysis_path = args.output.parent / "analysis.json"
    analysis = read(analysis_path)
    spec = {"title": "Arrival actuation calibration and witnessed control",
        "subtitle": "Opt-in native-pair allocation · unchanged fly-through contract · established rich tree",
        "summary": "Calibrated commands, witnessed reachability and retained-input comparison are separate gates.",
        "grouped": False, "focus_flat": True, "max_bytes": 1024**3, "overview_html": overview(rows, analysis),
        "rows": [{"capture": c["capture"], "population": ("pilot_core" if c["phase"] == "pilot" else next(r["population"] for r in rows if r["case"] == c["case"] and r["mode"] == c["mode"])),
                  "label": c["phase"] + " · " + c["mode"], "context_html": context(read(Path(c["capture"]) / "decisions.json"), c["mode"])} for c in captures]}
    write_new(args.output.parent / (args.output.name + "-spec.json"), spec)
    native.render_report(encoded(spec), str(args.output))
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    bench = read(args.output.parent / "calibration.json")
    fig, ax = plt.subplots(figsize=(8, 4.5), layout="constrained")
    for mode, color in [("legacy_v1", "#b65c2d"), ("native_pair_v2", "#146e62")]:
        points = [r for r in bench["cases"] if r["minimum"] == .25 and r["fuel"] == 6300 and not r["rotating"] and r["profile"] == mode]
        ax.plot([r["allocation"]["projected_magnitude_mps2"] for r in points], [r["native_mean_accel_mps2"]["y"] for r in points], "o-", label=mode, color=color)
    ax.plot([0, 18], [0, 18], "--", color="#686868", label="Exact requested acceleration")
    ax.set(xlabel="Projected desired thrust acceleration (m/s²)", ylabel="Native pair mean thrust acceleration (m/s²)")
    ax.grid(alpha=.2);ax.legend()
    fig.savefig(args.output / "actuation-calibration.svg")
    plt.close(fig)
    write_new(args.output / "presentation-receipt.json", {"renderer_sha256": sha(__file__), "analysis_sha256": sha(analysis_path),
        "inputs": [{"path": str(p.resolve()), "sha256": sha(p)} for p in args.results], "rich_captures": len(captures)})


if __name__ == "__main__":
    main()
