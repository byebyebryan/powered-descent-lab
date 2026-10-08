"""Paired saved-world sweep using the existing isolated cap-24 probe."""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import time

import cap_probe as p
import diagnostics as d
import study
import survey

PLAN = survey.HERE / "cap_sweep_plan.json"
CAP = 24


def contract(path=PLAN):
    plan = d.read(path)
    expected = {"schema": "pd-lab.correction-cap-sweep-plan.v1",
                "baseline_capture": "outputs/eval/planner_v2_random_terrain/capture-validation-1k-20261007-v1",
                "baseline_manifest_sha256": "c0a844f772f45dd2e39e362622e81e4cf5ceb04831925cdc492dd3e4989e2ee8",
                "baseline_receipt_sha256": "0ae5032ac69f4428f3e79649deafed76dfefedeca1139dd4bbdc3e8fc5b22f21",
                "production_executable_sha256": "fdbf20a694de3ac82b79492d119fe850e4bfea5d3c4c70a3ca854a72cfe5d2af",
                "maximum_corrections": CAP, "seed_count": 1000, "workers": 4, "checkpoint_count": 10,
                "repeat_indices": [0, 250, 500, 750, 1, 30, 280], "maximum_measured_attempts": 1010,
                "case_wall_limit_s": 300, "campaign_wall_limit_s": 10800, "build_wall_limit_s": 900}
    if plan != expected:
        raise ValueError("changed paired cap-sweep contract")
    return plan


def baseline_inputs(plan, complete=False):
    root = survey.REPO / plan["baseline_capture"]
    for name, key in (("manifest.json", "baseline_manifest_sha256"), ("receipt.json", "baseline_receipt_sha256")):
        if study.digest((root / name).read_bytes()) != plan[key]:
            raise ValueError("changed cap-sweep baseline")
    if complete:
        survey.verify(root)
    manifest, receipt = d.read(root / "manifest.json"), d.read(root / "receipt.json")["files"]
    for name in ("plan.json", "survey.json"):
        if study.digest((root / name).read_bytes()) != receipt[name]:
            raise ValueError("unbound baseline population/results")
    if (len(manifest["random_cases"]) != 1000 or len(manifest["sentinel_cases"]) != 3
            or manifest["source"]["executable_sha256"] != plan["production_executable_sha256"]):
        raise ValueError("baseline population/executable differs")
    return root, manifest, receipt


def attempts(plan, manifest):
    if len(manifest["sentinel_cases"]) != 3 or len(manifest["random_cases"]) != plan["seed_count"]:
        raise ValueError("paired population differs")
    rows = [dict(c, cohort="sentinel", attempt_id=c["case_id"], status="not_attempted") for c in manifest["sentinel_cases"]]
    rows += [dict(c, cohort="random", attempt_id=c["case_id"], status="not_attempted") for c in manifest["random_cases"]]
    rows += [dict(manifest["random_cases"][i], cohort="repeat", attempt_id=f"repeat-{i:03}", status="not_attempted")
             for i in plan["repeat_indices"]]
    if len(rows) != plan["maximum_measured_attempts"]:
        raise ValueError("paired attempt allowance differs")
    return rows


def bound_read(root, relative, inventory):
    path = d.safe_file(root, relative)
    if study.digest(path.read_bytes()) != inventory[relative]:
        raise ValueError("changed paired artifact: " + relative)
    return d.read(path)


def check_record(root, row, baseline, old_inventory, input_inventory):
    if row["output_dir"] != "runs/" + row["attempt_id"] or row["exit_code"] != 0:
        raise ValueError("paired output/exit identity differs")
    output = d.safe_file(root, row["output_dir"])
    flight = d.read(output / "flight.json")
    result = p.validate_native(flight, d.read(output / "summary.json"), CAP)
    scenario = bound_read(root, row["scenario_path"], input_inventory)
    if d.read(output / "scenario.json") != scenario or result != row["result"]:
        raise ValueError("paired scenario/result differs")
    printed = d.read(root / "logs" / (row["attempt_id"] + ".stdout"))
    if (printed.get("output_dir") != str(output.resolve()) or printed.get("policy_version") != 3
            or any(printed.get(k) != flight[k] for k in ("planning_stop", "correction_count", "initial_nominal_terrain_blocked",
                   "integrity_passed", "final_source_replay_passed", "physical_outcome", "mission_outcome", "reason", "timings"))):
        raise ValueError("paired CLI stdout differs")
    previous = bound_read(baseline, "runs/" + row["case_id"] + "/flight.json", old_inventory)
    comparison = p.compare_prefix(flight, previous)
    if row["cohort"] == "sentinel" and not result["verified_landing"]:
        raise ValueError("paired control did not land")
    if row["cohort"] == "repeat":
        survey.comparison.compare(flight, d.read(root / "runs" / row["case_id"] / "flight.json"))
    return {"baseline_result": survey.projection(previous), "comparison": comparison,
            "endpoint": flight["ordinary_flight"]["final_state"] if previous["planning_stop"] == "correction_limit" else None}


