"""Display-only interpretation of a completed timing map; never runs a flight."""
from __future__ import annotations

import argparse
import hashlib
import html
import json
import os
import statistics
from pathlib import Path


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def analyze(root):
    root = Path(root); campaign = root / "campaign"
    cases = read(root / "execution-allocation.json")["cases"]
    maps = read(campaign / "map-summary.json")["maps"]
    rows = [json.loads(s) for s in (campaign / "results.jsonl").read_text().splitlines()]
    result = {"display_only": True, "native_steps": 0, "post_hoc_descriptive": True,
              "material_saving_fraction_of_M_consumption": .01,
              "early_contact_threshold_s": -.1, "cohorts": {}, "original_M_misses": []}
    for cohort in ["development", "diagnostic"]:
        details = []
        for case, summary in zip(cases, maps):
            assert case["id"] == summary["id"]
            if case["cohort"] != cohort:
                continue
            group = [r for r in rows if r["id"] == case["id"]]
            by_slot = {r["slot"]: r for r in group}
            if not summary["has_successful_point"]:
                continue
            best = by_slot[summary["best_slot"]]
            interior = [r for r in group if r["native_success"] and r["metrics"]["admitted"] and r["slot"] is not None
                        and all(k in by_slot and by_slot[k]["native_success"] and by_slot[k]["metrics"]["admitted"]
                                for k in [r["slot"] - 1, r["slot"] + 1])]
            interior_best = min(interior, key=lambda r: r["metrics"]["fuel_used_kg"]) if interior else None
            policy = summary["policies"]["M"]
            fuel = policy["fuel_used_kg"]
            eligible = policy["native_success"] and fuel > 0
            details.append({"id": case["id"], "arc": case["arc"], "M_success": policy["native_success"],
                "best_slot": summary["best_slot"], "best_contact_residual_s": best["metrics"]["contact_residual_s"],
                "best_two_sided_success": summary["best_neighbors"]["two_sided_success"],
                "saving_fraction_of_M": (fuel - best["metrics"]["fuel_used_kg"]) / fuel if eligible else None,
                "interior_saving_fraction_of_M": (fuel - interior_best["metrics"]["fuel_used_kg"]) / fuel if eligible and interior_best else None})
            if cohort == "diagnostic" and not policy["native_success"]:
                intervals = summary["successful_powered_components"]
                slot = policy["slot"]
                relation = "no request" if slot is None else "too early" if slot < intervals[0]["first_slot"] else "too late" if slot > intervals[-1]["last_slot"] else "between successful components"
                result["original_M_misses"].append({"id": case["id"], "arc": case["arc"],
                    "M_slot": slot, "successful_components": intervals, "relation": relation,
                    "best_slot": summary["best_slot"]})
        good = [r for r in details if r["saving_fraction_of_M"] is not None]
        material = [r for r in good if r["saving_fraction_of_M"] >= .01]
        result["cohorts"][cohort] = {"arcs": sum(c["cohort"] == cohort for c in cases),
            "arcs_with_successful_grid_point": len(details), "M_successful_arcs_in_cost_comparison": len(good),
            "median_saving_fraction_of_M": statistics.median(r["saving_fraction_of_M"] for r in good),
            "material_savings_arcs": len(material),
            "material_at_timing_edge": sum(not r["best_two_sided_success"] for r in material),
            "material_early_contact": sum(r["best_contact_residual_s"] < -.1 for r in material),
            "median_interior_saving_fraction_of_M": statistics.median(r["interior_saving_fraction_of_M"] for r in good if r["interior_saving_fraction_of_M"] is not None),
            "arcs_with_one_percent_interior_saving": sum(r["interior_saving_fraction_of_M"] is not None and r["interior_saving_fraction_of_M"] >= .01 for r in good),
            "material_details": material}
    return result


def interpretation(findings, cohort, findings_name="timing-findings.json"):
    s = findings["cohorts"][cohort]
    text = ('<section id="timing-interpretation"><h2>What this map establishes</h2>'
        f'<p>The median fuel saving available relative to M consumption is {100*s["median_saving_fraction_of_M"]:.3f}%. '
        f'{s["material_savings_arcs"]} successful M arcs offer at least 1% lower fuel at another tested timing; '
        f'{s["material_at_timing_edge"]} of those minima lack two successful adjacent timing requests and '
        f'{s["material_early_contact"]} contact more than 0.1 s before the supplied reference end.</p>'
        f'<p>Requiring the candidate and both adjacent 0.1 s requests to succeed leaves '
        f'{s["arcs_with_one_percent_interior_saving"]} arcs with at least 1% savings. Median saving under this discrete interior screen: '
        f'{100*s["median_interior_saving_fraction_of_M"]:.3f}%. This screen describes observed timing sensitivity, not robustness to physical perturbations.</p>'
        '<p><strong>Recommended next focus: reliable handoff timing.</strong> Fixed-end fuel savings are small for the typical arc. '
        'Larger savings concentrate at early-contact timing edges. Further fuel training, end-time selection or controller admission changes need their own proposal. '
        'The existing fresh reliability criterion remains unmet; these exposed maps do not change it.</p>')
    if cohort == "diagnostic":
        table = ''
        for r in findings["original_M_misses"]:
            windows = ', '.join(f'{c["first_slot"]/10:.1f}–{c["last_slot"]/10:.1f} s' for c in r["successful_components"])
            table += f'<tr><td>{r["arc"]}</td><td>{r["M_slot"]/10:.1f} s</td><td>{windows}</td><td>{html.escape(r["relation"])}</td></tr>'
        text += ('<p>All seven retained M misses have successful tested handoffs. Six requests precede their observed successful interval; '
                 'the accepted-crash request follows its interval. A universal earlier/later offset is not supported.</p>'
                 '<div class="table-wrap stopping-table-wrap"><table><thead><tr><th>Original arc</th><th>M request</th><th>Successful powered interval</th><th>Observed relation</th></tr></thead><tbody>'
                 + table + '</tbody></table></div>')
    return (text + '<p>Arc numbers identify the retained admitted-input order; rich-flight headers preserve the original proposal IDs. Physical hashes bind each comparison.</p>'
            + f'<p><a href="../{findings_name}">Bound descriptive findings</a> · <a href="../../independent-audit-final.json">Independent read-only audit</a></p></section>')


