"""Frozen native finite-acquisition probe, focused candidate and paired 1k diagnostic."""
import argparse
import json
from pathlib import Path
import subprocess
import time

import ballistic_exit_diagnostic as scheduling
import ballistic_feedback_sweep as sweep
import diagnostics as d
import ridge_waypoint_panel as ridge
import study
import survey
import waypoint_entry_panel as entry

PREVIOUS = scheduling.ROOT
SEAL = ("48f84223cc6e9212276588130168d323d5885f9aa9acdba4cdf323e8787d2ac9",
        "76e9c8088cdb53938ad714260ba5d1aa833241a15ea9a4e66ac56ea046c3d782")
NATIVE = survey.REPO / "target/ballistic-finite-correction-20261009/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
DIRECT = [1, 264, 503, 753]
PROBE = [44, 50, 62, 81, 236, 262, 862, 971, 84, 142, 349, 715, 974, *DIRECT]
FOCUS = [*PROBE, 357, 392, 565, 570, 814, 35, 47, 94, 20]
REPEATS = [44, 262, 715]
FULL_REPEATS = [44, 262, 715, 349, 20]


def capture(stage):
    version = 2 if stage == "focus" else 1
    return survey.OUTPUTS / f"capture-ballistic-finite-correction-{stage}-20261009-v{version}"


def layout(stage):
    indices = PROBE if stage == "probe" else FOCUS if stage == "focus" else range(1000)
    repeats = FULL_REPEATS if stage == "full" else REPEATS
    return [f"random-{i:03}" for i in indices], [f"random-{i:03}" for i in repeats]


def previous_verify(root):
    if tuple(study.digest((root / name).read_bytes()) for name in
             ("receipt.json", "ballistic-feedback-sweep.json")) != SEAL:
        raise ValueError("different frozen 706/1000 reference")
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("reference inventory changed")
    survey.check_inventory(root, receipt)
    report = d.read(root / "ballistic-feedback-sweep.json")
    if report["summary"]["verified_landings"] != 706 or report["stopped_reason"] is not None:
        raise ValueError("reference verdict changed")


def spec(stage):
    mode = "finite-correction-probe" if stage == "probe" else "finite-correction"
    return entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, sweep.FINITE_CANDIDATES,
                           "exit-consistency", mode, lambda _: layout(stage),
                           "docs/ballistic_finite_correction_plan.md", previous_verify,
                           "ballistic-finite-correction")


def verify(root, stage):
    summary = entry.verify(root, spec(stage))
    plan, report = [d.read(root / name) for name in ("plan.json", "ballistic-feedback-sweep.json")]
    if (plan["stage"] != stage or plan["workers"] != 4 or plan["campaign_wall_limit_s"] != 7200
            or plan["correction_cap"] != 24 or plan["case_wall_limit_s"] != 60
            or (plan["previous_receipt_sha256"], plan["previous_results_sha256"]) != SEAL):
        raise ValueError("finite pass limits/reference differ")
    primary = [row for row in report["rows"] if row["cohort"] != "repeat"]
    for row in primary:
        current = d.read(root / row["output_dir"] / "feedback.json")
        prior = d.read(root / "previous-feedback" / (row["case_id"] + ".json"))
        if (stage == "probe" or int(row["case_id"].split("-")[1]) in DIRECT):
            if current["ordinary_flight"] != prior["ordinary_flight"]:
                raise ValueError("probe/direct ordinary flight changed")
        # Both variants must preserve every initial constructor, before any
        # airborne admission can affect the route.
        def initial(feedback):
            return next((r["desired_arc"] for r in feedback["refreshes"] if r["desired_arc"]), None)
        if initial(current) != initial(prior):
            raise ValueError("initial terrain-blind arc changed")
    if stage == "full":
        focus = capture("focus")
        focus_report = d.read(focus / "ballistic-feedback-sweep.json")
        if report["source_after"] != focus_report["source_after"]:
            raise ValueError("candidate changed between focused and full diagnostic")
        for row in focus_report["rows"]:
            if row["cohort"] != "repeat":
                if d.read(root / "runs" / row["case_id"] / "feedback.json") != d.read(focus / row["output_dir"] / "feedback.json"):
                    raise ValueError("complete focused/full overlap differs")
    return summary