def attempt(root, binary, row, limit, baseline, old_inventory, input_inventory):
    command = [str(binary), "waypoint-v2-flight", "--scenario", str(root / row["scenario_path"]),
               "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--output-dir", str(root / "runs" / row["attempt_id"])]
    started = time.monotonic()
    try:
        result = subprocess.run(command, capture_output=True, timeout=limit)
        stdout, stderr, code = result.stdout, result.stderr, result.returncode
        status = "recorded" if code == 0 else "runner_error"
    except subprocess.TimeoutExpired as e:
        stdout, stderr, code, status = e.stdout or b"", e.stderr or b"", None, "runner_timeout"
    except OSError as e:
        stdout, stderr, code, status = b"", str(e).encode(), None, "runner_error"
    study.write_new(root / "logs" / (row["attempt_id"] + ".stdout"), stdout)
    study.write_new(root / "logs" / (row["attempt_id"] + ".stderr"), stderr)
    actual = dict(row, status=status, exit_code=code, wall_s=time.monotonic() - started,
                  output_dir="runs/" + row["attempt_id"], result=None, paired=None, failure=None)
    if status == "recorded":
        try:
            out = root / actual["output_dir"]
            actual["result"] = p.validate_native(d.read(out / "flight.json"), d.read(out / "summary.json"), CAP)
            actual["paired"] = check_record(root, actual, baseline, old_inventory, input_inventory)
        except (ValueError, KeyError, TypeError, OSError) as e:
            actual.update(status="evidence_error", failure=str(e))
    return actual


def paired_summary(rows):
    primary = [r for r in rows if r["cohort"] == "random" and r["status"] == "recorded"]
    affected = [r for r in primary if r["paired"]["baseline_result"]["planning_stop"] == "correction_limit"]
    recipes = sorted({r["geometry"]["recipe_id"] for r in primary})
    return {"baseline_landings": sum(r["paired"]["baseline_result"]["verified_landing"] for r in primary),
            "preserved_landings": sum(r["paired"]["baseline_result"]["verified_landing"] and r["result"]["verified_landing"] for r in primary),
            "new_landings": [r["case_id"] for r in primary if not r["paired"]["baseline_result"]["verified_landing"] and r["result"]["verified_landing"]],
            "blocked_landings": sum(r["result"]["nominal_class"] == "blocked" and r["result"]["verified_landing"] for r in primary),
            "old_cap_outcomes": dict(sorted(Counter(r["result"]["planning_stop"] for r in affected).items())),
            "old_cap_cases": [{"case_id": r["case_id"], "result": r["result"], "endpoint": r["paired"]["endpoint"]} for r in affected],
            "maximum_actual_corrections": max((r["result"]["correction_count"] for r in primary), default=0),
            "per_recipe": {recipe: {"count": sum(r["geometry"]["recipe_id"] == recipe for r in primary),
                 "baseline_landings": sum(r["geometry"]["recipe_id"] == recipe and r["paired"]["baseline_result"]["verified_landing"] for r in primary),
                 "landings": sum(r["geometry"]["recipe_id"] == recipe and r["result"]["verified_landing"] for r in primary)} for recipe in recipes}}


