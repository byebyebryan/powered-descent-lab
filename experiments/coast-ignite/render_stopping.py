"""Established rich mission views with stopping evidence; display only."""
from __future__ import annotations

import argparse
import html
import json
import re
from pathlib import Path

from coast_ignite import encoded, native, sha, write_new


def read(path):
    return json.loads(Path(path).read_text())


def scroll_summary_tables(markup):
    return ('<style>.stopping-table-wrap{max-width:100%;min-width:0;overflow-x:auto}</style>'
            + re.sub(r'<table>(.*?)</table>',
                     r'<div class="table-wrap stopping-table-wrap"><table>\1</table></div>',
                     markup, flags=re.S))


def stopping_context(row, campaign):
    folder = Path(row["capture"])
    proof = read(folder / "stopping-proof.json")
    for name, key in [("diagnostics.json", "diagnostics_sha256"),
                      ("stopping.json", "stopping_sha256"),
                      ("evaluation.json", "capture_sha256"),
                      ("decisions.json", "decisions_sha256")]:
        if sha(folder/name) != proof[key]:
            raise ValueError("stopping sidecar does not match its complete proof")
    if not proof["random_stream_reconstructed"]:
        raise ValueError("unreconstructed stopping stream")
    diagnostic = read(folder/"diagnostics.json")
    stopping = read(folder/"stopping.json")
    admission = diagnostic["admission"]
    admission_text = "No ignition request" if admission is None else admission["kind"] + (": " + admission["reason"] if "reason" in admission else "")
    contact = diagnostic["incoming_contact"]
    contact_html = "<p>No native contact in the declared coverage.</p>"
    if contact:
        g = contact["geometry"]
        contact_html = (f'<p>Native contact: <strong>{html.escape(str(contact["recorded"]["classification"]))}</strong>. '
                        f'Foot clearances {g["min_touchdown_clearance_m"]:.4f} to {g["max_touchdown_clearance_m"]:.4f} m; '
                        f'normal speed {g["normal_speed_mps"]:.3f} m/s; tangential speed {g["tangential_speed_mps"]:.3f} m/s; '
                        f'attitude error {g["attitude_error_rad"]:.4f} rad.</p>')
    projection = diagnostic["reference_projection"]
    reference_html = ""
    if projection and "reference" in projection:
        first = projection["first_contact"]
        when = f'{first["sim_time_s"]:.3f} s' if first else "no contact"
        reference_html = (f'<p>Reference-only first contact: {when}; endpoint {projection["reference_end_physics_step"]/120:.3f} s; '
                          f'minimum center height {projection["minimum_center_height_m"]:.3f} m. '
                          'Assumes instantaneous requested pointing and zero angular rate. This is query evidence, not a native forecast or touchdown certificate.</p>')
    trace = stopping["trace"]
    table = "".join(f'<tr><td>{p["physics_tick"]/120:.1f}</td><td>{p["hazard"]:.5f}</td><td>{p["cumulative_stopping_probability"]:.5f}</td><td>{p["draw"] if p["draw"] is not None else "—"}</td><td>{"IGNITE" if p["action"] else "WAIT"}</td></tr>' for p in trace if p["hazard"] is not None)
    curve = ""
    if trace and trace[0]["hazard"] is not None:
        def points(field):
            return " ".join(f'{40+640*(p["physics_tick"]/120)/8:.2f},{160-140*p[field]:.2f}' for p in trace)
        curve = (f'<svg viewBox="0 0 710 190" role="img" aria-label="Ignition hazard and cumulative stopping probability" style="width:100%;max-width:850px">'
                 '<path d="M40 20V160H680" fill="none" stroke="#64748b"/><path d="M40 90H680" stroke="#94a3b8" stroke-dasharray="4 4"/>'
                 '<text x="8" y="25" fill="currentColor">1</text><text x="8" y="96" fill="currentColor">0.5</text><text x="8" y="164" fill="currentColor">0</text>'
                 f'<polyline points="{points("hazard")}" fill="none" stroke="#38bdf8" stroke-width="2"/>'
                 f'<polyline points="{points("cumulative_stopping_probability")}" fill="none" stroke="#f59e0b" stroke-width="2"/>'
                 '<text x="40" y="185" fill="currentColor">0 s</text><text x="650" y="185" fill="currentColor">8 s</text></svg>'
                 '<p>Blue: conditional ignition probability. Amber: cumulative stopping probability along the observed coast prefix. Dashed line: 0.5.</p>')
    relative = folder.relative_to(campaign).as_posix()
    return (f'<section class="stopping-evidence"><h2>Stopping and contact evidence</h2><p><strong>{html.escape(admission_text)}</strong></p>'
            + contact_html + reference_html + curve
            + f'<details><summary>Every stopping decision</summary><table><thead><tr><th>Time s</th><th>Hazard</th><th>Cumulative probability</th><th>Uniform draw</th><th>Decision</th></tr></thead><tbody>{table}</tbody></table></details>'
            + f'<p><a href="../{relative}/stopping.json">Complete stopping trace</a> · <a href="../{relative}/diagnostics.json">Native and reference diagnostics</a> · <a href="../{relative}/stopping-proof.json">Bound sidecar reconstruction proof</a></p></section>')


