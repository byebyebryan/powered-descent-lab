"""New outcome-based admission; unchanged V12 binary and common 1002-record collector.

The old exact-control contract stays failed. Never tune or rebuild the candidate.
"""
import argparse
import json
from pathlib import Path
import re
import sys
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "studies/terrain_profiles"))
import ballistic_feedback_sweep as sweep
import diagnostics as d
import study
import survey
import terminal_centering_panel as prior
import waypoint_entry_panel as entry

RUNNER = "scripts/run-terminal-centering-sweep.py"
PROTOCOL = "docs/ballistic_terminal_centering_sweep_plan.md"
ROOT = survey.OUTPUTS / "capture-terminal-centering-sweep-20261009-v1"
PLAN_ROOT = survey.OUTPUTS / "plan-terminal-centering-sweep-20261009-v1"
NATIVE_SHA = "eeb07070b7378186c50f7deba34a0afab96cbd3b1d5b44a32a0311b056418366"
RUST_SHA = "696ae4de5a80abb742a347e610143e9166d9039dc566eebab2cffbdf85cb3dab"
RENDERER_SHA = "1f52999f81e4827614346c445af7b7b3b2a331407c62c59230f9c0c92115a591"
GATE_HASHES = {
    "focus": {
        "receipt": "0182fc9cb329bfac3ccfc9cdedcc298689f249546a53ae0c33a8f7332c6977a3",
        "results": "64c39b0483974bf8827b5908d8041b7d66e0ba7758f52e87bb0148ff5255a4a9"},
    "panel": {
        "receipt": "e66d9a78e7eb1c80fe6d843e4755f53d67369126d1267782bf208c69b3f7783a",
        "results": "8c1575acf5781842655c4e830624f1c1291b5067a9efadf71cf9ec11e72c293b"}}
BASE_SOURCE = sweep.source_state


def fixed_contract():
    value = prior.fixed_contract()
    value.update(purpose="User-authorized unchanged V12 full 1k; outcome-based direct-control admission",
                 admission_id="terminal-centering-outcome-preservation-v1", experiment_plan=PROTOCOL,
                 candidate_executable_sha256=NATIVE_SHA, candidate_rust_source_sha256=RUST_SHA,
                 candidate_renderer_sha256=RENDERER_SHA)
    for kind, values in GATE_HASHES.items():
        value.update({f"gate_{kind}_{name}_sha256": digest for name, digest in values.items()})
    return value


VARIABLE_HASHES = ["experiment_plan_sha256", "candidate_report_script_sha256", "admission_runner_sha256",
                   "prior_protocol_sha256", "gate_focus_manifest_sha256", "gate_panel_manifest_sha256"]


def contract(path):
    value = d.read(path)
    fixed = fixed_contract()
    if (set(value) != set(fixed) | set(VARIABLE_HASHES) or any(value[k] != v for k, v in fixed.items())
            or any(not isinstance(value[k], str) or not re.fullmatch("[0-9a-f]{64}", value[k]) for k in VARIABLE_HASHES)):
        raise ValueError("changed outcome-based 1k contract")
    return value


def control_prefix(current, previous):
    entries = [next((i for i, u in enumerate(record["updates"]) if u["phase"] == "maintained_terminal"), None)
               for record in (current, previous)]
    if entries[0] is None or entries[0] != entries[1]:
        raise ValueError("direct-control terminal entry differs")
    end = entries[0]
    if (current["ordinary_flight"]["actions"][:end] != previous["ordinary_flight"]["actions"][:end]
            or current["ordinary_flight"]["actions"][end]["physics_step"]
            != previous["ordinary_flight"]["actions"][end]["physics_step"]
            or current["refreshes"] != previous["refreshes"] or current["handoffs"] or previous["handoffs"]):
        raise ValueError("direct-control pre-terminal flight differs")


def authenticate_panel():
    root = prior.PANEL_ROOT
    receipt = d.read(root / "receipt.json")["files"]
    if study.digest((root / "receipt.json").read_bytes()) != GATE_HASHES["panel"]["receipt"]:
        raise ValueError("different retained preservation capture")
    if survey.inventory(root) != receipt:
        raise ValueError("preservation inventory differs")
    survey.check_inventory(root, receipt)
    plan, manifest, report = [d.read(root / n) for n in ("plan.json", "manifest.json", "ballistic-feedback-sweep.json")]
    primary, repeats = prior.previous.layout(True)
    if ([r["attempt_id"] for r in report["rows"]] != primary + ["repeat-" + n for n in repeats]
            or report["source_after"] != manifest["source"] or report["protected_after"] != manifest["protected"]
            or report["stopped_reason"] is not None or not prior.gate(report["rows"], True)["passed"]):
        raise ValueError("outcome preservation failed")
    for row in report["rows"]:
        if row["status"] != "recorded" or sweep.check_record(root, row, plan) != row["result"]:
            raise ValueError("retained preservation record proof failed")
        if row["cohort"] == "control":
            control_prefix(d.read(root / row["output_dir"] / "feedback.json"),
                           d.read(root / "previous-feedback" / (row["case_id"] + ".json")))