def supported(probe_root):
    verify(probe_root, "probe")
    rows = d.read(probe_root / "ballistic-feedback-sweep.json")["rows"]
    return any(q["ideal_conflict"] is not None and q["acquisition"] is not None
               and q["rejection"] is None
               for row in rows if row["cohort"] != "repeat"
               for refresh in d.read(probe_root / row["output_dir"] / "feedback.json")["refreshes"]
               if (q := refresh.get("finite_destination_query")) is not None)


def run(stage):
    s = spec(stage)
    previous_verify(PREVIOUS)
    if stage != "probe" and not supported(capture("probe")):
        raise ValueError("native probe did not support finite admission")
    if stage != "probe":
        probe_source = d.read(capture("probe") / "manifest.json")["source"]
        if study.digest(NATIVE.read_bytes()) != probe_source["executable_sha256"]:
            raise ValueError("native behavior changed after the probe")
    if stage == "full":
        verify(capture("focus"), "focus")  # Integrity only, NOT a zero-loss gate.
    prior = {r["attempt_id"]: r for r in d.read(PREVIOUS / "ballistic-feedback-sweep.json")["rows"]}
    primary, repeats = layout(stage)
    rows = []
    for case_id in primary:
        row = {k: v for k, v in prior[case_id].items() if k not in
               ("output_dir", "exit_code", "failure", "result", "wall_s")}
        rows.append(dict(row, status="not_attempted", previous_result=prior[case_id]["result"]))
    rows += [dict(next(r for r in rows if r["case_id"] == name), attempt_id="repeat-" + name,
                  cohort="repeat") for name in repeats]
    source, safe = entry.source(s), entry.protected(s)
    protocol = (survey.REPO / s.protocol).read_bytes()
    mode = "finite-correction-probe" if stage == "probe" else "finite-correction"
    plan = {"schema": f"pd-lab.{s.identity}-panel-plan.v1", "stage": stage,
            "waypoint_experiment": mode, "preservation_panel": False, "candidate_id": s.modes[mode],
            "correction_cap": 24, "workers": 4, "campaign_wall_limit_s": 7200,
            "candidate_executable_sha256": source["executable_sha256"],
            "candidate_rust_source_sha256": source["rust_source_tree_sha256"],
            "candidate_report_script_sha256": source["files"]["pd-report/src/planning_cycles.js"],
            "case_wall_limit_s": 60, "maximum_measured_attempts": len(rows), "retries": 0,
            "tuning": False, "publication": False, "experiment_plan_sha256": study.digest(protocol),
            "previous_receipt_sha256": SEAL[0], "previous_results_sha256": SEAL[1]}
    root = capture(stage)
    survey.reserve(root)
    study.write_new(root / "plan.json", study.encoded(plan))
    study.write_new(root / "manifest.json", study.encoded({"rows": rows, "source": source, "protected": safe}))
    study.write_new(root / "experiment-plan.md", protocol)
    for name in ("receipt.json", "ballistic-feedback-sweep.json"):
        study.write_new(root / ("previous-receipt.json" if name == "receipt.json" else "previous-results.json"),
                        (PREVIOUS / name).read_bytes())
    for name, digest in source["files"].items():
        data = (survey.REPO / name).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed during finite diagnostic snapshot")
        study.write_new(root / "candidate/source" / name, data)
    for name, binary in (("pd-eval", NATIVE), ("batch-report", RENDERER)):
        study.write_new(root / "candidate/bin" / name, binary.read_bytes())
        (root / "candidate/bin" / name).chmod(0o755)
    for row in rows[:len(primary)]:
        study.write_new(root / row["scenario_path"], d.safe_file(PREVIOUS, row["scenario_path"]).read_bytes())
        study.write_new(root / "previous-feedback" / (row["case_id"] + ".json"),
                        (PREVIOUS / "runs" / row["case_id"] / "feedback.json").read_bytes())
    started = time.monotonic()
    actual = scheduling.measure(root, rows, plan)
    after, protected_after = entry.source(s), entry.protected(s)
    report = {"schema": f"pd-lab.{s.identity}-panel.v1", "rows": actual,
              "source_after": after, "protected_after": protected_after,
              "stopped_reason": None if all(r["status"] == "recorded" for r in actual)
              and source == after and safe == protected_after else "evidence_or_collection_error",
              "collection_wall_s": time.monotonic() - started, "summary": ridge.metrics(actual)}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    return verify(root, stage)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("stage", choices=["probe", "focus", "full"])
    args = parser.parse_args()
    print(json.dumps(run(args.stage) if args.action == "run" else verify(capture(args.stage), args.stage), indent=2))
