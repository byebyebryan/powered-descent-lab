"""Create-only V5 ridge placement probe/panel on the receipt-bound V4 worlds.

Two development flights plus 21 panel records; no tuning, retry or publication.
Reuses native command/decision proofs and common rich report templates.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import subprocess

import ballistic_feedback_sweep as sweep
import diagnostics as d
import study
import survey
import waypoint_clearance_panel as previous

PREVIOUS = survey.OUTPUTS / "capture-waypoint-clearance-panel-20261008-v1"
NATIVE = survey.REPO / "target/ballistic-ridge-waypoint-20261008/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"


def rust_digest():
    """Match native capture_source_state's sorted depth-first Rust identity."""
    data = bytearray()

    def append(path):
        data.extend(str(path.relative_to(survey.REPO)).encode())
        data.append(0)
        data.extend(path.read_bytes())
        data.append(255)

    def visit(root):
        for path in sorted(root.iterdir()):
            if path.is_symlink():
                raise ValueError("symlink in Rust tree")
            if path.is_dir():
                visit(path)
            elif path.suffix == ".rs":
                append(path)

    crates = ("pd-eval", "pd-core", "pd-plan", "pd-report", "pd-control")
    for name in crates:
        visit(survey.REPO / name / "src")
    for name in ("Cargo.toml", "Cargo.lock", *(f"{c}/Cargo.toml" for c in crates)):
        append(survey.REPO / name)
    return study.digest(data)


def source():
    value = sweep.source_state(NATIVE)
    value["renderer_sha256"] = study.digest(RENDERER.read_bytes())
    value["rust_source_tree_sha256"] = rust_digest()
    return value


def metrics(rows):
    result = sweep.summary(rows)
    controls = [r for r in rows if r["cohort"] == "control"]
    result.update(control_count=len(controls),
                  control_landings=sum(r["status"] == "recorded" and r["result"]["verified_landing"] for r in controls))
    return result


def verify(root):
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("ridge panel inventory differs")
    survey.check_inventory(root, receipt)
    plan, manifest, report = (d.read(root / name) for name in
                             ("plan.json", "manifest.json", "ballistic-feedback-sweep.json"))
    mode = plan["mode"]
    expected = [f"random-{i:03}" for i in ([807, 928] if mode == "probe" else previous.INDICES)]
    if mode == "panel":
        expected += previous.CONTROLS + [f"repeat-random-{i:03}" for i in previous.REPEATS]
    count = 2 if mode == "probe" else 21
    if (mode not in ("probe", "panel") or plan["candidate_id"] != sweep.RIDGE_CANDIDATE
            or plan["maximum_measured_attempts"] != count or plan["retries"] != 0 or plan["tuning"]
            or plan["publication"] or [r["attempt_id"] for r in report["rows"]] != expected
            or report["source_after"] != manifest["source"]
            or report["protected_after"] != manifest["protected"] or report["stopped_reason"] is not None):
        raise ValueError("ridge panel contract/source/denominator differs")
    old_receipt = d.read(root / "previous-receipt.json")["files"]
    old_bytes = (root / "previous-results.json").read_bytes()
    if (study.digest((root / "previous-receipt.json").read_bytes()) != plan["previous_receipt_sha256"]
            or study.digest(old_bytes) != old_receipt["ballistic-feedback-sweep.json"]
            or study.digest(old_bytes) != plan["previous_results_sha256"]):
        raise ValueError("portable V4 comparison differs")
    old = {r["case_id"]: r for r in d.read(root / "previous-results.json")["rows"] if r["cohort"] != "repeat"}
    for row, frozen in zip(report["rows"], manifest["rows"]):
        if any(row[k] != v for k, v in frozen.items() if k != "status"):
            raise ValueError("changed frozen attempt")
        if (row["previous_result"] != old[row["case_id"]]["result"]
                or row["scenario_sha256"] != old_receipt[row["scenario_path"]]
                or row["status"] != "recorded" or sweep.check_record(root, row, plan) != row["result"]):
            raise ValueError("ridge native/paired input proof differs")
        if row["cohort"] == "repeat":
            first = next(r for r in report["rows"] if r["attempt_id"] == row["case_id"])
            if d.read(root / row["output_dir"] / "feedback.json") != d.read(root / first["output_dir"] / "feedback.json"):
                raise ValueError("complete ridge repeat differs")
        if row["cohort"] == "control":
            # The complete old feedback bytes are bound by the V4 receipt too.
            current = d.read(root / row["output_dir"] / "feedback.json")
            saved_control = root / "previous-controls" / (row["case_id"] + ".json")
            if (study.digest(saved_control.read_bytes()) != old_receipt[f'runs/{row["case_id"]}/feedback.json']
                    or current["ordinary_flight"] != d.read(saved_control)["ordinary_flight"]):
                raise ValueError("direct control flight changed")
    seal = manifest["source"]
    for name, key in (("pd-eval", "executable_sha256"), ("batch-report", "renderer_sha256")):
        if study.digest((root / "candidate/bin" / name).read_bytes()) != seal[key]:
            raise ValueError("saved candidate binary differs")
    for name, digest in seal["files"].items():
        if study.digest(d.safe_file(root / "candidate/source", name).read_bytes()) != digest:
            raise ValueError("saved source snapshot differs")
    if metrics(report["rows"]) != report["summary"]:
        raise ValueError("ridge panel summary differs")
    return report["summary"]


