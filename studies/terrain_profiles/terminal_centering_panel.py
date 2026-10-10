"""Body-aware rescue: fixed focus, preservation, then a conditional paired 1k.

Reuse sealed common collectors and rich reports. No tuning/retries or promotion.
"""
import argparse
import json
import re
from pathlib import Path
from types import SimpleNamespace

import ballistic_feedback_sweep as sweep
import coast_terminal_validation as previous
import diagnostics as d
import study
import survey
import terminal_braking_panel as braking
import waypoint_entry_panel as entry

NATIVE = survey.REPO / "target/ballistic-terminal-centering-20261009/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
FOCUS_ROOT = survey.OUTPUTS / "capture-terminal-centering-focus-20261009-v1"
PANEL_ROOT = survey.OUTPUTS / "capture-terminal-centering-preservation-20261009-v1"
PLAN_ROOT = survey.OUTPUTS / "plan-terminal-centering-sweep-20261009-v1"
PREVIOUS_SWEEP = survey.OUTPUTS / "capture-coast-terminal-sweep-20261009-v1"
PROTOCOL = "docs/ballistic_terminal_centering_plan.md"
MODES = sweep.CENTERING_CANDIDATES
FOCUS_SPEC = entry.PanelSpec(PREVIOUS_SWEEP, NATIVE, RENDERER, MODES, "coast-terminal",
    "landing-body-centering", braking.focus_layout, PROTOCOL, sweep.verify, "terminal-centering-focus")
PANEL_SPEC = entry.PanelSpec(previous.PANEL_ROOT, NATIVE, RENDERER, MODES, "coast-terminal",
    "landing-body-centering", previous.layout, PROTOCOL, previous.verify, "terminal-centering-preservation")
SHA_FIELDS = ["candidate_executable_sha256", "candidate_rust_source_sha256", "candidate_renderer_sha256",
              "candidate_report_script_sha256", "experiment_plan_sha256"] + [
    f"gate_{kind}_{name}_sha256" for kind in ("focus", "panel") for name in ("receipt", "results", "manifest")]


def gate(rows, panel=False):
    result = braking.gate(rows, panel)
    if panel and result["passed"]:
        subjects = {r["case_id"]: r["result"] for r in rows if r["cohort"] != "repeat"}
        result["passed"] = (subjects["random-142"]["verified_landing"]
                            and subjects["random-084"]["verified_landing"]
                            and subjects["random-084"]["handoffs"] == 1)
    return result


def verify(root, panel=False):
    spec = PANEL_SPEC if panel else FOCUS_SPEC
    summary = entry.verify(root, spec)
    return {"summary": summary, **gate(d.read(root / "ballistic-feedback-sweep.json")["rows"], panel)}


def run(root, panel=False):
    spec = PANEL_SPEC if panel else FOCUS_SPEC
    if panel:
        if not verify(FOCUS_ROOT)["passed"]:
            raise ValueError("focus does not authorize preservation")
        focus = d.read(FOCUS_ROOT / "manifest.json")
        if (entry.source(spec) != focus["source"]
                or study.digest((survey.REPO / PROTOCOL).read_bytes())
                != d.read(FOCUS_ROOT / "plan.json")["experiment_plan_sha256"]):
            raise ValueError("source/binary/renderer/protocol changed after focus")
    entry.run(root, "landing-body-centering", panel, spec)
    return verify(root, panel)


def fixed_contract():
    old = sweep.contract(sweep.COAST_PLAN)
    return {"schema": "pd-lab.ballistic-feedback-sweep-plan.v1",
            "purpose": "Frozen body-centering candidate after focus/preservation; paired development only",
            "candidate_id": sweep.CENTERING_CANDIDATE, "waypoint_experiment": "landing-body-centering",
            **{key: old[key] for key in ("baseline_capture", "baseline_receipt_sha256", "baseline_results_sha256")},
            "previous_capture": str(PREVIOUS_SWEEP.relative_to(survey.REPO)),
            "previous_receipt_sha256": "3b49d6b57410db740780b408fbb6f43a65adb4b7223f6ce68ccbc17a7f4eb648",
            "previous_results_sha256": "47fd9c9f2b1a078335b1e5d79b482df89f172975fb2ee94462f7e4a14b1425cc",
            "gate_focus_capture": str(FOCUS_ROOT.relative_to(survey.REPO)),
            "gate_panel_capture": str(PANEL_ROOT.relative_to(survey.REPO)),
            "experiment_plan": PROTOCOL, "native_path": str(NATIVE.relative_to(survey.REPO)),
            "renderer_path": str(RENDERER.relative_to(survey.REPO)), "primary_cases": 1000,
            "repeat_indices": [142, 974], "maximum_measured_attempts": 1002, "correction_cap": 24,
            "workers": 4, "case_wall_limit_s": 60, "campaign_wall_limit_s": 7200,
            "retries": 0, "tuning": False, "publication": False}


