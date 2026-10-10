"""Bounded 807/928 waypoint-clearance panel; no tuning, publication or full sweep.

Uses original receipt-bound worlds, the common native command/repeat proof reader,
and the maintained common batch/detail templates. Outputs are create-only.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import subprocess
from pathlib import Path

import ballistic_feedback_sweep as sweep
import diagnostics as d
import study
import survey

INDICES = [807, 928, 484, 34, 55, 715, 0, 1, 6, 84, 114, 139, 142, 268, 308, 349]
CONTROLS = ["v2_clear_845", "fresh_clear_uphill_805", "fresh_clear_downhill_805"]
REPEATS = [807, 928]
NATIVE = survey.REPO / "target/ballistic-waypoint-clearance-20261008/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
PREVIOUS = "outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1"


def comparison_plan():
    return dict(sweep.contract(sweep.TERRAIN_PLAN), previous_capture=PREVIOUS,
                previous_receipt_sha256="20c474ff79bf722d9bbd6a0e46fab5fbd867f510c88ac7b593df08cd39110afe",
                previous_results_sha256="9f39ff37b57767b0d2efa6af3f05da63882be8c68a4515942c16adcfb9c15621")


def verify(root):
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("panel inventory differs")
    survey.check_inventory(root, receipt)
    plan, manifest, results = (d.read(root / name) for name in
                              ("plan.json", "manifest.json", "ballistic-feedback-sweep.json"))
    if (plan["candidate_id"] != sweep.WAYPOINT_CANDIDATE or plan["maximum_measured_attempts"] != 21
            or len(results["rows"]) != 21 or results["source_after"] != manifest["source"]
            or results["protected_after"] != manifest["protected"] or results["stopped_reason"] is not None):
        raise ValueError("panel source, bound or collection differs")
    for row in results["rows"]:
        if row["status"] != "recorded" or sweep.check_record(root, row, plan) != row["result"]:
            raise ValueError("panel native proof differs")
    if sweep.summary(results["rows"]) != results["summary"]:
        raise ValueError("panel summary differs")
    return results["summary"]


def run(root):
    baseline_plan = comparison_plan()
    cases, baseline_receipt, bound = sweep.baseline(baseline_plan)
    selected = [dict(cases[i]) for i in INDICES]
    inputs = {row["case_id"]: bound(row["scenario_path"]) for row in selected}
    accepted = survey.REPO / "outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native"
    subprocess.run([str(survey.REPO / "target/release/pd-eval"), "check-planner-v2", "--dir", str(accepted)],
                   capture_output=True, check=True, timeout=60)
    expanded = {row["case_id"]: row["scenario"] for row in d.read(accepted / "expanded-inputs.json")}
    control_source = {"capture": str(accepted.relative_to(survey.REPO)),
                      "expanded_inputs_sha256": study.digest((accepted / "expanded-inputs.json").read_bytes()),
                      "summary_sha256": study.digest((accepted / "summary.json").read_bytes()),
                      "native_saved_check_passed": True}
    for case_id in CONTROLS:
        relative = f"runs/{case_id}/scenario.json"
        data = d.safe_file(accepted, relative).read_bytes()
        if d.read(accepted / relative) != expanded[case_id]:
            raise ValueError("changed control input")
        inputs[case_id] = data
        selected.append({"case_id": case_id, "attempt_id": case_id, "cohort": "control",
                         "scenario_sha256": study.digest(data), "status": "not_attempted"})
    for row in selected:
        row["scenario_path"] = f'scenarios/{row["case_id"]}.json'
    rows = selected + [dict(next(row for row in selected if row["case_id"] == f"random-{i:03}"),
                            attempt_id=f"repeat-random-{i:03}", cohort="repeat") for i in REPEATS]
    source = sweep.source_state(NATIVE)
    source["renderer_sha256"] = study.digest(RENDERER.read_bytes())
    native_seal = d.read(survey.REPO / "outputs/eval/planner_v2_random_terrain/dev-waypoint-clearance-20261008-r2-807/attempt.json")
    if native_seal["binary_sha256"] != source["executable_sha256"]:
        raise ValueError("panel binary differs from reviewed development build")
    plan = {"schema": "pd-lab.waypoint-clearance-panel-plan.v1", "candidate_id": sweep.WAYPOINT_CANDIDATE,
            "correction_cap": 24, "candidate_executable_sha256": source["executable_sha256"],
            "candidate_rust_source_sha256": native_seal["content_source"]["rust_source_tree_sha256"],
            "candidate_report_script_sha256": source["files"]["pd-report/src/planning_cycles.js"],
            "case_wall_limit_s": 60, "maximum_measured_attempts": 21,
            "indices": INDICES, "controls": CONTROLS, "repeats": REPEATS,
            "retries": 0, "tuning": False, "publication": False}
    protected = sweep.protected_state(baseline_plan)
    survey.reserve(root)
    study.write_new(root / "plan.json", study.encoded(plan))
    study.write_new(root / "manifest.json", study.encoded({"rows": rows, "source": source, "protected": protected}))
    study.write_new(root / "baseline-receipt.json", baseline_receipt)
    study.write_new(root / "control-source.json", study.encoded(control_source))
    study.write_new(root / "control-source-expanded-inputs.json", (accepted / "expanded-inputs.json").read_bytes())
    study.write_new(root / "control-source-summary.json", (accepted / "summary.json").read_bytes())
    study.write_new(root / "candidate/bin/pd-eval", NATIVE.read_bytes())
    (root / "candidate/bin/pd-eval").chmod(0o755)
    for name in ("receipt.json", "ballistic-feedback-sweep.json"):
        study.write_new(root / ("previous-" + name), (survey.REPO / PREVIOUS / name).read_bytes())
    for case_id, data in inputs.items():
        study.write_new(root / f"scenarios/{case_id}.json", data)
    with ThreadPoolExecutor(max_workers=4) as pool:
        actual = list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[:len(selected)]))
        actual += list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[len(selected):]))
    after = sweep.source_state(NATIVE)
    after["renderer_sha256"] = study.digest(RENDERER.read_bytes())
    protected_after = sweep.protected_state(baseline_plan)
    if after != source or protected_after != protected:
        raise ValueError("panel source/protected drift; outputs retained")
    results = {"schema": "pd-lab.waypoint-clearance-panel.v1", "rows": actual,
               "source_after": after, "protected_after": protected_after,
               "stopped_reason": None if all(row["status"] == "recorded" for row in actual) else "evidence_error",
               "summary": sweep.summary(actual)}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(results))
    subprocess.run([str(RENDERER), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    verify(root)
    for row in actual:
        print(row["attempt_id"], row["status"], row["result"], flush=True)
    return results


def finish(root):
    """Finalize already collected flights after the display-only control-ID fix."""
    if (root / "index.html").exists() or (root / "receipt.json").exists():
        raise ValueError("panel is already rendered or finalized")
    manifest, results, plan = (d.read(root / name) for name in
                              ("manifest.json", "ballistic-feedback-sweep.json", "plan.json"))
    if results["stopped_reason"] is not None or len(results["rows"]) != 21:
        raise ValueError("cannot finish incomplete flight collection")
    for row in results["rows"]:
        if row["status"] != "recorded" or sweep.check_record(root, row, plan) != row["result"]:
            raise ValueError("saved native record differs")
    after = sweep.source_state(NATIVE)
    after["renderer_sha256"] = study.digest(RENDERER.read_bytes())
    before = manifest["source"]
    changed = {name for name in set(after["files"]) | set(before["files"])
               if after["files"].get(name) != before["files"].get(name)}
    allowed = {"pd-eval/examples/ballistic_feedback_batch_report.rs",
               "studies/terrain_profiles/waypoint_clearance_panel.py",
               "studies/terrain_profiles/test_ballistic_feedback_sweep.py"}
    if (changed != allowed or after["executable_sha256"] != before["executable_sha256"]
            or after["git_commit"] != before["git_commit"]
            or sweep.protected_state(comparison_plan()) != manifest["protected"]):
        raise ValueError("native/protected drift beyond display-only repair")
    study.write_new(root / "renderer-repair.json", study.encoded({
        "reason": "Common batch rejected safe underscore IDs of retained control missions",
        "flight_reruns": 0, "native_source_changed": False,
        "source_before": before, "source_after": after,
        "changed_files": sorted(changed), "failed_renderer_invocations": 2}))
    study.write_new(root / "renderer-finalizer.py", Path(__file__).read_bytes())
    study.write_new(root / "candidate/bin/ballistic_feedback_batch_report", RENDERER.read_bytes())
    subprocess.run([str(RENDERER), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    return verify(root)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify", "finish"])
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    action = {"run": run, "verify": verify, "finish": finish}[args.action]
    result = action(args.root.resolve())
    if args.action != "run":
        print(result)
