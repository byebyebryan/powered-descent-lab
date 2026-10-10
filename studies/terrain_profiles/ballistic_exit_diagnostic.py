"""Separately authorized, frozen V14 exit-check diagnostic; no tuning or promotion."""
import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import json
import subprocess
import time

import ballistic_mechanics_panel as panel
import ballistic_mechanics_review as review
import diagnostics as d
import ridge_waypoint_panel as ridge
import study
import survey
import waypoint_entry_panel as entry

MODE = "exit-consistency"
ROOT = survey.OUTPUTS / "capture-ballistic-exit-diagnostic-1k-20261009-v1"
FOCUS = review.root_for(MODE)
FOCUS_SEAL = ("f322767d5adf3701badcbae8af4a940d3e8b086c3636f5c26199d3d2c6190f83",
              "2e6a893d438a3d6268270b9dbb19ee657da9d8900fdf919e40c5c78c6f63af94")
IDENTITY = {"executable_sha256": "06b1d487cb6b02be4182b4a97fba8ad769775bf971d796b9717038aea5a7ca90",
            "rust_source_tree_sha256": "7c4dacf7e87887e54ce9d0f407a0fbcb536330de616054387ff271cb5a84db97",
            "renderer_sha256": "e8ff05d3b61310f2f0cfaf8413263b945d3b629050d3f9a9288b8516d74af0c0",
            "report_script_sha256": "9c7129d5d9aa5b4efa06e82c6ced7c9a72616be2474d859adc030302d63ec37d"}
REPEATS = [47, 142, 715, 44, 20]
ADMISSION = {"passed": False, "gains": ["random-035", "random-047", "random-094"],
             "losses": ["random-142", "random-715"], "subject_gains": ["random-047"]}
METADATA = {"stage": "diagnostic-sweep", "user_authorized_diagnostic": True,
            "primary_cases": 1000, "workers": 4, "campaign_wall_limit_s": 7200,
            "panel_receipt_sha256": FOCUS_SEAL[0], "panel_results_sha256": FOCUS_SEAL[1],
            "panel_admission": ADMISSION}


def layout(panel_stage=False):
    if panel_stage:
        raise ValueError("diagnostic is not the failed conditional stage")
    return ([f"random-{i:03}" for i in range(1000)], [f"random-{i:03}" for i in REPEATS])


def flight_identity(source):
    return {key: source[key] for key in IDENTITY if key != "report_script_sha256"} | {
        "report_script_sha256": source["files"]["pd-report/src/planning_cycles.js"]}


def check_frozen(source, focus_report):
    if flight_identity(source) != IDENTITY or flight_identity(focus_report["source_after"]) != IDENTITY:
        raise ValueError("unchanged exit candidate drifted")
    if panel.admission(focus_report, MODE) != ADMISSION or focus_report["stopped_reason"] is not None:
        raise ValueError("failed panel verdict changed or evidence incomplete")


def previous_verify(root):
    panel.previous_verify(root)
    if tuple(study.digest((FOCUS / name).read_bytes()) for name in
             ("receipt.json", "ballistic-feedback-sweep.json")) != FOCUS_SEAL:
        raise ValueError("different exit-panel authorization reference")
    review.verify(FOCUS)
    check_frozen(entry.source(SPEC), d.read(FOCUS / "ballistic-feedback-sweep.json"))


SPEC = entry.PanelSpec(panel.PREVIOUS, panel.NATIVE, panel.RENDERER,
                       {MODE: panel.MODES[MODE]}, "terminal-coordination", MODE, layout,
                       "docs/ballistic_exit_diagnostic_plan.md", previous_verify,
                       "ballistic-exit-diagnostic")


def protected():
    value = entry.protected(SPEC)
    for root in (panel.capture(MODE), FOCUS):
        for name in ("receipt.json", "ballistic-feedback-sweep.json", "index.html"):
            path = root / name
            value[str(path.relative_to(survey.REPO))] = study.digest(path.read_bytes())
    return value


def verify(root=ROOT):
    summary = entry.verify(root, SPEC)
    plan, manifest, report = [d.read(root / name) for name in
                             ("plan.json", "manifest.json", "ballistic-feedback-sweep.json")]
    if (any(plan.get(k) != v for k, v in METADATA.items()) or plan["correction_cap"] != 24
            or plan["case_wall_limit_s"] != 60 or plan["maximum_measured_attempts"] != 1005
            or (plan["previous_receipt_sha256"], plan["previous_results_sha256"]) != panel.SEAL):
        raise ValueError("diagnostic authorization/limits/reference changed")
    if tuple(study.digest((root / ("focus-" + name)).read_bytes()) for name in
             ("receipt.json", "ballistic-feedback-sweep.json")) != FOCUS_SEAL:
        raise ValueError("portable exit-panel reference changed")
    focus = d.read(root / "focus-ballistic-feedback-sweep.json")
    check_frozen(manifest["source"], focus)
    files = d.read(root / "focus-receipt.json")["files"]
    if study.digest((root / "focus-ballistic-feedback-sweep.json").read_bytes()) != files["ballistic-feedback-sweep.json"]:
        raise ValueError("focus result is not receipt-bound")
    for row in report["rows"]:
        current = root / row["output_dir"] / "feedback.json"
        focus_path = f'runs/{row["case_id"]}/feedback.json'
        if focus_path in files and study.digest(current.read_bytes()) != files[focus_path]:
            raise ValueError("complete overlapping exit flight changed")
        if int(row["case_id"].split("-")[1]) in panel.DIRECT:
            prior = d.read(root / "previous-feedback" / (row["case_id"] + ".json"))
            if d.read(current)["ordinary_flight"] != prior["ordinary_flight"]:
                raise ValueError("zero-H direct control changed")
    return summary


