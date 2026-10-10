"""Create-only V13 admission, gated sweep or separately authorized diagnostic sweep."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import sys
import time

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "studies/terrain_profiles"))
import ballistic_feedback_sweep as sweep
import diagnostics as d
import ridge_waypoint_panel as ridge
import study
import survey
import waypoint_clearance_panel as controls

PROTOCOL = "docs/ballistic_terminal_coordination_plan.md"
RUNNER = "scripts/run-terminal-coordination.py"
NATIVE = REPO / "target/ballistic-terminal-coordination-20261009/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
V10 = survey.OUTPUTS / "capture-coast-terminal-sweep-20261009-v1"
V12 = survey.OUTPUTS / "capture-terminal-centering-sweep-20261009-v1-complete"
CONTROL_SOURCE = survey.OUTPUTS / "capture-terminal-centering-preservation-20261009-v1"
PANEL = survey.OUTPUTS / "capture-terminal-coordination-panel-20261009-v1"
FULL = survey.OUTPUTS / "capture-terminal-coordination-sweep-20261009-v1"
DIAGNOSTIC_FULL = survey.OUTPUTS / "capture-terminal-coordination-diagnostic-1k-20261009-v1"
DIAGNOSTIC_PROTOCOL = "docs/ballistic_terminal_coordination_sweep_plan.md"
PANEL_SEAL = (
    "12e232a09a929d8725b6feee81dd311f6f1401fd6744f6c7e57b614e43980293",
    "c55328ce66f624ad5e8228b47f816bc4f128b86236b1e33ca0533517d0895640",
)
REPEATS = [142, 150, 288, 757, 974]
SEALS = {
    "v12": ("98e072430d172a833d9b3f702eb46179a66ad4b47cb09747f98ffe74f15107ac",
            "4f86018a4d6ad29e50d49a0f424276ffe5c0284d804a2d266b6eed17dceece34"),
    "controls": ("e66d9a78e7eb1c80fe6d843e4755f53d67369126d1267782bf208c69b3f7783a",
                 "8c1575acf5781842655c4e830624f1c1291b5067a9efadf71cf9ec11e72c293b"),
}


def authenticated(capture, name):
    receipt_bytes = (capture / "receipt.json").read_bytes()
    files = json.loads(receipt_bytes)["files"]
    data = (capture / "ballistic-feedback-sweep.json").read_bytes()
    if name in SEALS and (study.digest(receipt_bytes), study.digest(data)) != SEALS[name]:
        raise ValueError("different prior capture: " + name)
    if study.digest(data) != files["ballistic-feedback-sweep.json"]:
        raise ValueError("unbound prior result")
    survey.check_inventory(capture, files)
    return json.loads(data), files


def cohorts(old, current):
    previous = {r["case_id"]: r for r in old["rows"] if r["cohort"] == "random"}
    now = {r["case_id"]: r for r in current["rows"] if r["cohort"] == "random"}
    if list(previous) != [f"random-{i:03}" for i in range(1000)] or list(now) != list(previous):
        raise ValueError("wrong paired population")
    gains = [name for name, r in now.items() if (r.get("result") or {}).get("verified_landing", False)
             and not previous[name]["result"]["verified_landing"]]
    losses = [name for name, r in now.items() if previous[name]["result"]["verified_landing"]
              and not (r.get("result") or {}).get("verified_landing", False)]
    if (len(gains), len(losses)) != (75, 45):
        raise ValueError("changed declared diagnostic cohorts")
    return gains, losses


def source():
    value = sweep.source_state(NATIVE)
    value.update(renderer_sha256=study.digest(RENDERER.read_bytes()), rust_source_tree_sha256=ridge.rust_digest())
    for path in (RUNNER, PROTOCOL, DIAGNOSTIC_PROTOCOL, "scripts/test-terminal-coordination.py"):
        value["files"][path] = study.digest((REPO / path).read_bytes())
    return value


def protected():
    value = survey.protected_state()
    for capture in (V10, V12, CONTROL_SOURCE):
        for name in ("receipt.json", "ballistic-feedback-sweep.json", "index.html"):
            path = capture / name
            value[str(path.relative_to(REPO))] = study.digest(path.read_bytes())
    for relative in ("target/release/pd-eval", "fixtures/reports/report_navigation.json",
                     "outputs/index.html", "outputs/reports/index.html", "outputs/reports/eval/index.html"):
        path = REPO / relative
        value[relative] = study.digest(path.read_bytes())
    return value


def prefix_equal(current, previous):
    entries = [next((r["origin"]["physics_step"] for r in f["refreshes"]
                     if r["decision"] in ("maintained_landing_entry", "coast_terminal_entry")), None)
               for f in (current, previous)]
    if entries[0] != entries[1]:
        raise ValueError("terminal entry clock changed")
    if entries[0] is None:
        if current["ordinary_flight"] != previous["ordinary_flight"]:
            raise ValueError("nonterminal flight changed")
    else:
        end = entries[0]
        actions = lambda f: [a for a in f["ordinary_flight"]["actions"] if a["physics_step"] < end]
        updates = lambda f: [u for u in f["updates"] if u["physics_step"] < end]
        if actions(current) != actions(previous) or updates(current) != updates(previous):
            raise ValueError("pre-terminal command prefix changed")
    if current["handoffs"] != previous["handoffs"] or current["refreshes"] != previous["refreshes"]:
        # Short-command rejection records after entry legitimately change.
        before = lambda f: [r for r in f["refreshes"] if entries[0] is None
                           or r["origin"]["physics_step"] < entries[0]
                           or r["decision"] in ("maintained_landing_entry", "coast_terminal_entry")]
        if current["handoffs"] != previous["handoffs"] or before(current) != before(previous):
            raise ValueError("pre-terminal planning evidence changed")


def admission(rows, gains, losses):
    primary = {r["case_id"]: r for r in rows if r["cohort"] != "repeat"}
    landed = lambda name: (primary[name].get("result") or {}).get("verified_landing", False)
    lost_gains = [name for name in gains if not landed(name)]
    recovered = [name for name in losses if landed(name)]
    missing_controls = [name for name in controls.CONTROLS if not landed(name)]
    missing_upright = [f"random-{i:03}" for i in (559, 757, 888) if not landed(f"random-{i:03}")]
    return {"passed": not (lost_gains or missing_controls or missing_upright) and bool(recovered),
            "lost_gains": lost_gains, "recovered_losses": recovered,
            "missing_controls": missing_controls, "missing_upright": missing_upright}


def flight_identity(value):
    """Collector/docs may evolve; executable, Rust and presentation stay frozen."""
    return {key: value[key] for key in (
        "executable_sha256", "rust_source_tree_sha256", "renderer_sha256"
    )} | {"report_script_sha256": value["files"]["pd-report/src/planning_cycles.js"]}


def check_sweep_admission(stage, admitted, gains, losses, seal):
    verdict = admission(admitted["rows"], gains, losses)
    if admitted["stopped_reason"] is not None:
        raise ValueError("incomplete admission capture")
    if stage == "sweep":
        if not verdict["passed"]:
            raise ValueError("admission failed; conditional sweep is closed")
        if seal != admitted["source_after"]:
            raise ValueError("candidate changed after admission")
    elif stage == "diagnostic-sweep":
        if flight_identity(seal) != flight_identity(admitted["source_after"]):
            raise ValueError("frozen flight candidate changed")
    else:
        raise ValueError("unknown sweep stage")
    return verdict


def verify(root):
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("capture inventory differs")
    survey.check_inventory(root, receipt)
    plan, manifest, report = [d.read(root / name) for name in ("plan.json", "manifest.json", "ballistic-feedback-sweep.json")]
    if (plan["candidate_id"] != sweep.COORDINATION_CANDIDATE or plan["retries"] != 0 or plan["tuning"]
            or plan["publication"] or report["source_after"] != manifest["source"]
            or report["protected_after"] != manifest["protected"]
            or study.digest((root / "experiment-plan.md").read_bytes()) != plan["experiment_plan_sha256"]
            or len(report["rows"]) != plan["maximum_measured_attempts"]):
        raise ValueError("capture contract/source differs")
    if (plan["stage"] not in ("panel", "sweep", "diagnostic-sweep") or plan["waypoint_experiment"] != "terminal-coordination"
            or plan["correction_cap"] != 24 or plan["case_wall_limit_s"] != 60
            or plan["campaign_wall_limit_s"] != 7200
            or plan["maximum_measured_attempts"] != (129 if plan["stage"] == "panel" else 1005)):
        raise ValueError("changed fixed stage contract")
    archived = {}
    for name in ("v10", "v12", "controls"):
        prior_receipt = (root / (name + "-receipt.json")).read_bytes()
        prior_data = (root / (name + "-ballistic-feedback-sweep.json")).read_bytes()
        files = json.loads(prior_receipt)["files"]
        if (study.digest(prior_data) != files["ballistic-feedback-sweep.json"]
                or (name in SEALS and (study.digest(prior_receipt), study.digest(prior_data)) != SEALS[name])):
            raise ValueError("archived comparison receipt differs")
        archived[name] = (json.loads(prior_data), files)
    gains, losses = cohorts(archived["v10"][0], archived["v12"][0])
    selected = set(gains + losses + ["random-559"])
    primary = [f"random-{i:03}" for i in range(1000) if plan["stage"] != "panel" or f"random-{i:03}" in selected]
    if plan["stage"] == "panel":
        primary += controls.CONTROLS
    expected = primary + [f"repeat-random-{i:03}" for i in REPEATS]
    if [r["attempt_id"] for r in report["rows"]] != expected:
        raise ValueError("declared cohort or repeat inventory differs")
    admission_files = {}
    if plan["stage"] == "diagnostic-sweep":
        panel_receipt = (root / "admission-receipt.json").read_bytes()
        panel_data = (root / "admission-ballistic-feedback-sweep.json").read_bytes()
        if (study.digest(panel_receipt), study.digest(panel_data)) != PANEL_SEAL:
            raise ValueError("different diagnostic authorization reference")
        admitted = json.loads(panel_data)
        admission_files = json.loads(panel_receipt)["files"]
        verdict = check_sweep_admission("diagnostic-sweep", admitted, gains, losses, manifest["source"])
        if (not plan.get("user_authorized_diagnostic") or plan.get("panel_admission") != verdict
                or plan.get("panel_receipt_sha256") != PANEL_SEAL[0]
                or plan.get("panel_results_sha256") != PANEL_SEAL[1]):
            raise ValueError("diagnostic contract or retained admission verdict differs")
    for path, digest in manifest["source"]["files"].items():
        if study.digest(d.safe_file(root / "candidate/source", path).read_bytes()) != digest:
            raise ValueError("candidate archived source differs")
    for name, key in (("pd-eval", "executable_sha256"), ("batch-report", "renderer_sha256")):
        if study.digest((root / "candidate/bin" / name).read_bytes()) != manifest["source"][key]:
            raise ValueError("candidate archived binary differs")
    for row, frozen in zip(report["rows"], manifest["rows"]):
        if any(row[k] != v for k, v in frozen.items() if k != "status"):
            raise ValueError("frozen attempt differs")
        if row["status"] == "recorded":
            if sweep.check_record(root, row, plan) != row["result"]:
                raise ValueError("candidate proof differs")
            panel_path = f'runs/{row["case_id"]}/feedback.json'
            if panel_path in admission_files and study.digest(
                    (root / row["output_dir"] / "feedback.json").read_bytes()) != admission_files[panel_path]:
                raise ValueError("frozen panel flight changed in full sweep")
            prior = root / "previous-feedback" / (row["case_id"] + ".json")
            if prior.exists():
                name = "controls" if row["case_id"] in controls.CONTROLS else "v12"
                if study.digest(prior.read_bytes()) != archived[name][1][f'runs/{row["case_id"]}/feedback.json']:
                    raise ValueError("archived previous-flight source differs")
                prefix_equal(d.read(root / row["output_dir"] / "feedback.json"), d.read(prior))
        elif report["stopped_reason"] is None:
            raise ValueError("unattempted row hidden as complete")
    if sweep.summary(report["rows"]) != report["summary"]:
        raise ValueError("summary differs")
    return report


def run(stage):
    old, old_files = authenticated(V10, "v10")
    current, current_files = authenticated(V12, "v12")
    control, control_files = authenticated(CONTROL_SOURCE, "controls")
    gains, losses = cohorts(old, current)
    root = {"panel": PANEL, "sweep": FULL, "diagnostic-sweep": DIAGNOSTIC_FULL}[stage]
    admitted = verify(PANEL) if stage != "panel" else None
    seal, safe = source(), protected()
    verdict = check_sweep_admission(stage, admitted, gains, losses, seal) if admitted is not None else None
    if stage == "diagnostic-sweep" and tuple(study.digest((PANEL / name).read_bytes()) for name in
            ("receipt.json", "ballistic-feedback-sweep.json")) != PANEL_SEAL:
        raise ValueError("different panel reference")
    selected = set(gains + losses + ["random-559"])
    rows = []
    inputs, previous = {}, {}
    for row in old["rows"]:
        if row["cohort"] != "random" or (stage == "panel" and row["case_id"] not in selected):
            continue
        item = {k: v for k, v in row.items() if k not in ("output_dir", "exit_code", "failure", "result", "wall_s")}
        item.update(status="not_attempted", previous_result=row["result"])
        rows.append(item)
        data = d.safe_file(V10, row["scenario_path"]).read_bytes()
        if study.digest(data) != old_files[row["scenario_path"]]:
            raise ValueError("unbound original input")
        inputs[row["case_id"]] = data
        newer = next(r for r in current["rows"] if r["attempt_id"] == row["case_id"])
        if newer["status"] == "recorded":
            previous[row["case_id"]] = d.safe_file(V12, newer["output_dir"] + "/feedback.json").read_bytes()
    if stage == "panel":
        for name in controls.CONTROLS:
            row = next(r for r in control["rows"] if r["attempt_id"] == name)
            item = {k: v for k, v in row.items() if k not in ("output_dir", "exit_code", "failure", "result", "wall_s")}
            item.update(status="not_attempted", previous_result=row["result"])
            rows.append(item)
            inputs[name] = d.safe_file(CONTROL_SOURCE, row["scenario_path"]).read_bytes()
            previous[name] = d.safe_file(CONTROL_SOURCE, row["output_dir"] + "/feedback.json").read_bytes()
    rows += [dict(next(r for r in rows if r["case_id"] == f"random-{i:03}"),
                  attempt_id=f"repeat-random-{i:03}", cohort="repeat") for i in REPEATS]
    expected_count = 129 if stage == "panel" else 1005
    if len(rows) != expected_count:
        raise ValueError("wrong declared attempt count")
    protocol = (REPO / (DIAGNOSTIC_PROTOCOL if stage == "diagnostic-sweep" else PROTOCOL)).read_bytes()
    plan = {"schema": "pd-lab.terminal-coordination-plan.v1", "stage": stage,
            "candidate_id": sweep.COORDINATION_CANDIDATE, "waypoint_experiment": "terminal-coordination",
            "correction_cap": 24, "case_wall_limit_s": 60, "campaign_wall_limit_s": 7200,
            "maximum_measured_attempts": len(rows), "retries": 0, "tuning": False, "publication": False,
            "candidate_executable_sha256": seal["executable_sha256"],
            "candidate_rust_source_sha256": seal["rust_source_tree_sha256"],
            "candidate_report_script_sha256": seal["files"]["pd-report/src/planning_cycles.js"],
            "experiment_plan_sha256": study.digest(protocol)}
    if stage == "diagnostic-sweep":
        plan.update(user_authorized_diagnostic=True, panel_admission=verdict,
                    panel_receipt_sha256=PANEL_SEAL[0], panel_results_sha256=PANEL_SEAL[1])
    survey.reserve(root)
    for name, value in (("plan.json", plan), ("manifest.json", {"rows": rows, "source": seal, "protected": safe})):
        study.write_new(root / name, study.encoded(value))
    study.write_new(root / "experiment-plan.md", protocol)
    for path in seal["files"]:
        study.write_new(root / "candidate/source" / path, (REPO / path).read_bytes())
    for name, binary in (("pd-eval", NATIVE), ("batch-report", RENDERER)):
        study.write_new(root / "candidate/bin" / name, binary.read_bytes())
        (root / "candidate/bin" / name).chmod(0o755)
    for row in rows:
        if row["cohort"] != "repeat":
            study.write_new(root / row["scenario_path"], inputs[row["case_id"]])
    for name, data in previous.items():
        study.write_new(root / "previous-feedback" / (name + ".json"), data)
    for name, capture in (("v10", V10), ("v12", V12), ("controls", CONTROL_SOURCE)):
        for artifact in ("receipt.json", "ballistic-feedback-sweep.json"):
            study.write_new(root / (name + "-" + artifact), (capture / artifact).read_bytes())
    if admitted is not None:
        for name in ("receipt.json", "ballistic-feedback-sweep.json"):
            study.write_new(root / ("admission-" + name), (PANEL / name).read_bytes())
    started, stopped = time.monotonic(), None
    with ThreadPoolExecutor(max_workers=4) as pool:
        for offset in range(0, len(rows), 20):
            wave = list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[offset:offset + 20]))
            rows[offset:offset + len(wave)] = wave
            study.write_new(root / "progress" / f"wave-{offset:04}.json", study.encoded(wave))
            print(json.dumps({"stage": stage, "attempted": offset + len(wave), "summary": sweep.summary(rows)}), flush=True)
            broken = next((r for r in wave if r["status"] != "recorded" or r["result"]["physical_outcome"] not in
                           ("flying", "landed_on_target") or r["result"]["stop_group"] == "actual_terrain_domain"), None)
            if broken or source() != seal or protected() != safe or time.monotonic() - started >= 7200:
                stopped = "runner_evidence_physical_or_drift_stop" if broken else "drift_or_wall_stop"
                break
    report = {"schema": "pd-lab.terminal-coordination.v1", "rows": rows, "summary": sweep.summary(rows),
              "source_after": source(), "protected_after": protected(), "stopped_reason": stopped}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    checked = verify(root)
    return {"summary": checked["summary"], "admission": admission(rows, gains, losses) if stage == "panel" else None,
            "stopped_reason": stopped}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("panel", "sweep", "diagnostic-sweep", "verify-panel", "verify-sweep", "verify-diagnostic"))
    args = parser.parse_args()
    roots = {"verify-panel": PANEL, "verify-sweep": FULL, "verify-diagnostic": DIAGNOSTIC_FULL}
    result = verify(roots[args.action]) if args.action.startswith("verify-") else run(args.action)
    print(json.dumps(result if not args.action.startswith("verify-") else result["summary"], indent=2))