def overview(campaign, rows):
    result = read(campaign/"result.json")
    development, validation = [read(campaign/p/"result.json") for p in ["development", "validation"]]
    verdict = ("Fresh confirmation released after the prospective validation gate." if result["fresh_released"] else
               "Fresh confirmation withheld: the prospective validation nomination gate did not pass. The existing greedy package is retained.")
    table=[]
    for phase, data in [("Development", development), ("Validation", validation)]:
        for seed, scores in data["scores"].items():
            for rule, score in scores.items():
                table.append(f'<tr><td>{phase}</td><td>{seed}</td><td>{rule}</td><td>{score["successes"]}/{score["occurrences"]}</td><td>{100*score["successes"]/score["occurrences"]:.2f}%</td><td>{score["independent_arcs"]}</td></tr>')
    fresh = ""
    if result["fresh_released"]:
        f = result["fresh"]
        packages = {}
        for row in rows:
            if row["population"] == "Fresh confirmation" and row["replicate"] == 0:
                label = f'Seed {row["seed"]} · {row["rule"]}' if row["seed"] else 'Fixed three-second delay'
                packages.setdefault(label, [0,0])
                packages[label][0] += int(row["verified_target_landing"])
                packages[label][1] += 1
        package_table = ''.join(f'<tr><th>{html.escape(label)}</th><td>{count}/{total}</td><td>{100*count/total:.3f}%</td></tr>' for label,(count,total) in packages.items())
        fresh = (f'<h2>Independent confirmation</h2><p>Primary candidate {f["primary_successes"]}/1200; current greedy {f["baseline_successes"]}/1200. '
                 f'Gains {f["gains"]}, losses {f["losses"]}; paired lift {100*f["lift"]:.3f} percentage points, exact one-sided p {f["one_sided_p"]:.6g}. '
                 f'Demonstrated improvement: {f["demonstrated_improvement"]}. Separate 99% reliability criterion: {f["reliability99_one_sided95_passed"]}.</p>'
                 f'<table><thead><tr><th>Frozen package</th><th>Verified target landings</th><th>Rate</th></tr></thead><tbody>{package_table}</tbody></table>'
                 '<p>These are conditional results for the supplied-arc task and previously selected weights. Only the fixed primary weight has a matched fresh greedy comparator; the other weights do not supply independent training-seed qualification.</p>')
    return ('<section id="stopping-results"><h2>Frozen-policy stopping study</h2>'
            f'<p><strong>{verdict}</strong></p><p>Selected rule: {html.escape(result["selected_rule"])}. '
            'Zero training updates. G is the original local greedy decision; S samples the frozen hazard using recorded uniforms; M uses the fixed cumulative median.</p>'
            + fresh + '<h2>All three frozen checkpoints</h2><p>These development and validation outcomes are native Summary scores. S has eight correlated draws per arc and weight; average repeats within each arc. They are not independent additional missions or replay-certified landing claims.</p>'
            '<table><thead><tr><th>Phase</th><th>Frozen seed</th><th>Rule</th><th>Native successes / occurrences</th><th>Rate</th><th>Independent arcs</th></tr></thead><tbody>'
            + "".join(table) + '</tbody></table>'
            f'<p>Validation aggregate lift over G: {100*validation["aggregate_lift"]:.3f} percentage points; fixed-primary lift: {100*validation["primary_lift"]:.3f}. Nomination requires at least 0.5 points for both.</p>'
            f'<h2>Captured flight evidence</h2><p>The report links {len(rows)} complete rich captures across the populations below: preservation controls, the four-arc cost pilot, selected validation proof, and fresh confirmation if released. Each has physical replay, controller reconstruction, live policy reconstruction and bound diagnostic sidecars.</p>'
            '<p>Captures are grouped by physical mission and population. Summary-only development/validation occurrences remain in their complete ledgers; they do not have invented flight views. The original 600-case result is preserved.</p>'
            '<p><a href="../result.json">Run closure</a> · <a href="../selection-freeze.json">Frozen selection</a> · <a href="../development/result.json">Development result</a> · <a href="../validation/result.json">Validation result</a> · <a href="../development/results.jsonl">All development occurrences</a> · <a href="../validation/results.jsonl">All validation occurrences</a> · <a href="presentation-receipt.json">Report receipt</a></p></section>')