def gate_data(plan, values):
    admitted = None
    for kind, (receipt_bytes, results_bytes, manifest_bytes) in values.items():
        for data, name in ((receipt_bytes, "receipt"), (results_bytes, "results"), (manifest_bytes, "manifest")):
            if study.digest(data) != plan[f"gate_{kind}_{name}_sha256"]:
                raise ValueError("changed retained admission seal")
        files, report, manifest = json.loads(receipt_bytes)["files"], json.loads(results_bytes), json.loads(manifest_bytes)
        source = manifest["source"]
        identity = "terminal-centering-preservation" if kind == "panel" else "terminal-centering-focus"
        if (files["ballistic-feedback-sweep.json"] != study.digest(results_bytes)
                or files["manifest.json"] != study.digest(manifest_bytes)
                or files["experiment-plan.md"] != plan["prior_protocol_sha256"]
                or report["schema"] != f"pd-lab.{identity}-panel.v1" or report["stopped_reason"] is not None
                or report["source_after"] != source or report["protected_after"] != manifest["protected"]
                or not prior.gate(report["rows"], kind == "panel")["passed"]):
            raise ValueError("retained outcome/proof admission failed")
        if admitted is not None and admitted != source:
            raise ValueError("candidate changed between retained stages")
        admitted = source
    if set(values) != {"focus", "panel"} or any(admitted[key] != plan[plan_key] for key, plan_key in (
            ("executable_sha256", "candidate_executable_sha256"), ("rust_source_tree_sha256", "candidate_rust_source_sha256"),
            ("renderer_sha256", "candidate_renderer_sha256"))) or (
                admitted["files"]["pd-report/src/planning_cycles.js"] != plan["candidate_report_script_sha256"]):
        raise ValueError("different admitted native/source/renderer")
    return admitted


def require_gate(plan, root=None):
    values = {}
    for kind, capture in (("focus", prior.FOCUS_ROOT), ("panel", prior.PANEL_ROOT)):
        values[kind] = [(capture / name).read_bytes() if root is None else (root / f"gate-{kind}-{label}.json").read_bytes()
                        for name, label in (("receipt.json", "receipt"), ("ballistic-feedback-sweep.json", "results"),
                                            ("manifest.json", "manifest"))]
    admitted = gate_data(plan, values)
    if root is None:
        if not prior.verify(prior.FOCUS_ROOT)["passed"]:
            raise ValueError("focus failed")
        authenticate_panel()
        current = BASE_SOURCE(prior.NATIVE)
        current.update(renderer_sha256=study.digest(prior.RENDERER.read_bytes()),
                       rust_source_tree_sha256=prior.entry.source(prior.PANEL_SPEC)["rust_source_tree_sha256"])
        if (current != admitted or entry.protected(prior.PANEL_SPEC) != d.read(prior.PANEL_ROOT / "manifest.json")["protected"]
                or study.digest((REPO / RUNNER).read_bytes()) != plan["admission_runner_sha256"]):
            raise ValueError("frozen candidate/protected state/runner changed")
    else:
        seal = d.read(root / "candidate/source-seal.json")
        expected_files = {**admitted["files"], RUNNER: plan["admission_runner_sha256"]}
        if (seal["files"] != expected_files or seal["git_commit"] != admitted["git_commit"]
                or study.digest((root / "candidate/source" / RUNNER).read_bytes()) != plan["admission_runner_sha256"]):
            raise ValueError("new capture changed flight sources or omitted admission runner")


def collection_source(binary):
    """Archive this separate driver without modifying frozen common source files."""
    value = BASE_SOURCE(binary)
    value["files"][RUNNER] = study.digest((REPO / RUNNER).read_bytes())
    return value


def run():
    focus, panel = prior.FOCUS_ROOT, prior.PANEL_ROOT
    source = d.read(panel / "manifest.json")["source"]
    plan = {**fixed_contract(), "experiment_plan_sha256": study.digest((REPO / PROTOCOL).read_bytes()),
            "admission_runner_sha256": study.digest((REPO / RUNNER).read_bytes()),
            "prior_protocol_sha256": study.digest((focus / "experiment-plan.md").read_bytes()),
            "candidate_report_script_sha256": source["files"]["pd-report/src/planning_cycles.js"],
            "gate_focus_manifest_sha256": study.digest((focus / "manifest.json").read_bytes()),
            "gate_panel_manifest_sha256": study.digest((panel / "manifest.json").read_bytes())}
    require_gate(plan)
    if ROOT.exists():
        raise ValueError("sweep destination already exists; never overwrite or retry")
    survey.reserve(PLAN_ROOT)
    path = PLAN_ROOT / "plan.json"
    study.write_new(path, study.encoded(plan))
    sweep.source_state = collection_source
    try:
        sweep.run(SimpleNamespace(plan=path, output=ROOT), contract_reader=contract, gate_reader=require_gate)
    finally:
        sweep.source_state = BASE_SOURCE


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("run", "verify"))
    args = parser.parse_args()
    if args.action == "run":
        run()
    else:
        result = sweep.verify(ROOT, contract_reader=contract, gate_reader=require_gate)
        print(json.dumps({"stopped_reason": result["stopped_reason"], "summary": result["summary"]}))
