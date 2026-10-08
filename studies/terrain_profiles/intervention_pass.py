"""Create-only A/B timing pass over saved inputs, not a new terrain campaign."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import copy
import json
from pathlib import Path
import time

import study
import survey

HERE = Path(__file__).resolve().parent
PLAN = HERE / "intervention_timing_plan.json"


def read(path):
    return json.loads(path.read_bytes())


def motion(value):
    """Only additive query diagnostics and three wall timings are excluded."""
    value = copy.deepcopy(value)
    value.pop("timings", None)
    for cycle in value.get("cycles", []):
        if cycle.get("local_search") is not None:
            cycle["local_search"].pop("row_diagnostics", None)
    return value


def contract():
    plan = read(PLAN)
    if (plan["schema"] != "pd-lab.intervention-timing-pass.v1"
            or plan["focus_indices"] != [0, 13, 23, 34, 35, 43, 49, 50, 54, 56, 60, 70, 85, 93, 94, 99]
            or plan["repeat_indices"] != [35, 50] or plan["seed_count"] != 100
            or plan["successful_focus_controls"] != [34, 50, 54]
            or [plan[k] for k in ("workers", "case_wall_limit_s", "campaign_wall_limit_s")] != [4, 300, 10800]):
        raise ValueError("changed bounded timing-pass contract")
    return plan


def baseline(plan):
    root = survey.REPO / plan["baseline_capture"]
    for name, key in (("manifest.json", "baseline_manifest_sha256"),
                      ("inputs-receipt.json", "baseline_inputs_receipt_sha256")):
        if study.digest((root / name).read_bytes()) != plan[key]:
            raise ValueError("changed challenge input identity")
    survey.verify(root)
    return root, read(root / "manifest.json")


def verify(root, plan=None, plan_path=PLAN):
    plan = contract() if plan is None else plan
    manifest, report, receipt = (read(root / name) for name in ("manifest.json", "survey.json", "receipt.json"))
    if survey.inventory(root) != receipt["files"]:
        raise ValueError("capture inventory differs")
    survey.check_inventory(root, receipt["files"])
    phase = manifest["pass_phase"]
    if phase not in ("diagnostics", "focus", "challenge"):
        raise ValueError("unknown phase")
    for name, key in (("baseline-manifest.json", "baseline_manifest_sha256"),
                      ("baseline-inputs-receipt.json", "baseline_inputs_receipt_sha256")):
        if study.digest((root / "inputs" / name).read_bytes()) != plan[key]:
            raise ValueError("unbound baseline identity")
    original = read(root / "inputs/baseline-manifest.json")
    inputs = read(root / "inputs/baseline-inputs-receipt.json")["files"]
    indices = phase_indices(plan, phase)
    expected = [original["random_cases"][i] for i in indices]
    if (manifest["random_cases"] != expected or manifest["sentinel_cases"] != []
            or (root / "plan.json").read_bytes() != plan_path.read_bytes()
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or report["source_before"] != manifest["source"]
            or report["source_before"] != report["source_after"]):
        raise ValueError("phase/input/source identity mismatch")
    for path, digest in manifest["source"]["files"].items():
        if study.digest((root / "inputs/source" / path).read_bytes()) != digest:
            raise ValueError("source snapshot mismatch")
    for row in expected:
        if study.digest((root / row["scenario_path"]).read_bytes()) != inputs[row["scenario_path"]]:
            raise ValueError("scenario changed from challenge")
    attempts = [(r, "random", r["case_id"]) for r in expected]
    if phase == "challenge":
        attempts += [(original["random_cases"][i], "repeat", f"repeat-{i:03}") for i in plan["repeat_indices"]]
    if len(report["rows"]) != len(attempts):
        raise ValueError("attempt count differs")
    for row, (expected_row, cohort, identifier) in zip(report["rows"], attempts):
        if (row["cohort"] != cohort or row["attempt_id"] != identifier
                or any(row.get(k) != expected_row.get(k) for k in ("case_id", "seed", "scenario_path", "geometry"))):
            raise ValueError("attempt input/order differs")
        if row["status"] != "recorded":
            if report["stopped_reason"] is None:
                raise ValueError("unaccounted attempt")
            continue
        output = root / row["output_dir"]
        flight = read(output / "flight.json")
        if survey.validate_record(flight, read(output / "summary.json")) != row["result"]:
            raise ValueError("result projection differs")
        if read(output / "scenario.json") != read(root / row["scenario_path"]):
            raise ValueError("executed another input")
        if cohort == "repeat":
            survey.comparison.compare(flight, read(root / "runs" / row["case_id"] / "flight.json"))
        if phase == "diagnostics":
            old = read(motion_baseline(plan) / "runs" / row["case_id"] / "flight.json")
            if motion(flight) != motion(old):
                raise ValueError("diagnostics changed motion/selection/proofs")
    if report["summary"] != survey.summary_for(report["rows"]):
        raise ValueError("summary differs")
    return report


def phase_indices(plan, phase):
    if phase == "challenge":
        return list(range(100))
    if phase == "diagnostics":
        return plan.get("diagnostic_indices", plan["focus_indices"])
    return plan["focus_indices"]


def motion_baseline(plan):
    return survey.REPO / plan.get("motion_baseline_capture", plan["baseline_capture"])


def run(args, plan=None, plan_path=PLAN):
    plan = contract() if plan is None else plan
    old_root, old = baseline(plan)
    binary = args.binary.resolve()
    source = survey.source_state(binary)
    protected = survey.protected_state()
    root = args.output.resolve()
    survey.reserve(root)
    indices = phase_indices(plan, args.phase)
    cases = [old["random_cases"][i] for i in indices]
    manifest = {"schema": "pd-lab.random-terrain-inputs.v1", "source": source,
                "pass_phase": args.phase, "plan_source_path": str(plan_path.relative_to(survey.REPO)),
                "random_cases": cases, "sentinel_cases": [], "protected": protected}
    study.write_new(root / "plan.json", plan_path.read_bytes())
    for name in ("manifest.json", "inputs-receipt.json"):
        study.write_new(root / "inputs" / ("baseline-" + name), (old_root / name).read_bytes())
    for path, digest in source["files"].items():
        data = (survey.REPO / path).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source drift during preparation")
        study.write_new(root / "inputs/source" / path, data)
    for row in cases:
        study.write_new(root / row["scenario_path"], (old_root / row["scenario_path"]).read_bytes())
    study.write_new(root / "manifest.json", study.encoded(manifest))
    rows = [dict(r, attempt_id=r["case_id"], cohort="random", status="not_attempted") for r in cases]
    if args.phase == "challenge":
        rows += [dict(old["random_cases"][i], attempt_id=f"repeat-{i:03}", cohort="repeat", status="not_attempted") for i in plan["repeat_indices"]]
    study.write_new(root / "run-start.json", study.encoded({"source": source, "attempts": rows}))
    started, stopped = time.monotonic(), None
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        for start in range(0, len(rows), plan["workers"]):
            if survey.source_state(binary) != source or survey.protected_state() != protected:
                stopped = "source/executable/accepted evidence drift"
                break
            remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - started)
            if remaining <= 0:
                stopped = "campaign wall bound"
                break
            wave = list(range(start, min(start + plan["workers"], len(rows))))
            futures = [(i, pool.submit(survey.attempt, binary, root, rows[i], min(remaining, plan["case_wall_limit_s"]))) for i in wave]
            for i, future in futures:
                rows[i] = future.result()
                if args.phase == "diagnostics" and rows[i]["status"] == "recorded":
                    actual = read(root / rows[i]["output_dir"] / "flight.json")
                    original = read(motion_baseline(plan) / "runs" / rows[i]["case_id"] / "flight.json")
                    if motion(actual) != motion(original):
                        rows[i].update(status="evidence_error", failure="instrumentation changed motion")
                study.write_new(root / f'ledger/{rows[i]["attempt_id"]}.json', study.encoded(rows[i]))
                print(json.dumps({k: rows[i].get(k) for k in ("attempt_id", "status", "result", "failure")}), flush=True)
            bad = next((rows[i] for i in wave if rows[i]["status"] != "recorded"), None)
            if bad:
                stopped = f'{bad["attempt_id"]}: {bad["failure"] or bad["status"]}'
                break
    after = survey.source_state(binary)
    if after != source or survey.protected_state() != protected:
        stopped = stopped or "source/executable/accepted evidence drift"
    report = {"schema": "pd-lab.random-terrain-survey.v1", "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "source_before": source, "source_after": after, "stopped_reason": stopped,
              "rows": rows, "summary": survey.summary_for(rows)}
    study.write_new(root / "survey.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    verify(root, plan, plan_path)
    print(json.dumps({"capture": str(root), "phase": args.phase, "summary": report["summary"], "stopped_reason": stopped}), flush=True)
    if stopped:
        raise RuntimeError("pass stopped; evidence retained")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    command = sub.add_parser("run")
    command.add_argument("--phase", choices=("diagnostics", "focus", "challenge"), required=True)
    command.add_argument("--binary", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command = sub.add_parser("verify")
    command.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    else:
        print(json.dumps({"verified": str(args.capture), "summary": verify(args.capture)["summary"], "fresh_flights": 0}))