def verify(root):
    root = root.resolve()
    plan = contract(root / "plan.json")
    manifest, report, receipt = (d.read(root / n) for n in ("manifest.json", "cap-sweep.json", "receipt.json"))
    baseline, old_manifest, old_inventory = baseline_inputs(plan)
    if (root / "plan.json").read_bytes() != PLAN.read_bytes() or survey.inventory(root) != receipt["files"]:
        raise ValueError("paired plan/complete receipt differs")
    survey.check_inventory(root, receipt["files"])
    source = manifest["source"]
    if (manifest["schema"] != "pd-lab.correction-cap-sweep-inputs.v1"
            or report["schema"] != "pd-lab.correction-cap-sweep-results.v1"
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or report["source_after"] != source or report["protected_after"] != manifest["protected"]
            or source["production_executable_sha256"] != plan["production_executable_sha256"]
            or source["files"].get(str(PLAN.relative_to(survey.REPO))) != study.digest((root / "plan.json").read_bytes())):
        raise ValueError("paired source/default executable/protected evidence drift")
    for relative, expected in source["files"].items():
        if study.digest(d.safe_file(root / "inputs/source", relative).read_bytes()) != expected:
            raise ValueError("paired source snapshot differs")
    for name in ("manifest.json", "receipt.json", "plan.json", "survey.json"):
        if (root / "baseline" / name).read_bytes() != (baseline / name).read_bytes():
            raise ValueError("paired baseline snapshot differs")
    original_cases = old_manifest["sentinel_cases"] + old_manifest["random_cases"]
    expected_inputs = {c["scenario_path"]: old_inventory[c["scenario_path"]] for c in original_cases}
    if manifest["inputs"] != expected_inputs:
        raise ValueError("paired input inventory differs")
    for relative, expected in expected_inputs.items():
        if study.digest(d.safe_file(root, relative).read_bytes()) != expected:
            raise ValueError("paired input changed")
    frozen = attempts(plan, old_manifest)
    if d.read(root / "run-start.json") != frozen or len(report["rows"]) != len(frozen):
        raise ValueError("paired population/order differs")
    if report["rows"] and any(r["status"] != "not_attempted" for r in report["rows"]):
        stage = root / "variants/cap-24"
        expected = dict(source["files"])
        expected[p.POLICY_PATH] = study.digest(p.transformed_policy((root / "inputs/source" / p.POLICY_PATH).read_bytes(), CAP))
        seal = d.read(stage / "source-seal.json")
        if seal["files"] != expected or p.variant_state(stage, expected) != seal:
            raise ValueError("paired probe source/binary differs")
    measured = 0
    for row, declared in zip(report["rows"], frozen):
        if any(row.get(k) != v for k, v in declared.items() if k != "status"):
            raise ValueError("paired row identity differs")
        if row["status"] not in ("not_attempted", "recorded", "runner_error", "runner_timeout", "evidence_error"):
            raise ValueError("unknown paired status")
        if row["status"] != "not_attempted":
            measured += 1
            if d.read(root / "ledger" / (row["attempt_id"] + ".json")) != row:
                raise ValueError("paired ledger differs")
        if row["status"] == "recorded":
            preflight = d.read(root / "preflight" / (row["case_id"] + ".json"))
            if preflight != {"supported": True, "rejection": None, "reason": None, "simulation_created": False}:
                raise ValueError("paired preflight differs")
            if check_record(root, row, baseline, old_inventory, expected_inputs) != row["paired"]:
                raise ValueError("paired observation differs")
        elif report["stopped_reason"] is None:
            raise ValueError("unaccounted paired attempt")
    if (measured != report["measured_attempts"] or measured > plan["maximum_measured_attempts"]
            or report["summary"] != survey.summary_for(report["rows"]) or report["paired_summary"] != paired_summary(report["rows"])):
        raise ValueError("paired totals/allowance differ")
    return report