def sweep_contract(path):
    plan = d.read(path)
    fixed = fixed_contract()
    if (set(plan) != set(fixed) | set(SHA_FIELDS) or any(plan.get(k) != v for k, v in fixed.items())
            or any(not isinstance(plan.get(k), str) or not re.fullmatch("[0-9a-f]{64}", plan[k]) for k in SHA_FIELDS)):
        raise ValueError("changed centering sweep contract")
    return plan


def verify_gate_data(plan, values):
    admitted_source = None
    for kind in ("focus", "panel"):
        receipt, results, manifest = values[kind]
        for data, name in ((receipt, "receipt"), (results, "results"), (manifest, "manifest")):
            if study.digest(data) != plan[f"gate_{kind}_{name}_sha256"]:
                raise ValueError("centering admission seal differs")
        files, report, source = json.loads(receipt)["files"], json.loads(results), json.loads(manifest)["source"]
        identity = "terminal-centering-preservation" if kind == "panel" else "terminal-centering-focus"
        if (files["ballistic-feedback-sweep.json"] != study.digest(results)
                or files["manifest.json"] != study.digest(manifest)
                or files["experiment-plan.md"] != plan["experiment_plan_sha256"]
                or report["schema"] != f"pd-lab.{identity}-panel.v1" or report["stopped_reason"] is not None
                or report["source_after"] != source or not gate(report["rows"], kind == "panel")["passed"]):
            raise ValueError("centering focus/preservation gate failed")
        if admitted_source is not None and source != admitted_source:
            raise ValueError("candidate changed between focus and preservation")
        admitted_source = source
    if any(admitted_source[key] != plan[plan_key] for key, plan_key in (
            ("executable_sha256", "candidate_executable_sha256"),
            ("rust_source_tree_sha256", "candidate_rust_source_sha256"),
            ("renderer_sha256", "candidate_renderer_sha256"))) or (
                admitted_source["files"]["pd-report/src/planning_cycles.js"] != plan["candidate_report_script_sha256"]):
        raise ValueError("sweep identity differs from admitted candidate")
    return admitted_source


def require_sweep_gate(plan, root=None):
    values = {}
    for kind, capture in (("focus", FOCUS_ROOT), ("panel", PANEL_ROOT)):
        if root is None:
            if not verify(capture, kind == "panel")["passed"]:
                raise ValueError("failed focus/preservation does not authorize a sweep")
            values[kind] = [(capture / name).read_bytes() for name in
                            ("receipt.json", "ballistic-feedback-sweep.json", "manifest.json")]
        else:
            values[kind] = [(root / f"gate-{kind}-{name}.json").read_bytes()
                            for name in ("receipt", "results", "manifest")]
    admitted = verify_gate_data(plan, values)
    if root is None and entry.source(PANEL_SPEC) != admitted:
        raise ValueError("candidate changed after preservation")


def run_sweep(root):
    if not verify(FOCUS_ROOT)["passed"] or not verify(PANEL_ROOT, True)["passed"]:
        raise ValueError("focus/preservation does not authorize a sweep")
    seal = d.read(PANEL_ROOT / "manifest.json")["source"]
    plan = {**fixed_contract(), "candidate_executable_sha256": seal["executable_sha256"],
            "candidate_rust_source_sha256": seal["rust_source_tree_sha256"],
            "candidate_renderer_sha256": seal["renderer_sha256"],
            "candidate_report_script_sha256": seal["files"]["pd-report/src/planning_cycles.js"],
            "experiment_plan_sha256": study.digest((PANEL_ROOT / "experiment-plan.md").read_bytes())}
    for kind, capture in (("focus", FOCUS_ROOT), ("panel", PANEL_ROOT)):
        for name, source_name in (("receipt", "receipt.json"), ("results", "ballistic-feedback-sweep.json"), ("manifest", "manifest.json")):
            plan[f"gate_{kind}_{name}_sha256"] = study.digest((capture / source_name).read_bytes())
    require_sweep_gate(plan)
    survey.reserve(PLAN_ROOT)
    path = PLAN_ROOT / "plan.json"
    study.write_new(path, study.encoded(plan))
    sweep.run(SimpleNamespace(plan=path, output=root), contract_reader=sweep_contract, gate_reader=require_sweep_gate)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify", "sweep", "verify-sweep"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--panel", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    if args.action == "run":
        print(run(root, args.panel))
    elif args.action == "verify":
        print(verify(root, args.panel))
    elif args.action == "sweep":
        run_sweep(root)
    else:
        report = sweep.verify(root, contract_reader=sweep_contract, gate_reader=require_sweep_gate)
        print({"stopped_reason": report["stopped_reason"], "summary": report["summary"]})
