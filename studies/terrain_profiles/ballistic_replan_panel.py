"""Fixed 12-case replan mechanism panel plus two exact repeats; no tuning.

Uses original receipt-bound scenario bytes, create-only native captures and the
shared candidate verifier. This is diagnosis, not planner-pack acceptance.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess

import ballistic_feedback_sweep as sweep
import diagnostics as d
import study
import survey

INDICES = [55, 32, 30, 20, 0, 715, 1, 6, 142]
CONTROLS = ["v2_clear_845", "fresh_clear_uphill_805", "fresh_clear_downhill_805"]
REPEATS = ["random-055", "random-142"]
NATIVE = survey.REPO / "target/ballistic-replan-20261008/release/pd-eval"


def run(root, final=False, terrain=False):
    old_plan = sweep.contract()
    cases, _, bound = sweep.baseline(old_plan)
    additions = [34, 41, 66, 84, 268, 308, 349] if terrain else [308, 349] if final else []
    selected = [dict(cases[i]) for i in INDICES + additions]
    accepted = survey.REPO / "outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native"
    subprocess.run([str(survey.REPO / "target/release/pd-eval"), "check-planner-v2", "--dir", str(accepted)],
                   capture_output=True, check=True, timeout=60)
    expanded = {r["case_id"]: r["scenario"] for r in d.read(accepted / "expanded-inputs.json")}
    inputs = {c["case_id"]: bound(c["scenario_path"]) for c in selected}
    for case_id in CONTROLS:
        relative = f"runs/{case_id}/scenario.json"
        data = d.safe_file(accepted, relative).read_bytes()
        if json.loads(data) != expanded[case_id]:
            raise ValueError("unauthenticated control input")
        inputs[case_id] = data
        selected.append({"case_id": case_id, "attempt_id": case_id, "cohort": "control",
                         "scenario_sha256": study.digest(data), "status": "not_attempted"})
    for row in selected:
        row["scenario_path"] = f'scenarios/{row["case_id"]}.json'
    repeats = ["random-034", "random-055"] if terrain else ["random-308", "random-349"] if final else REPEATS
    rows = selected + [dict(next(r for r in selected if r["case_id"] == case_id),
                            attempt_id=f"repeat-{case_id}", cohort="repeat") for case_id in repeats]
    native = survey.REPO / "target/ballistic-terrain-correction-20261008/release/pd-eval" if terrain else survey.REPO / "target/ballistic-replan-20261008-r2/release/pd-eval" if final else NATIVE
    source = sweep.source_state(native)
    # The native dev attempt records the matching Rust content tree and binary.
    seal_path = "outputs/eval/planner_v2_random_terrain/dev-terrain-correction-20261008-034/attempt.json" if terrain else "outputs/research/ballistic-replan-20261008/dev-r2-308/attempt.json" if final else "outputs/research/ballistic-replan-20261008/dev-02-055/attempt.json"
    native_seal = d.read(survey.REPO / seal_path)
    if native_seal["binary_sha256"] != source["executable_sha256"]:
        raise ValueError("panel binary differs from reviewed dev build")
    plan = {"candidate_id": sweep.TERRAIN_CANDIDATE if terrain else "ballistic_feedback_v2_replan_r2" if final else "ballistic_feedback_v2_replan", "correction_cap": 24,
            "candidate_executable_sha256": source["executable_sha256"],
            "candidate_rust_source_sha256": native_seal["content_source"]["rust_source_tree_sha256"],
            "candidate_report_script_sha256": source["files"]["pd-report/src/planning_cycles.js"],
            "case_wall_limit_s": 60, "maximum_measured_attempts": len(rows), "retries": 0, "tuning": False}
    protected = sweep.protected_state(old_plan)
    survey.reserve(root)
    study.write_new(root / "plan.json", study.encoded(plan))
    study.write_new(root / "manifest.json", study.encoded({"rows": rows, "source": source, "protected": protected}))
    study.write_new(root / "candidate/bin/pd-eval", native.read_bytes())
    (root / "candidate/bin/pd-eval").chmod(0o755)
    for case_id, data in inputs.items():
        study.write_new(root / f"scenarios/{case_id}.json", data)
    with ThreadPoolExecutor(max_workers=4) as pool:
        # Repeats wait for all primaries, so exact comparison never races.
        actual = list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[:len(selected)]))
        actual += list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[len(selected):]))
    if sweep.source_state(native) != source or sweep.protected_state(old_plan) != protected:
        raise ValueError("panel source/protected state drift")
    study.write_new(root / "panel.json", study.encoded({"rows": actual, "source_after": source,
                                                       "protected_after": protected}))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    for row in actual:
        print(json.dumps({"id": row["attempt_id"], "status": row["status"], "result": row["result"],
                          "failure": row["failure"]}), flush=True)
    if any(r["status"] != "recorded" for r in actual):
        raise ValueError("panel infrastructure/evidence failure; artifacts retained")
    return actual


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--final", action="store_true", help="Include both same-goal regression cases and repeats")
    parser.add_argument("--terrain-correction", action="store_true", help="Run the fixed terrain correction mechanism/control panel")
    args = parser.parse_args()
    if args.final and args.terrain_correction:
        parser.error("choose one panel revision")
    run(args.output.resolve(), args.final, args.terrain_correction)