DISPLAY = """<style>
.timing-study svg{color:#334155}.timing-study svg circle{stroke:#334155;stroke-width:.5}
.timing-study .timing-caption{color:#475569}.timing-study a{color:#0f766e;text-decoration:underline}
.timing-study svg text{fill:#334155}@media(max-width:600px){.timing-study svg text{font-size:22px}.timing-study svg{min-height:150px}}
</style><script>
(()=>{const s=document.getElementById('timing-request'), cases=document.getElementById('timing-case'), strip=document.getElementById('timing-strip');if(!s)return;
const refresh=()=>{s.replaceChildren();for(const b of strip.querySelectorAll('button')){const o=document.createElement('option');o.value=b.dataset.slot;o.textContent=b.title;s.appendChild(o)}const active=strip.querySelector('.selected');if(active)s.value=active.dataset.slot;};
s.addEventListener('change',()=>{[...strip.querySelectorAll('button')].find(b=>b.dataset.slot===s.value)?.click()});cases.addEventListener('change',refresh);
new MutationObserver(()=>{const active=strip.querySelector('.selected');if(active)s.value=active.dataset.slot;}).observe(strip,{subtree:true,attributes:true,attributeFilter:['class']});refresh();})();
</script>"""


def finalize(root, edition="", linked_views=False):
    root = Path(root).resolve(); campaign = root / "campaign"
    assert read(root / "independent-audit-final.json")["passed"]
    if edition and (not edition.isalnum() or len(edition) > 16):
        raise ValueError("edition must be a short alphanumeric label")
    suffix = "-" + edition if edition else ""
    findings_name = "timing-findings" + suffix + ".json"
    findings = analyze(root)
    findings["bindings"] = {name: sha(campaign / name) for name in ["results.jsonl", "map-summary.json", "result.json"]}
    findings["analysis_source_sha256"] = sha(__file__)
    with (campaign / findings_name).open("x") as f:
        json.dump(findings, f, sort_keys=True, indent=2)
    delivery = read(campaign / "report-delivery.json")
    files = []
    original_folders = {r["folder"] for r in delivery["reports"]}
    for report in delivery["reports"]:
        original = Path(report["report"])
        revised = original.read_text()
        if report["folder"] != "report-probes":
            cohort = "development" if report["folder"] == "report" else "diagnostic"
            raw = original.read_text(); anchor = '<section class="timing-study"'
            assert raw.count(anchor) == 1
            revised = raw.replace(anchor, interpretation(findings, cohort, findings_name) + anchor, 1)
            point_anchor = '<div id="timing-plot">'
            assert revised.count(point_anchor) == 1 and revised.count('</body>') == 1
            revised = revised.replace(point_anchor, '<p><label>Inspect request <select id="timing-request" aria-label="Inspect handoff timing"></select></label></p>' + point_anchor, 1)
            revised = revised.replace('</body>', DISPLAY + '</body>', 1)
        if linked_views:
            target_folder = original.parent.with_name(report["folder"] + "-final")
            target_folder.mkdir()
            # Immutable rich payloads are aliased, not regenerated or copied.
            aliases = []
            for source in sorted(original.parent.iterdir()):
                if source.name.startswith("flight-") or source.name in ["report-input.json", "navigation.json"]:
                    target = target_folder / source.name
                    os.link(source, target)
                    assert source.stat().st_ino == target.stat().st_ino
                    aliases.append({"file": source.name, "sha256": sha(target), "source": str(source)})
            for folder in original_folders:
                revised = revised.replace("../" + folder + "/", "../" + folder + "-final/")
            target = target_folder / "report.html"
            with (target_folder / "presentation-receipt.json").open("x") as f:
                json.dump({"display_only": True, "native_steps": 0, "immutable_hardlink_aliases": aliases,
                           "source_presentation_sha256": sha(original.parent / "presentation-receipt.json"),
                           "index_sha256": hashlib.sha256(revised.encode()).hexdigest()}, f, sort_keys=True, indent=2)
            report["folder"] += "-final"
        else:
            target = original.with_name("result" + suffix + ".html")
        with target.open("x") as f: f.write(revised)
        report["report"] = str(target)
        files.append({"original": str(original), "original_sha256": sha(original),
                      "result": str(target), "result_sha256": sha(target)})
    delivery.update({"display_only": True, "native_steps": 0, "indexes_only": True,
                     "finalization_source_sha256": sha(__file__), "files": files})
    with (campaign / ("report-delivery-final" + suffix + ".json")).open("x") as f:
        json.dump(delivery, f, sort_keys=True, indent=2)
    print(json.dumps({"final_indexes": len(files), "native_steps": 0}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("--root", required=True); parser.add_argument("--edition", default="")
    parser.add_argument("--linked-views", action="store_true")
    args = parser.parse_args(); finalize(args.root, args.edition, args.linked_views)