def run(args):
    plan = contract()
    baseline, old_manifest, old_inventory = baseline_inputs(plan, complete=True)
    source, protected = p.main_state(), survey.protected_state()
    if source["production_executable_sha256"] != plan["production_executable_sha256"]:
        raise ValueError("ordinary evaluator changed before paired sweep")
    root = args.output.resolve()
    target = survey.REPO / "target" / ("cap-probe-" + root.name)
    if target.exists() or target.is_symlink():
        raise ValueError("paired build target must be new")
    survey.reserve(root)
    study.write_new(root / "plan.json", PLAN.read_bytes())
    for relative, expected in source["files"].items():
        data = d.safe_file(survey.REPO, relative).read_bytes()
        if study.digest(data) != expected:
            raise ValueError("source drift during paired preparation")
        study.write_new(root / "inputs/source" / relative, data)
    for name in ("manifest.json", "receipt.json", "plan.json", "survey.json"):
        study.write_new(root / "baseline" / name, (baseline / name).read_bytes())
    inputs = {}
    for case in old_manifest["sentinel_cases"] + old_manifest["random_cases"]:
        relative = case["scenario_path"]
        data = d.safe_file(baseline, relative).read_bytes()
        if study.digest(data) != old_inventory[relative]:
            raise ValueError("baseline input drift before paired sweep")
        study.write_new(root / relative, data)
        inputs[relative] = old_inventory[relative]
    manifest = {"schema": "pd-lab.correction-cap-sweep-inputs.v1", "source": source, "protected": protected, "inputs": inputs}
    study.write_new(root / "manifest.json", study.encoded(manifest))
    rows = attempts(plan, old_manifest)
    study.write_new(root / "run-start.json", study.encoded(rows))
    stopped, measured, elapsed = None, 0, 0.0
    try:
        print(json.dumps({"building_isolated_cap": CAP, "default_unchanged": 6, "measured_allowance": len(rows)}), flush=True)
        seal = p.build_variant(root, CAP, source, plan, target)
        stage = root / "variants/cap-24"
        binary = stage / "bin/pd-eval"
        for cid in inputs:
            preflight = survey.preflight(binary, root / cid)
            study.write_new(root / "preflight" / (Path(cid).stem + ".json"), study.encoded(preflight))
            if preflight != {"supported": True, "rejection": None, "reason": None, "simulation_created": False}:
                raise ValueError("paired preflight rejected: " + cid)
        started = time.monotonic()
        with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
            for wave in survey.waves_for(plan):
                if (p.main_state() != source or survey.protected_state() != protected
                        or p.variant_state(stage, source["files"]) != seal):
                    raise ValueError("paired source/executable/protected drift")
                remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - started)
                if remaining <= 0:
                    raise ValueError("paired campaign wall bound")
                futures = [(i, pool.submit(attempt, root, binary, rows[i], min(remaining, plan["case_wall_limit_s"]),
                                          baseline, old_inventory, inputs)) for i in wave]
                measured += len(wave)
                for index, future in futures:
                    rows[index] = future.result()
                    study.write_new(root / "ledger" / (rows[index]["attempt_id"] + ".json"), study.encoded(rows[index]))
                elapsed = time.monotonic() - started
                errors = [rows[i] for i in wave if rows[i]["status"] != "recorded"]
                if errors:
                    raise ValueError(errors[0]["attempt_id"] + ": " + (errors[0]["failure"] or errors[0]["status"]))
                if measured <= 13 or measured % 40 < len(wave) or measured == len(rows):
                    print(json.dumps({"measured": measured, "summary": survey.summary_for(rows), "wall_s": elapsed}), flush=True)
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as e:
        stopped = str(e)
    after, protected_after = p.main_state(), survey.protected_state()
    if after != source or protected_after != protected:
        stopped = stopped or "paired final source/default executable/protected drift"
    report = {"schema": "pd-lab.correction-cap-sweep-results.v1", "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "source_after": after, "protected_after": protected_after, "rows": rows,
              "measured_attempts": measured, "collection_wall_s": elapsed, "stopped_reason": stopped,
              "summary": survey.summary_for(rows), "paired_summary": paired_summary(rows)}
    study.write_new(root / "cap-sweep.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if stopped:
        raise RuntimeError("paired sweep stopped; partial evidence retained: " + stopped)
    verify(root)
    print(json.dumps({"verified": str(root), "summary": report["summary"], "paired_summary": report["paired_summary"]}), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    command = sub.add_parser("run")
    command.add_argument("--output", type=Path, required=True)
    command = sub.add_parser("verify")
    command.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    else:
        report = verify(args.capture)
        print(json.dumps({"verified": str(args.capture), "fresh_flights": 0, "completed": report["stopped_reason"] is None,
                          "measured_attempts": report["measured_attempts"], "summary": report["summary"]}))
