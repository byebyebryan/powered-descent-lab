"""Create-only independent acquisition and terminal safety ablations versus 829."""
import argparse
import json
import subprocess
import time

import ballistic_exit_diagnostic as scheduling
import ballistic_feedback_sweep as sweep
import ballistic_recovery_consistency_panel as recovery
import diagnostics as d
import ridge_waypoint_panel as ridge
import study
import survey
import waypoint_entry_panel as entry

PREVIOUS = recovery.capture("full")
SEAL = ("bc4f4ebffd1699a167cc3fdd58f8901b9cb42e9e3a8654b18d53200630c6728b",
        "932693f34d1660c925674a52eb42040cde6caf1b5dc7678c8e2ae62948ae3678")
NATIVE = survey.REPO / "target/ballistic-acquisition-safety-20261010/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
MISSING_AIM = [56, 59, 167, 271, 332, 367, 378, 426, 488, 547, 662, 676,
               764, 786, 811, 817, 844, 850, 870, 902, 943, 971, 979]
WAYPOINT_MISS = [377, 466, 584, 765, 860, 889, 899, 900, 901, 942, 958, 981]
TERMINAL = [24, 48, 119, 138, 190, 392, 439, 530, 538, 564, 576, 650,
            654, 683, 692, 734, 796, 862, 869, 883, 916, 922, 967]
FOCUS = sorted(set(recovery.FOCUS + MISSING_AIM + WAYPOINT_MISS + TERMINAL))
REPEATS = [56, 138, 538, 715, 349]
MODES = {"recovery-consistency": sweep.RECOVERY_CANDIDATES["recovery-consistency"],
         **sweep.SAFETY_CANDIDATES}
STAGES = {"control": "recovery-consistency", "acquisition-focus": "acquisition-gate",
          "terminal-focus": "terminal-safety-fallback", "acquisition-full": "acquisition-gate",
          "terminal-full": "terminal-safety-fallback"}


def capture(stage):
    return survey.OUTPUTS / f"capture-ballistic-acquisition-safety-{stage}-20261010-v1"


def layout(stage):
    return ([f"random-{i:03}" for i in (range(1000) if stage.endswith("-full") else FOCUS)],
            [f"random-{i:03}" for i in REPEATS])


def previous_verify(root):
    if tuple(study.digest((root / name).read_bytes()) for name in
             ("receipt.json", "ballistic-feedback-sweep.json")) != SEAL:
        raise ValueError("different frozen 829/1000 reference")
    if survey.inventory(root) != d.read(root / "receipt.json")["files"]:
        raise ValueError("reference inventory changed")
    report = d.read(root / "ballistic-feedback-sweep.json")
    if report["summary"]["verified_landings"] != 829 or report["stopped_reason"] is not None:
        raise ValueError("reference verdict changed")


def spec(stage):
    return entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "recovery-consistency",
                           STAGES[stage], lambda _: layout(stage),
                           "docs/ballistic_acquisition_safety_plan.md", previous_verify,
                           "ballistic-acquisition-safety")


def protected():
    # Auto-discovery pages can change naturally without changing saved flight
    # evidence. Protect the accepted site/selector, baseline and sources instead.
    value = survey.protected_state()
    paths = ["target/release/pd-eval", "README.md", "docs/reports.md",
             "fixtures/reports/report_navigation.json", "pd-report/src/report_navigation.rs",
             "scripts/README.md", "scripts/serve-reports", "scripts/report_server.py",
             "scripts/test-report-library.mjs", "scripts/test-report-server.py"]
    paths += [str(p.relative_to(survey.REPO)) for p in
              (survey.REPO / "pd-report/src/report_navigation").rglob("*") if p.is_file()]
    for name in paths:
        value[name] = study.digest((survey.REPO / name).read_bytes())
    for name in ("receipt.json", "ballistic-feedback-sweep.json", "index.html"):
        value[str((PREVIOUS / name).relative_to(survey.REPO))] = study.digest((PREVIOUS / name).read_bytes())
    return value


def source(s):
    value = entry.source(s)
    value["files"][s.protocol] = study.digest((survey.REPO / s.protocol).read_bytes())
    return value