def measure(root, rows, plan):
    """Bound scheduling; honest finite stops/regressions do not close the batch."""
    deadline = time.monotonic() + plan["campaign_wall_limit_s"]
    actual = []
    iterator = iter(rows)
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        pending = {pool.submit(panel.sweep.attempt, root, row, plan): row
                   for row in [next(iterator) for _ in range(plan["workers"])]}
        while pending:
            done, _ = wait(pending, timeout=5, return_when=FIRST_COMPLETED)
            for future in done:
                pending.pop(future)
                actual.append(future.result())
                if len(actual) % 50 == 0:
                    print(json.dumps({"completed": len(actual), "of": len(rows)}), flush=True)
                if time.monotonic() < deadline:
                    row = next(iterator, None)
                    if row is not None:
                        pending[pool.submit(panel.sweep.attempt, root, row, plan)] = row
    lookup = {row["attempt_id"]: row for row in actual}
    return [lookup.get(row["attempt_id"], row) for row in rows]


def run():
    previous_verify(SPEC.previous)
    old = d.read(SPEC.previous / "ballistic-feedback-sweep.json")
    prior = {row["attempt_id"]: row for row in old["rows"]}
    primary, repeats = layout()
    rows = []
    for case_id in primary:
        row = {k: v for k, v in prior[case_id].items() if k not in
               ("output_dir", "exit_code", "failure", "result", "wall_s")}
        rows.append(dict(row, status="not_attempted", previous_result=prior[case_id]["result"]))
    rows += [dict(rows[int(name.split("-")[1])], attempt_id="repeat-" + name, cohort="repeat") for name in repeats]
    source, safe = entry.source(SPEC), protected()
    protocol = (survey.REPO / SPEC.protocol).read_bytes()
    plan = {"schema": f"pd-lab.{SPEC.identity}-panel-plan.v1", "waypoint_experiment": MODE,
            "preservation_panel": False, "candidate_id": SPEC.modes[MODE], "correction_cap": 24,
            "candidate_executable_sha256": source["executable_sha256"],
            "candidate_rust_source_sha256": source["rust_source_tree_sha256"],
            "candidate_report_script_sha256": IDENTITY["report_script_sha256"],
            "case_wall_limit_s": 60, "maximum_measured_attempts": len(rows), "retries": 0,
            "tuning": False, "publication": False, "experiment_plan_sha256": study.digest(protocol),
            "previous_receipt_sha256": panel.SEAL[0], "previous_results_sha256": panel.SEAL[1], **METADATA}
    survey.reserve(ROOT)
    study.write_new(ROOT / "plan.json", study.encoded(plan))
    study.write_new(ROOT / "manifest.json", study.encoded({"rows": rows, "source": source, "protected": safe}))
    study.write_new(ROOT / "experiment-plan.md", protocol)
    for name in ("receipt.json", "ballistic-feedback-sweep.json"):
        target = "previous-receipt.json" if name == "receipt.json" else "previous-results.json"
        study.write_new(ROOT / target, (SPEC.previous / name).read_bytes())
        study.write_new(ROOT / ("focus-" + name), (FOCUS / name).read_bytes())
    for name, digest in source["files"].items():
        data = (survey.REPO / name).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed during diagnostic snapshot")
        study.write_new(ROOT / "candidate/source" / name, data)
    for name, binary in (("pd-eval", SPEC.native), ("batch-report", SPEC.renderer)):
        study.write_new(ROOT / "candidate/bin" / name, binary.read_bytes())
        (ROOT / "candidate/bin" / name).chmod(0o755)
    for row in rows[:1000]:
        study.write_new(ROOT / row["scenario_path"], d.safe_file(SPEC.previous, row["scenario_path"]).read_bytes())
        study.write_new(ROOT / "previous-feedback" / (row["case_id"] + ".json"),
                        (SPEC.previous / "runs" / row["case_id"] / "feedback.json").read_bytes())
    started = time.monotonic()
    actual = measure(ROOT, rows, plan)
    after, protected_after = entry.source(SPEC), protected()
    report = {"schema": f"pd-lab.{SPEC.identity}-panel.v1", "rows": actual,
              "source_after": after, "protected_after": protected_after,
              "stopped_reason": None if all(r["status"] == "recorded" for r in actual)
              and source == after and safe == protected_after else "evidence_or_collection_error",
              "collection_wall_s": time.monotonic() - started, "summary": ridge.metrics(actual)}
    study.write_new(ROOT / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(ROOT / "candidate/bin/batch-report"), str(ROOT)], capture_output=True, check=True, timeout=60)
    study.write_new(ROOT / "receipt.json", study.encoded({"files": survey.inventory(ROOT)}))
    return verify()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    args = parser.parse_args()
    print(json.dumps(run() if args.action == "run" else verify()), flush=True)
