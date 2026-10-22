"""Display-only final edition over sealed results and the maintained report UI."""
from __future__ import annotations

import argparse
import copy
import hashlib
import html
import json
import os
import re
from pathlib import Path


def read(path):
    return json.loads(path.read_text())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, data):
    with path.open("x") as stream:
        stream.write(data)


def encoded(value):
    return json.dumps(value, separators=(",", ":"), allow_nan=False).replace("<", "\\u003c")


def focus_flat_flight(page):
    """Set the initial camera from saved flight/pad/reference bounds only."""
    marker = "const reportData = "
    data, _ = json.JSONDecoder().raw_decode(page[page.index(marker) + len(marker):])
    if len({p["yM"] for p in data["terrain"]}) != 1:
        raise ValueError("this presentation camera is scoped to flat missions")
    xs, ys = [p["xM"] for p in data["samples"]], [p["yM"] for p in data["samples"]]
    pad = data["pad"]
    xs += [pad["centerXM"] - pad["widthM"] / 2, pad["centerXM"] + pad["widthM"] / 2]
    ys.append(pad["surfaceYM"])
    overlay = (data.get("flightAnnotations") or {}).get("referenceOverlay")
    if overlay:
        xs += [p["x"] for p in overlay["points"]]
        ys += [p["y"] for p in overlay["points"]]
    margin_x, margin_y = max(10, 0.08 * (max(xs) - min(xs))), max(10, 0.08 * (max(ys) - min(ys)))
    bounds_x = [min(xs) - margin_x, max(xs) + margin_x]
    bounds_y = [min(ys) - margin_y, max(ys) + margin_y]
    original = 'xaxis: axisStyle({ title: "" }),\n          yaxis: axisStyle({ title: "", scaleanchor: "x", scaleratio: 1 }),'
    replacement = ('xaxis: axisStyle({ title: "", range: ' + encoded(bounds_x) + ' }),\n'
                   '          yaxis: axisStyle({ title: "", scaleanchor: "x", scaleratio: 1, range: ' + encoded(bounds_y) + ' }),')
    if page.count(original) != 1:
        raise ValueError("maintained initial-camera boundary missing")
    # All saved sample/terrain/command bytes remain in the complete payload.
    return page.replace(original, replacement, 1)