def split_reports(rows):
    populations = {}
    for row in rows:
        populations.setdefault(row["population"], []).append(row)
    result = []
    if "Fresh confirmation" in populations:
        fresh = populations.pop("Fresh confirmation")
        primary = [r for r in fresh if r.get("role") != "secondary"]
        def order(row):
            if row.get("role") == "candidate" and row["seed"] == 281103: return 0
            if row.get("role") == "baseline": return 1
            if row.get("role") == "candidate": return 2 if row["seed"] == 281101 else 3
            return 4
        primary.sort(key=lambda row: (row["arc"], order(row)))
        result.append(("report", "Fresh confirmation", primary))
        secondary = [r for r in fresh if r.get("role") == "secondary"]
        if secondary:
            secondary += [r for r in primary if r["seed"] == 281103 and r.get("role") == "candidate" and r["arc"] < 128]
            secondary.sort(key=lambda row: (row["arc"], row["replicate"]))
            populations["Secondary sampling diagnostics"] = secondary
    else:
        result.append(("report", "Validation proof", populations.pop("Validation proof")))
    for population, name in [("Preservation", "report-preservation"),
                             ("Development cost pilot", "report-development"),
                             ("Validation proof", "report-validation"),
                             ("Secondary sampling diagnostics", "report-sampling")]:
        if population in populations:
            result.append((name, population, populations[population]))
    return result


def label_for(row, population):
    if population == "Secondary sampling diagnostics":
        return "Primary sampled · draw " + str(row["replicate"])
    if row.get("display_label"):
        return row["display_label"] + " · " + row["rule"]
    if not row["seed"]:
        return "Three-second delay" if row["rule"] == "delay" else "Immediate ignition"
    names = {"G": "Greedy", "M": "Median", "S": "Sampled"}
    who = "primary" if population == "Fresh confirmation" and row["seed"] == 281103 else "seed " + str(row["seed"])
    return names[row["rule"]] + " · " + who + (" · draw " + str(row["replicate"]) if row["rule"] == "S" else "")


def render(root):
    root = Path(root).resolve(); campaign = root/"campaign"
    rows = read(campaign/"report-captures.json")
    reports = split_reports(rows)
    links = '<p>Complete captured populations: ' + ' · '.join(
        f'<a href="../{name}/report.html">{html.escape(population)} ({len(subset)} captures)</a>'
        for name,population,subset in reports) + '</p>'
    outputs = []
    for name,population,subset in reports:
        spec_rows = [{"capture":row["capture"], "population":population,
                      "label":label_for(row,population), "context_html":stopping_context(row,campaign)}
                     for row in subset]
        intro = (overview(campaign,rows) if name == "report" else
                 f'<section><h2>{html.escape(population)}</h2><p>{len(subset)} complete captured occurrences with independent physical, controller and planner reconstruction. These are {html.escape(population.lower())} inputs, separate from the fresh denominator.</p></section>')
        spec = {"title":"Coast ignition stopping · " + population,
                "summary":"Frozen weights; unchanged Poly Landing; complete native proof.",
                "rows":spec_rows, "grouped":True, "focus_flat":True,
                "max_bytes":16*2**30, "overview_html":scroll_summary_tables(intro)+links}
        write_new(campaign/(name+"-spec.json"),spec)
        output = campaign/name
        native.render_report(encoded(spec),str(output))
        review = read(output/"report-input.json")
        expected = list(range(len(review["labels"])))
        if any([a["approach"] for a in m["attempts"]] != expected for m in review["missions"]):
            raise ValueError("established reader needs dense, consistently ordered approach slots within each report")
        if sum(len(m["attempts"]) for m in review["missions"]) != len(subset):
            raise ValueError("paired tree inventory differs from capture allocation")
        files = [{"file":str(p.relative_to(output)),"bytes":p.stat().st_size,"sha256":sha(p)}
                 for p in sorted(output.iterdir())]
        receipt = {"schema_version":1,"display_only":True,"native_steps":0,
                   "captured_occurrences":len(subset),"physical_missions":len(review["missions"]),
                   "dense_approach_slots_verified":True,"all_rich_payloads_preserved":True,
                   "presentation_source_sha256":sha(Path(__file__).resolve()),
                   "native_artifact_sha256":sha(__import__('coast_ignite').NATIVE_ARTIFACT),
                   "capture_inventory_sha256":sha(campaign/"report-captures.json"),"files":files}
        write_new(output/"presentation-receipt.json",receipt)
        outputs.append({"report":str(output/"report.html"),"population":population,
                        "missions":len(review["missions"]),"rich_views":len(subset)})
        print(encoded(outputs[-1]),flush=True)
    write_new(campaign/"report-delivery-inventory.json",{"display_only":True,"native_steps":0,"reports":outputs})


if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--root",required=True)
    render(parser.parse_args().root)
