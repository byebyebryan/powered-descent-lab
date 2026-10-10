"""Unchanged coast-terminal full preservation panel and conditional sweep gate.

The earlier six-world runner remains frozen. No native rebuild, tuning or retry.
"""
import argparse
import json
from pathlib import Path

import ballistic_feedback_sweep as sweep
import coast_terminal_panel as focused
import diagnostics as d
import study
import survey
import waypoint_clearance_panel as previous
import waypoint_countdown_panel as countdown
import waypoint_entry_panel as entry

WITNESS = survey.OUTPUTS / "capture-coast-terminal-focus-20261009-v1"
NATIVE_SHA = "24684011dc502fed49b1aa314d00f52fa2dc4ed9fd1ff09dbf2be01420081d82"
RUST_SHA = "8225fe78691aedc1b797171d997097c90107c4c222509945a79e1f636a6b2231"
RENDERER_SHA = "f9be324b8e4029f45bd3fde1400c868ebc44e3801740ca45ae0a60d3285b83fa"
PANEL_ROOT = survey.OUTPUTS / "capture-coast-terminal-preservation-20261009-v1"


def layout(panel=False):
    if not panel:
        raise ValueError("validation requires the full frozen preservation panel")
    return [f"random-{i:03}" for i in previous.INDICES] + previous.CONTROLS, ["random-084", "random-715"]


SPEC = entry.PanelSpec(focused.PREVIOUS, focused.NATIVE, focused.RENDERER,
                       {"coast-terminal": sweep.COAST_CANDIDATE}, "landing-countdown", "coast-terminal",
                       layout, "docs/ballistic_coast_terminal_validation_plan.md", countdown.verify,
                       "coast-terminal-preservation")


def require_identity(source):
    if (source["executable_sha256"] != NATIVE_SHA
            or source["rust_source_tree_sha256"] != RUST_SHA
            or source["renderer_sha256"] != RENDERER_SHA):
        raise ValueError("native/Rust/renderer differs from the admitted focused candidate")


def gate(rows):
    primary = [r for r in rows if r["cohort"] == "random"]
    if len(primary) != 16 or any(r["status"] != "recorded" for r in rows):
        raise ValueError("incomplete preservation panel")
    lost = [r["case_id"] for r in primary if r["previous_result"]["verified_landing"]
            and not r["result"]["verified_landing"]]
    abnormal = [r["attempt_id"] for r in rows if r["result"]["physical_outcome"]
                not in ("flying", "landed_on_target")]
    subject = next(r for r in primary if r["case_id"] == "random-084")["result"]
    controls = [r for r in rows if r["cohort"] == "control"]
    passed = (not lost and not abnormal and subject["verified_landing"] and subject["handoffs"] == 1
              and len(controls) == 3 and all(r["result"]["verified_landing"] for r in controls))
    return {"sweep_gate_passed": passed, "lost_v9_landings": lost, "abnormal_outcomes": abnormal}


def verify(root):
    summary = entry.verify(root, SPEC)
    require_identity(d.read(root / "manifest.json")["source"])
    return {"summary": summary, **gate(d.read(root / "ballistic-feedback-sweep.json")["rows"])}


def run(root):
    focused.verify(WITNESS)
    current = entry.source(SPEC)
    require_identity(current)
    admitted = d.read(WITNESS / "manifest.json")["source"]
    if current["files"]["pd-report/src/planning_cycles.js"] != admitted["files"]["pd-report/src/planning_cycles.js"]:
        raise ValueError("planning report script differs from focused candidate")
    entry.run(root, "coast-terminal", True, SPEC)
    return verify(root)


def verify_gate_data(plan, receipt, results, manifest):
    for data, key in [(receipt, "gate_panel_receipt_sha256"),
                      (results, "gate_panel_results_sha256"), (manifest, "gate_panel_manifest_sha256")]:
        if study.digest(data) != plan[key]:
            raise ValueError("conditional sweep gate seal differs")
    files, report, source = json.loads(receipt)["files"], json.loads(results), json.loads(manifest)["source"]
    if (files["ballistic-feedback-sweep.json"] != study.digest(results)
            or files["manifest.json"] != study.digest(manifest)
            or report["schema"] != "pd-lab.coast-terminal-preservation-panel.v1"
            or report["stopped_reason"] is not None or report["source_after"] != source):
        raise ValueError("conditional sweep panel/source binding differs")
    require_identity(source)
    if not gate(report["rows"])["sweep_gate_passed"]:
        raise ValueError("preservation panel does not authorize the sweep")


def require_sweep_gate(plan, root=None):
    panel = survey.REPO / plan["gate_panel_capture"]
    if root is None:
        if not verify(panel)["sweep_gate_passed"]:
            raise ValueError("preservation panel failed; no sweep authorized")
        values = [(panel / name).read_bytes() for name in
                  ("receipt.json", "ballistic-feedback-sweep.json", "manifest.json")]
    else:
        values = [(root / name).read_bytes() for name in
                  ("gate-panel-receipt.json", "gate-panel-results.json", "gate-panel-manifest.json")]
    verify_gate_data(plan, *values)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    print(run(args.root.resolve()) if args.action == "run" else verify(args.root.resolve()))