def render(learning, output):
    learning, output = Path(learning).resolve(), Path(output).resolve()
    if output.exists() or output.parent != learning:
        raise ValueError("require a new sibling edition under the learning root")
    source = learning / "report"
    result = read(learning / "confirmation/result.json")
    freeze = read(learning / "selection-freeze.json")
    original = read(source / "report-input.json")
    spec = read(learning / "report-spec.json")
    if not result["complete"] or len(spec["rows"]) != len(original["missions"]):
        raise ValueError("incomplete or mismatched report inventory")
    labels = ["RL", "Heuristic", "Immediate", "Never"]
    missions, by_identity, details = [], {}, []
    for old, row in zip(original["missions"], spec["rows"]):
        if row["population"] != "Fresh confirmation":
            continue
        identity = old["physical_sha256"]
        if identity not in by_identity:
            item = copy.deepcopy(old)
            item.update(id=f"mission-{len(missions)}", mission_index=len(missions),
                        label=old["label"].rsplit(" · ", 1)[0], attempts=[])
            by_identity[identity] = item
            missions.append(item)
        mission = by_identity[identity]
        approach = labels.index(row["label"])
        attempt = copy.deepcopy(old["attempts"][0])
        attempt["approach"] = approach
        mission["attempts"].append(attempt)
        details.append((attempt["href"], mission["id"], approach))
    if len(missions) != 600 or len(details) != 2400:
        raise ValueError("fresh schedule must retain exactly 600 arcs and 2400 attempts")
    for mission in missions:
        mission["attempts"].sort(key=lambda a: a["approach"])
        if [a["approach"] for a in mission["attempts"]] != list(range(4)):
            raise ValueError("missing or repeated paired policy")
    for i, label in enumerate(labels):
        if sum(m["attempts"][i]["outcome"] == "landed" for m in missions) != result["counts"][label]:
            raise ValueError("projection landing count differs from certified result")
    groups = {m["group"] for m in missions}
    branches = {p for m in missions for p in m["path"]}
    review = {**original, "labels": labels, "missions": missions,
              "groups": [g for g in original["groups"] if g["id"] in groups],
              "branches": [b for b in original["branches"] if b["id"] in branches]}
    curves = [read(learning / f"seed-{seed}/result.json") for seed in [281101, 281102, 281103]]
    curve_data = [{"seed": seed, "x": [p["checkpoint"] for p in c["curve"]],
                   "successes": [p["successes"] for p in c["curve"]],
                   "selected": c["selected"]["checkpoint"]} for seed, c in zip([281101, 281102, 281103], curves)]
    output.mkdir()
    for href, mission_id, approach in details:
        page = (source / href).read_text()
        page = focus_flat_flight(page)
        page = re.sub(r'data-review-mission="[^"]*"', f'data-review-mission="{mission_id}"', page)
        page = re.sub(r'data-review-approach="\d+"', f'data-review-approach="{approach}"', page)
        page = re.sub(r'href="report.html#mission-\d+"', f'href="report.html#{mission_id}"', page)
        write(output / href, page)
        stem = Path(href).stem
        for suffix in [".svg", "-evidence.json", "-proof.json"]:
            os.link(source / (stem + suffix), output / (stem + suffix))
    document = (source / "report.html").read_text()
    document, replaced = re.subn(r'(<script type="application/json" id="paired-review-data">).*?(</script>)',
                                 lambda m: m[1] + encoded(review) + m[2], document, count=1, flags=re.S)
    if replaced != 1:
        raise ValueError("maintained paired data boundary not found")
    options = "".join(f'<option value="{i}">{label}</option>' for i, label in enumerate(labels))
    for control in ["review-view", "review-a", "review-b", "review-approach"]:
        all_option = '<option value="all">Any approach</option>' if control == "review-approach" else ""
        document, count = re.subn(rf'(<select id="{control}">).*?(</select>)',
                                  lambda m: m[1] + all_option + options + m[2], document, count=1, flags=re.S)
        if count != 1:
            raise ValueError(f"maintained control boundary missing: {control}")
    table = []
    for label in labels:
        count = result["counts"][label]
        lo, hi = result["intervals"][label]
        table.append(f"<tr><th>{label}</th><td>{count}/600</td><td>{count / 6:.2f}%</td><td>{100 * lo:.2f}–{100 * hi:.2f}%</td></tr>")
    curve_table = "".join(f"<tr><th>{c['seed']}</th><td>{p['checkpoint']:,}</td><td>{p['successes']}/64</td><td>{'Selected' if p['checkpoint'] == c['selected'] else ''}</td></tr>" for c, run in zip(curve_data, curves) for p in run["curve"])
    advantage = "The selected RL policy demonstrates a paired advantage." if result["demonstrated_advantage"] else "This run does not demonstrate an RL advantage over the selected heuristic."
    baseline = (f"a fixed {freeze['baseline']['seconds']:g}-second delay" if freeze["baseline"]["kind"] == "delay" else html.escape(str(freeze["baseline"])))
    diagnostic_path = learning / "confirmation/diagnostic-misses.json"
    diagnostic = read(diagnostic_path)
    phases = "".join(f"<tr><th>{name.title()}</th><td>{diagnostic['initial_phase'][name]['arcs']}</td><td>{diagnostic['initial_phase'][name]['RL successes']}</td><td>{diagnostic['initial_phase'][name]['Heuristic successes']}</td></tr>" for name in ["ascending", "near-apex", "descending"])
    overview = f'''<section id="coast-results"><h2>600 fresh shared arcs</h2>
<p><strong>{advantage}</strong> Every policy executes a complete native episode from each identical starting input. All 2,400 slots have physical replay, controller reconstruction and policy reconstruction.</p>
<table><thead><tr><th>Policy</th><th>Verified target landings</th><th>Rate</th><th>Exact 95% interval</th></tr></thead><tbody>{''.join(table)}</tbody></table>
<p>RL-only wins: <strong>{result['rl_only_wins']}</strong>; heuristic-only wins: <strong>{result['heuristic_only_wins']}</strong>. Absolute lift: <strong>{100 * result['lift']:.2f} percentage points</strong>. Paired one-sided exact p = {result['one_sided_p']:.5g}.</p>
<p>The validation-selected heuristic is {baseline}. The primary RL package is seed {freeze['primary_seed']}, checkpoint {freeze['selected_model']['checkpoint']:,}. Package selection was frozen before these fresh inputs.</p>
<h2>What the policy learned, and the remaining misses</h2><p>Ignition varies from 0 to 7.1 s across 71 distinct decision slots, with a median of 3.6 s. The 11 RL misses comprise seven rejected burn requests followed by passive-coast crashes and four crashes after an accepted burn. <a href="?view=0&amp;approach=0&amp;outcome=misses">Inspect the 11 RL misses</a>.</p>
<details><summary>Descriptive breakdown by initial arc phase</summary><table><thead><tr><th>Initial phase</th><th>Arcs</th><th>RL successes</th><th>Heuristic successes</th></tr></thead><tbody>{phases}</tbody></table><p>These are subgroups of the same 600 arcs, not separate qualification populations.</p></details>
<h2>Learning across three independent seeds</h2><p>Native scores on the same 64 exposed validation arcs; selected checkpoints were subsequently captured and replay verified. Fresh results above qualify the selected package on this task.</p>
<div id="learning-curve" role="img" aria-label="Validation success across three training seeds" style="min-height:320px"></div>
<details><summary>All validation scores and selected checkpoints</summary><table><thead><tr><th>Seed</th><th>Decisions</th><th>Native successes</th><th>Selection</th></tr></thead><tbody>{curve_table}</tbody></table></details>
<h2>Task and evidence boundaries</h2><p>The mission supplies a target-directed ballistic arc, prepared attitude and absolute touchdown time. RL chooses WAIT or IGNITE at 10 Hz; the controller owns all coast and landing commands. This establishes a bounded hybrid task; generalized flight control, launch, terrain avoidance and duration selection remain outside its scope.</p>
<p><a href="../report/report.html?group=population-0">Validation flights and original full capture tree</a> · <a href="../confirmation/result.json">Complete confirmation outcome accounting</a> · <a href="../confirmation/resource-accounting-supplement.json">Sealed-record resource accounting</a> · <a href="../selection-freeze.json">Frozen selection and source bindings</a> · <a href="../confirmation/schedule.json">All 2,400 scheduled slots</a> · <a href="presentation-receipt.json">Presentation receipt</a></p></section>'''
    document, count = re.subn(r'<section><h2>Measured result</h2>.*?</section>', overview,
                              document, count=1, flags=re.S)
    if count != 1:
        raise ValueError("native overview boundary missing")
    document = document.replace("2656 complete rich flight views", "600 fresh arcs · 2,400 complete rich flight views")
    document = document.replace("Complete procedural transfers → terrain family → terrain stratum → mission. Controller results sit inside each mission.",
                                "Flat supplied arcs → population → mission. The four policy results sit inside each mission.")
    document = document.replace("one stratum", "one mission group").replace("terrain groups", "mission groups").replace("Expand families", "Expand populations")
    graph = f'''<script src="https://cdn.plot.ly/plotly-basic-2.35.2.min.js"></script><script>
if(typeof Plotly!=="undefined"){{const curves={encoded(curve_data)};Plotly.newPlot('learning-curve',curves.map(c=>({{x:c.x,y:c.successes.map(n=>100*n/64),name:'Seed '+c.seed,mode:'lines+markers',type:'scatter',customdata:c.successes,hovertemplate:'%{{x:,}} decisions<br>%{{customdata}}/64 successes<extra>%{{fullData.name}}</extra>'}})),{{margin:{{l:55,r:25,t:20,b:55}},xaxis:{{title:'Policy decisions'}},yaxis:{{title:'Native validation success (%)',range:[0,105]}},legend:{{orientation:'h'}},paper_bgcolor:'transparent',plot_bgcolor:'transparent'}},{{responsive:true,displaylogo:false}});}}
</script>'''
    document = document.replace("</body>", graph + "</body>")
    write(output / "report.html", document)
    for name in ["report-input.json", "navigation.json"]:
        write(output / name, encoded(review) + "\n")
    inputs = [source / "report.html", source / "report-input.json", learning / "report-spec.json",
              learning / "selection-freeze.json", learning / "confirmation/result.json", diagnostic_path,
              learning / "confirmation/resource-accounting-supplement.json", Path(__file__).resolve()]
    files = [{"file": str(p.relative_to(output)), "sha256": digest(p), "bytes": p.stat().st_size}
             for p in sorted(output.iterdir())]
    receipt = {"schema_version": 1, "display_only": True, "native_steps": 0,
               "independent_fresh_arcs": 600, "scheduled_attempts": 2400, "rich_views": 2400,
               "inputs": [{"path": str(p), "sha256": digest(p)} for p in inputs], "files": files}
    write(output / "presentation-receipt.json", encoded(receipt) + "\n")
    print(json.dumps({"report": str(output / "report.html"), "arcs": 600, "rich_views": 2400,
                      "bytes": sum(f["bytes"] for f in files)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--learning", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    render(args.learning, args.output)