def run(root, mode):
    previous.verify(PREVIOUS)
    old = d.read(PREVIOUS / "ballistic-feedback-sweep.json")
    indices = [807, 928] if mode == "probe" else previous.INDICES
    ids = [f"random-{i:03}" for i in indices] + ([] if mode == "probe" else previous.CONTROLS)
    rows = []
    for case_id in ids:
        prior = next(r for r in old["rows"] if r["attempt_id"] == case_id)
        rows.append({k: v for k, v in prior.items() if k not in
                     ("output_dir", "exit_code", "failure", "result", "wall_s")})
        rows[-1].update(status="not_attempted", previous_result=prior["result"])
    primary_count = len(rows)
    if mode == "panel":
        rows += [dict(next(r for r in rows if r["case_id"] == f"random-{i:03}"),
                      attempt_id=f"repeat-random-{i:03}", cohort="repeat") for i in previous.REPEATS]
    seal = source()
    protected = sweep.protected_state(previous.comparison_plan())
    protected.update({str((PREVIOUS / name).relative_to(survey.REPO)): study.digest((PREVIOUS / name).read_bytes())
                      for name in ("receipt.json", "ballistic-feedback-sweep.json", "index.html")})
    plan = {"schema": "pd-lab.ridge-waypoint-panel-plan.v1", "mode": mode,
            "candidate_id": sweep.RIDGE_CANDIDATE, "correction_cap": 24,
            "candidate_executable_sha256": seal["executable_sha256"],
            "candidate_rust_source_sha256": seal["rust_source_tree_sha256"],
            "candidate_report_script_sha256": seal["files"]["pd-report/src/planning_cycles.js"],
            "case_wall_limit_s": 60, "maximum_measured_attempts": len(rows),
            "retries": 0, "tuning": False, "publication": False,
            "previous_receipt_sha256": study.digest((PREVIOUS / "receipt.json").read_bytes()),
            "previous_results_sha256": study.digest((PREVIOUS / "ballistic-feedback-sweep.json").read_bytes())}
    survey.reserve(root)
    study.write_new(root / "plan.json", study.encoded(plan))
    study.write_new(root / "manifest.json", study.encoded({"rows": rows, "source": seal, "protected": protected}))
    for name in ("receipt.json", "ballistic-feedback-sweep.json"):
        study.write_new(root / ("previous-receipt.json" if name == "receipt.json" else "previous-results.json"), (PREVIOUS / name).read_bytes())
    study.write_new(root / "placement-plan.md", (survey.REPO / "docs/ballistic_ridge_waypoint_plan.md").read_bytes())
    for name, digest in seal["files"].items():
        data = (survey.REPO / name).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed during snapshot")
        study.write_new(root / "candidate/source" / name, data)
    for name, binary in (("pd-eval", NATIVE), ("batch-report", RENDERER)):
        study.write_new(root / "candidate/bin" / name, binary.read_bytes())
        (root / "candidate/bin" / name).chmod(0o755)
    for row in rows[:primary_count]:
        study.write_new(root / row["scenario_path"], d.safe_file(PREVIOUS, row["scenario_path"]).read_bytes())
        if row["cohort"] == "control":
            old_bytes = (PREVIOUS / "runs" / row["case_id"] / "feedback.json").read_bytes()
            study.write_new(root / "previous-controls" / (row["case_id"] + ".json"), old_bytes)
    with ThreadPoolExecutor(max_workers=4) as pool:
        actual = list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[:primary_count]))
        actual += list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[primary_count:]))
    after = source()
    protected_after = sweep.protected_state(previous.comparison_plan())
    protected_after.update({name: study.digest((survey.REPO / name).read_bytes()) for name in protected if name not in protected_after})
    report = {"schema": "pd-lab.ridge-waypoint-panel.v1", "rows": actual,
              "source_after": after, "protected_after": protected_after,
              "stopped_reason": None if all(r["status"] == "recorded" for r in actual)
              and after == seal and protected_after == protected else "evidence_error",
              "summary": metrics(actual)}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    result = verify(root)
    for row in actual:
        print(row["attempt_id"], row["status"], row["result"], flush=True)
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["probe", "panel", "verify"])
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    print(verify(root) if args.action == "verify" else run(root, args.action))