def verify(root, stage):
    summary = entry.verify(root, spec(stage))
    plan, report = [d.read(root / name) for name in ("plan.json", "ballistic-feedback-sweep.json")]
    if (plan["stage"] != stage or plan["waypoint_experiment"] != STAGES[stage]
            or plan["workers"] != 4 or plan["campaign_wall_limit_s"] != 7200
            or plan["correction_cap"] != 24 or plan["case_wall_limit_s"] != 60
            or (plan["previous_receipt_sha256"], plan["previous_results_sha256"]) != SEAL):
        raise ValueError("independent safety stage/limits/reference differ")
    for row in report["rows"]:
        current = d.read(root / row["output_dir"] / "feedback.json")
        prior = d.read(root / "previous-feedback" / (row["case_id"] + ".json"))
        if int(row["case_id"].split("-")[1]) in recovery.transition.DIRECT and current["ordinary_flight"] != prior["ordinary_flight"]:
            raise ValueError("direct control flight changed")
        def initial(feedback):
            return next((r["desired_arc"] for r in feedback["refreshes"] if r["desired_arc"]), None)
        if initial(current) != initial(prior):
            raise ValueError("initial terrain-blind construction changed")
        if STAGES[stage] == "terminal-safety-fallback":
            old_first = next((u["physics_step"] for u in prior["updates"] if u["phase"] == "maintained_terminal"), None)
            new_first = next((u["physics_step"] for u in current["updates"] if u["phase"] == "maintained_terminal"), None)
            if old_first != new_first:
                raise ValueError("terminal-only candidate changed entry clock")
            prefix = lambda f: [a for a in f["ordinary_flight"]["actions"] if old_first is None or a["physics_step"] < old_first]
            if prefix(current) != prefix(prior):
                raise ValueError("terminal-only candidate changed preterminal commands")
            if old_first is None and current["ordinary_flight"] != prior["ordinary_flight"]:
                raise ValueError("terminal-only changed a never-terminal flight")
    if stage != "control" and report["source_after"] != d.read(capture("control") / "manifest.json")["source"]:
        raise ValueError("source changed after exact control")
    if stage.endswith("-full"):
        focus = capture(stage.replace("-full", "-focus"))
        for row in d.read(focus / "ballistic-feedback-sweep.json")["rows"]:
            if row["cohort"] != "repeat" and d.read(root / "runs" / row["case_id"] / "feedback.json") != d.read(focus / row["output_dir"] / "feedback.json"):
                raise ValueError("complete focused/full overlap differs")
    return summary


def run(stage):
    s = spec(stage)
    previous_verify(PREVIOUS)
    if stage != "control":
        verify(capture("control"), "control")
        if source(s) != d.read(capture("control") / "manifest.json")["source"]:
            raise ValueError("source changed after exact control")
    if stage.endswith("-full"):
        verify(capture(stage.replace("-full", "-focus")), stage.replace("-full", "-focus"))
    old = {r["attempt_id"]: r for r in d.read(PREVIOUS / "ballistic-feedback-sweep.json")["rows"]}
    primary, repeats = layout(stage)
    rows = []
    for case_id in primary:
        row = {k: v for k, v in old[case_id].items() if k not in
               ("output_dir", "exit_code", "failure", "result", "wall_s")}
        rows.append(dict(row, cohort="random", status="not_attempted", previous_result=old[case_id]["result"]))
    rows += [dict(next(r for r in rows if r["case_id"] == name),
                  attempt_id="repeat-" + name, cohort="repeat") for name in repeats]
    seal, safe = source(s), protected()
    protocol = (survey.REPO / s.protocol).read_bytes()
    plan = {"schema": f"pd-lab.{s.identity}-panel-plan.v1", "stage": stage,
            "waypoint_experiment": STAGES[stage], "preservation_panel": False,
            "candidate_id": s.modes[STAGES[stage]], "correction_cap": 24,
            "workers": 4, "campaign_wall_limit_s": 7200, "case_wall_limit_s": 60,
            "candidate_executable_sha256": seal["executable_sha256"],
            "candidate_rust_source_sha256": seal["rust_source_tree_sha256"],
            "candidate_report_script_sha256": seal["files"]["pd-report/src/planning_cycles.js"],
            "maximum_measured_attempts": len(rows), "retries": 0, "tuning": False,
            "publication": False, "experiment_plan_sha256": study.digest(protocol),
            "previous_receipt_sha256": SEAL[0], "previous_results_sha256": SEAL[1]}
    root = capture(stage)
    survey.reserve(root)
    study.write_new(root / "plan.json", study.encoded(plan))
    study.write_new(root / "manifest.json", study.encoded({"rows": rows, "source": seal, "protected": safe}))
    study.write_new(root / "experiment-plan.md", protocol)
    for name in ("receipt.json", "ballistic-feedback-sweep.json"):
        study.write_new(root / ("previous-receipt.json" if name == "receipt.json" else "previous-results.json"),
                        (PREVIOUS / name).read_bytes())
    for name, digest in seal["files"].items():
        data = (survey.REPO / name).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed during snapshot")
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
    after, protected_after = source(s), protected()
    report = {"schema": f"pd-lab.{s.identity}-panel.v1", "rows": actual,
              "source_after": after, "protected_after": protected_after,
              "stopped_reason": None if all(r["status"] == "recorded" for r in actual)
              and seal == after and safe == protected_after else "evidence_or_collection_error",
              "collection_wall_s": time.monotonic() - started, "summary": ridge.metrics(actual)}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    return verify(root, stage)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("stage", choices=STAGES)
    args = parser.parse_args()
    print(json.dumps(run(args.stage) if args.action == "run" else verify(capture(args.stage), args.stage), indent=2))
