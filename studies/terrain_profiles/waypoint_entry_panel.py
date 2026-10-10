"""Create-only waypoint-entry ablations and preservation panel on frozen V5 worlds.

Four 8-record ablations plus one 21-record combined panel. No flight-driven
tuning, retry, publication or default changes; common rich reports are retained.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
import subprocess

import ballistic_feedback_sweep as sweep
import diagnostics as d
import ridge_waypoint_panel as ridge
import study
import survey
import waypoint_clearance_panel as previous

PREVIOUS = survey.OUTPUTS / "capture-ridge-waypoint-panel-20261008-v1"
NATIVE = survey.REPO / "target/ballistic-waypoint-entry-20261008/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
MODES = {"ridge": sweep.RIDGE_CANDIDATE, **sweep.ENTRY_CANDIDATES}
FOCUS = [349, 6, 807, 928]


def layout(panel=False):
    primary = [f"random-{i:03}" for i in (previous.INDICES if panel else FOCUS)] + previous.CONTROLS
    repeats = [f"random-{i:03}" for i in ([349, 807] if panel else [349])]
    return primary, repeats


@dataclass(frozen=True)
class PanelSpec:
    previous: Path
    native: Path
    renderer: Path
    modes: dict
    control_mode: str
    panel_mode: str
    layout: object
    protocol: str
    previous_verify: object
    identity: str


SPEC = PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "ridge", "combined", layout,
                 "docs/ballistic_waypoint_entry_plan.md", ridge.verify, "waypoint-entry")


def source(spec=SPEC):
    value = sweep.source_state(spec.native)
    value.update(renderer_sha256=study.digest(spec.renderer.read_bytes()),
                 rust_source_tree_sha256=ridge.rust_digest())
    return value


def protected(spec=SPEC):
    value = sweep.protected_state(previous.comparison_plan())
    value.update({str((spec.previous / name).relative_to(survey.REPO)): study.digest((spec.previous / name).read_bytes())
                  for name in ("receipt.json", "ballistic-feedback-sweep.json", "index.html")})
    return value


def verify(root, spec=SPEC):
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("waypoint entry inventory differs")
    survey.check_inventory(root, receipt)
    plan, manifest, report = (d.read(root / name) for name in
                             ("plan.json", "manifest.json", "ballistic-feedback-sweep.json"))
    mode, panel = plan["waypoint_experiment"], plan["preservation_panel"]
    primary, repeats = spec.layout(panel)
    expected = primary + [f"repeat-{name}" for name in repeats]
    if (mode not in spec.modes or plan["candidate_id"] != spec.modes[mode] or (panel and mode != spec.panel_mode)
            or plan["schema"] != f"pd-lab.{spec.identity}-panel-plan.v1"
            or report["schema"] != f"pd-lab.{spec.identity}-panel.v1"
            or plan["maximum_measured_attempts"] != len(expected) or plan["retries"] != 0
            or plan["tuning"] or plan["publication"]
            or [r["attempt_id"] for r in report["rows"]] != expected
            or report["source_after"] != manifest["source"] or report["protected_after"] != manifest["protected"]
            or report["stopped_reason"] is not None):
        raise ValueError("waypoint entry contract/source/denominator differs")
    old_receipt = d.read(root / "previous-receipt.json")["files"]
    old_bytes = (root / "previous-results.json").read_bytes()
    if (study.digest((root / "previous-receipt.json").read_bytes()) != plan["previous_receipt_sha256"]
            or study.digest(old_bytes) != old_receipt["ballistic-feedback-sweep.json"]
            or study.digest(old_bytes) != plan["previous_results_sha256"]
            or study.digest((root / "experiment-plan.md").read_bytes()) != plan["experiment_plan_sha256"]):
        raise ValueError("portable V5 comparison/experiment plan differs")
    old = {r["case_id"]: r for r in d.read(root / "previous-results.json")["rows"] if r["cohort"] != "repeat"}
    if len(manifest["rows"]) != len(report["rows"]):
        raise ValueError("attempt inventory length differs")
    for row, frozen in zip(report["rows"], manifest["rows"]):
        if any(row[k] != v for k, v in frozen.items() if k != "status"):
            raise ValueError("changed frozen entry attempt")
        if (row["previous_result"] != old[row["case_id"]]["result"]
                or row["scenario_sha256"] != old_receipt[row["scenario_path"]]
                or row["status"] != "recorded" or sweep.check_record(root, row, plan) != row["result"]):
            raise ValueError("entry native/paired input proof differs")
        current = d.read(root / row["output_dir"] / "feedback.json")
        if row["cohort"] == "repeat":
            original = next(r for r in report["rows"] if r["attempt_id"] == row["case_id"])
            if current != d.read(root / original["output_dir"] / "feedback.json"):
                raise ValueError("complete entry repeat differs")
        saved = root / "previous-feedback" / (row["case_id"] + ".json")
        if study.digest(saved.read_bytes()) != old_receipt[f'runs/{row["case_id"]}/feedback.json']:
            raise ValueError("previous complete feedback binding differs")
        baseline = d.read(saved)
        if (mode == spec.control_mode and current != baseline) or (row["cohort"] == "control" and current["ordinary_flight"] != baseline["ordinary_flight"]):
            raise ValueError("ridge baseline or direct-control flight changed")
    seal = manifest["source"]
    for name, key in (("pd-eval", "executable_sha256"), ("batch-report", "renderer_sha256")):
        if study.digest((root / "candidate/bin" / name).read_bytes()) != seal[key]:
            raise ValueError("saved entry binary differs")
    for name, digest in seal["files"].items():
        if study.digest(d.safe_file(root / "candidate/source", name).read_bytes()) != digest:
            raise ValueError("saved entry source differs")
    if ridge.metrics(report["rows"]) != report["summary"]:
        raise ValueError("waypoint entry summary differs")
    return report["summary"]


def run(root, mode, panel=False, spec=SPEC):
    if mode not in spec.modes or (panel and mode != spec.panel_mode):
        raise ValueError("unknown mode or wrong preservation candidate")
    spec.previous_verify(spec.previous)
    old = d.read(spec.previous / "ballistic-feedback-sweep.json")
    primary, repeats = spec.layout(panel)
    rows = []
    for case_id in primary:
        prior = next(r for r in old["rows"] if r["attempt_id"] == case_id)
        rows.append({k: v for k, v in prior.items() if k not in
                     ("output_dir", "exit_code", "failure", "result", "wall_s")})
        rows[-1].update(status="not_attempted", previous_result=prior["result"])
    primary_count = len(rows)
    rows += [dict(next(r for r in rows if r["case_id"] == name),
                  attempt_id=f"repeat-{name}", cohort="repeat") for name in repeats]
    seal, safe = source(spec), protected(spec)
    protocol = (survey.REPO / spec.protocol).read_bytes()
    plan = {"schema": f"pd-lab.{spec.identity}-panel-plan.v1", "waypoint_experiment": mode,
            "preservation_panel": panel, "candidate_id": spec.modes[mode], "correction_cap": 24,
            "candidate_executable_sha256": seal["executable_sha256"],
            "candidate_rust_source_sha256": seal["rust_source_tree_sha256"],
            "candidate_report_script_sha256": seal["files"]["pd-report/src/planning_cycles.js"],
            "case_wall_limit_s": 60, "maximum_measured_attempts": len(rows),
            "retries": 0, "tuning": False, "publication": False,
            "experiment_plan_sha256": study.digest(protocol),
            "previous_receipt_sha256": study.digest((spec.previous / "receipt.json").read_bytes()),
            "previous_results_sha256": study.digest((spec.previous / "ballistic-feedback-sweep.json").read_bytes())}
    survey.reserve(root)
    study.write_new(root / "plan.json", study.encoded(plan))
    study.write_new(root / "manifest.json", study.encoded({"rows": rows, "source": seal, "protected": safe}))
    study.write_new(root / "experiment-plan.md", protocol)
    for name in ("receipt.json", "ballistic-feedback-sweep.json"):
        study.write_new(root / ("previous-receipt.json" if name == "receipt.json" else "previous-results.json"), (spec.previous / name).read_bytes())
    for name, digest in seal["files"].items():
        data = (survey.REPO / name).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed during entry snapshot")
        study.write_new(root / "candidate/source" / name, data)
    for name, binary in (("pd-eval", spec.native), ("batch-report", spec.renderer)):
        study.write_new(root / "candidate/bin" / name, binary.read_bytes())
        (root / "candidate/bin" / name).chmod(0o755)
    for row in rows[:primary_count]:
        study.write_new(root / row["scenario_path"], d.safe_file(spec.previous, row["scenario_path"]).read_bytes())
        study.write_new(root / "previous-feedback" / (row["case_id"] + ".json"), (spec.previous / "runs" / row["case_id"] / "feedback.json").read_bytes())
    with ThreadPoolExecutor(max_workers=4) as pool:
        actual = list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[:primary_count]))
        actual += list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[primary_count:]))
    after, protected_after = source(spec), protected(spec)
    report = {"schema": f"pd-lab.{spec.identity}-panel.v1", "rows": actual,
              "source_after": after, "protected_after": protected_after,
              "stopped_reason": None if all(r["status"] == "recorded" for r in actual)
              and after == seal and protected_after == safe else "evidence_error",
              "summary": ridge.metrics(actual)}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    result = verify(root, spec)
    for row in actual:
        print(row["attempt_id"], row["status"], row["result"], flush=True)
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--mode", choices=list(MODES), default="combined")
    parser.add_argument("--panel", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    print(verify(root) if args.action == "verify" else run(root, args.mode, args.panel))
