"""Frozen paired 1k early-exit measurement; no tuning, retries or publication."""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import time

import cap_probe as cap
import cap_sweep
import diagnostics as d
import early_exit as early
import handoff_probe as handoff
import study
import survey

PLAN = survey.HERE / "early_exit_sweep_plan.json"
PLAN_SHA256 = "006b229a7c67e7b72bcbd504b36f661d5eac0050d5c9dd68c8b480c60fa5e810"


def contract(path=PLAN):
    if study.digest(path.read_bytes()) != PLAN_SHA256:
        raise ValueError("changed frozen early-exit 1k contract")
    return d.read(path)


def baseline_inputs(plan):
    root = survey.REPO / plan["baseline_capture"]
    for name, key in (("manifest.json", "baseline_manifest_sha256"), ("receipt.json", "baseline_receipt_sha256")):
        if study.digest(d.safe_file(root, name).read_bytes()) != plan[key]:
            raise ValueError("changed paired cap-24 baseline")
    inventory = d.read(root / "receipt.json")["files"]
    report = cap_sweep.bound_read(root, "cap-sweep.json", inventory)
    population = cap_sweep.bound_read(root, "baseline/manifest.json", inventory)
    if (report["stopped_reason"] is not None or report["measured_attempts"] != 1010
            or report["summary"]["verified_landings"] != plan["baseline_landings"]):
        raise ValueError("baseline population is incomplete or has changed")
    return root, population, inventory


def validate_native(flight, compact, plan):
    policy = {"policy_id": plan["candidate_policy_id"], "maximum_corrections": plan["maximum_corrections"]}
    keys = {"planning_stop", "reason", "correction_count", "initial_nominal_terrain_blocked", "integrity_passed",
            "physical_outcome", "mission_outcome", "final_source_replay_passed", "timings"}
    if (flight["policy"] != policy or compact["policy"] != policy
            or compact["schema_id"] != "waypoint_v2_flight_summary_v1"
            or not flight["input_identity"] or compact["input_identity"] != flight["input_identity"]
            or set(compact["result"]) != keys or any(compact["result"][k] != flight[k] for k in keys)
            or flight["integrity_passed"] is not True or flight["final_source_replay_passed"] is not True):
        raise ValueError("native policy/result/integrity/replay failure")
    survey.non_timing(flight)
    final = flight["ordinary_flight"]["final_state"]
    if (flight["physical_outcome"] != final["physical_outcome"] or flight["mission_outcome"] != final["mission_outcome"]
            or final["physics_step"] > flight["absolute_deadline_physics_step"]
            or flight["correction_count"] > plan["maximum_corrections"]
            or flight["correction_count"] != sum(s["kind"] == "local_correction" for s in flight["segments"])):
        raise ValueError("native physical/clock/correction inventory differs")
    result = survey.projection(flight)
    stop = flight["planning_stop"]
    if stop == "landed":
        if not result["verified_landing"]:
            raise ValueError("incomplete landing tuple")
    elif stop in ("no_clearing", "no_nominal", "correction_limit", "nominal_rejected", "deadline"):
        running = flight["physical_outcome"] == "flying" and flight["mission_outcome"] == "in_progress"
        timeout = stop == "deadline" and flight["physical_outcome"] == "timed_out" and flight["mission_outcome"] == "failed_timeout"
        if not running and not timeout:
            raise ValueError("unexpected physical failure")
    else:
        raise ValueError("unexpected planner failure")
    if stop == "correction_limit" and flight["correction_count"] != plan["maximum_corrections"]:
        raise ValueError("nonbinding correction limit")
    return result


def exit_records(flight):
    records = []
    for cycle in flight["cycles"]:
        local = cycle["local_search"]
        if not local or not local.get("early_exit"):
            continue
        record, selected = local["early_exit"], local["selected"]
        if record["witness_handoff_physics_step"] != selected["schedule"]["handoff_physics_step"]:
            raise ValueError("early original witness differs")
        boundaries = [s["state"] for s in selected["trajectory"] if s["state"]["physics_step"] > selected["schedule"]["powered_end_physics_step"]
                      and s["state"]["physics_step"] % 2 == 0 and s["state"]["held_command"]["throttle_frac"] == 0
                      and s["state"]["held_command"]["target_attitude_rad"] == 0
                      and s["state"]["attitude_rad"] == 0 and s["state"]["angular_rate_radps"] == 0]
        expected = boundaries[0] if boundaries and boundaries[0]["physics_step"] < record["witness_handoff_physics_step"] else None
        if record["query_state"] != expected or (expected is None and record["disposition"] != "no_earlier_boundary"):
            raise ValueError("query is not the first eligible earlier boundary")
        if record["disposition"] == "committed":
            if expected is None or not local["handoff_source_replay_passed"] or not local["certificate_source_replay_passed"]:
                raise ValueError("unproved early commit")
            search, audit = record["nominal_search"], record["audit"]
            nominal = search["selected"]
            if not audit["passed"] or not audit["safe_target_contact"] or not audit["ordinary_neutral_parity"] or not audit["commands_match"]:
                raise ValueError("early commit lacks clear audit")
            nxt = flight["cycles"][cycle["cycle_index"] + 1]
            segment = next(s for s in flight["segments"] if s["kind"] == "local_correction" and s["start_physics_step"] == selected["entry_state"]["physics_step"])
            if (nxt["current_state"] != expected or nxt["nominal_search_identity"] != search["identity"]
                    or nxt["nominal_proposal_identity"] != nominal["identity"] or nxt["nominal_updates"] != nominal["updates"]
                    or nxt["audit"] != audit or segment["end_state"] != expected
                    or segment["updates"] != [u for u in selected["schedule"]["updates"] if u["physics_step"] < expected["physics_step"]]
                    or local["certificate_state"] != record["continuation_end_state"]):
                raise ValueError("actual early prefix/exact queued nominal differs")
        records.append(record)
    return records


def compare_flight(actual, previous):
    records = exit_records(actual)
    commits = [e for e in records if e["disposition"] == "committed"]
    if len(commits) > 1:
        raise ValueError("multiple committed exits despite checked direct landing")
    if not commits:
        early.unchanged(actual, previous)
        return {"kind": "complete_except_root_policy_identity_timings_optional_query", "early_dispositions": [e["disposition"] for e in records]}
    query = commits[0]
    cycle = next(c for c in actual["cycles"] if c["local_search"] and c["local_search"].get("early_exit") == query)
    i = cycle["cycle_index"]
    if (actual["absolute_deadline_physics_step"] != previous["absolute_deadline_physics_step"]
            or cycle["local_search"]["selected"] != previous["cycles"][i]["local_search"]["selected"]):
        raise ValueError("original selected row/proposal/deadline changed")
    for j in range(i + 1):
        a, b = json.loads(json.dumps(actual["cycles"][j])), json.loads(json.dumps(previous["cycles"][j]))
        for obj in (a, b):
            if obj["local_search"]:
                obj["local_search"].pop("early_exit", None)
                if j == i:
                    for key in ("certificate_state", "handoff_source_replay_passed", "certificate_source_replay_passed"):
                        obj["local_search"].pop(key)
        if a != b:
            raise ValueError("planning prefix before first committed exit changed")
    tick = query["query_state"]["physics_step"]
    for key in ("actions", "events", "samples"):
        prefix = lambda f: [s for s in f["ordinary_flight"][key] if s["physics_step"] < tick or (key == "samples" and s["physics_step"] == tick)]
        if prefix(actual) != prefix(previous):
            raise ValueError("ordinary early prefix differs: " + key)
    if not survey.projection(actual)["verified_landing"]:
        raise ValueError("committed clear nominal did not land")
    return {"kind": "exact_selected_proposal_and_ordinary_prefix_to_early_exit", "through_physics_step": tick,
            "witness_physics_step": query["witness_handoff_physics_step"], "early_dispositions": [e["disposition"] for e in records]}


def check_record(root, row, baseline, inventory, plan):
    if row["exit_code"] != 0 or row["output_dir"] != "runs/" + row["attempt_id"]:
        raise ValueError("runner/output identity differs")
    output = root / row["output_dir"]
    flight = d.read(output / "flight.json")
    result = validate_native(flight, d.read(output / "summary.json"), plan)
    # Input bytes remain receipt-pinned; Rust's typed JSON output may differ in
    # whitespace or number spelling. Compare every parsed value, no tolerance.
    if result != row["result"] or not same_scenario((output / "scenario.json").read_bytes(), (root / row["scenario_path"]).read_bytes()):
        raise ValueError("ledger or complete scenario values differ")
    printed = d.read(root / "logs" / (row["attempt_id"] + ".stdout"))
    if any(printed.get(k) != flight[k] for k in ("planning_stop", "correction_count", "integrity_passed", "final_source_replay_passed", "physical_outcome", "mission_outcome", "reason", "timings")):
        raise ValueError("CLI stdout differs")
    previous = cap_sweep.bound_read(baseline, "runs/" + row["case_id"] + "/flight.json", inventory)
    comparison = compare_flight(flight, previous)
    if row["cohort"] == "repeat" and survey.non_timing(flight) != survey.non_timing(d.read(root / "runs" / row["case_id"] / "flight.json")):
        raise ValueError("exact numerical repeat differs")
    return {"baseline_result": survey.projection(previous), "comparison": comparison}


def same_scenario(native_bytes, input_bytes):
    return json.loads(native_bytes) == json.loads(input_bytes)


def paired_summary(rows):
    cases = [r for r in rows if r["cohort"] == "random" and r["status"] == "recorded"]
    recipes = sorted({r["geometry"]["recipe_id"] for r in cases})
    return {"baseline_landings": sum(r["paired"]["baseline_result"]["verified_landing"] for r in cases),
            "preserved_landings": sum(r["paired"]["baseline_result"]["verified_landing"] and r["result"]["verified_landing"] for r in cases),
            "new_landings": [r["case_id"] for r in cases if not r["paired"]["baseline_result"]["verified_landing"] and r["result"]["verified_landing"]],
            "lost_landings": [r["case_id"] for r in cases if r["paired"]["baseline_result"]["verified_landing"] and not r["result"]["verified_landing"]],
            "blocked_landings": sum(r["result"]["nominal_class"] == "blocked" and r["result"]["verified_landing"] for r in cases),
            "early_dispositions": dict(Counter(x for r in cases for x in r["paired"]["comparison"]["early_dispositions"])),
            "early_exit_flights": sum("committed" in r["paired"]["comparison"]["early_dispositions"] for r in cases),
            "maximum_actual_corrections": max((r["result"]["correction_count"] for r in cases), default=0),
            "per_recipe": {p: {"count": sum(r["geometry"]["recipe_id"] == p for r in cases),
                               "baseline_landings": sum(r["geometry"]["recipe_id"] == p and r["paired"]["baseline_result"]["verified_landing"] for r in cases),
                               "landings": sum(r["geometry"]["recipe_id"] == p and r["result"]["verified_landing"] for r in cases)} for p in recipes}}


def attempt(root, row, baseline, inventory, plan, limit):
    command = [str(root / "candidate/bin/pd-eval"), "waypoint-v2-flight", "--scenario", str(root / row["scenario_path"]),
               "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--output-dir", str(root / "runs" / row["attempt_id"])]
    start = time.monotonic()
    try:
        completed = subprocess.run(command, capture_output=True, timeout=limit)
        stdout, stderr, code = completed.stdout, completed.stderr, completed.returncode
        status = "recorded" if code == 0 else "runner_error"
    except subprocess.TimeoutExpired as e:
        stdout, stderr, code, status = e.stdout or b"", e.stderr or b"", None, "runner_timeout"
    except OSError as e:
        stdout, stderr, code, status = b"", str(e).encode(), None, "runner_error"
    study.write_new(root / "logs" / (row["attempt_id"] + ".stdout"), stdout)
    study.write_new(root / "logs" / (row["attempt_id"] + ".stderr"), stderr)
    row = dict(row, status=status, exit_code=code, wall_s=time.monotonic() - start, output_dir="runs/" + row["attempt_id"], result=None, paired=None, failure=None)
    if status == "recorded":
        try:
            output = root / row["output_dir"]
            row["result"] = validate_native(d.read(output / "flight.json"), d.read(output / "summary.json"), plan)
            row["paired"] = check_record(root, row, baseline, inventory, plan)
        except (ValueError, KeyError, TypeError, OSError, IndexError, StopIteration) as e:
            row.update(status="evidence_error", failure=str(e))
    return row


def verify(root):
    plan = contract(root / "plan.json")
    manifest, report, receipt = (d.read(root / n) for n in ("manifest.json", "early-exit-sweep.json", "receipt.json"))
    survey.check_inventory(root, receipt["files"])
    if survey.inventory(root) != receipt["files"]:
        raise ValueError("closed sweep inventory differs")
    baseline, population, old_inventory = baseline_inputs(plan)
    source = manifest["operator_source"]
    if (manifest["schema"] != "pd-lab.early-exit-sweep-inputs.v1"
            or report["schema"] != "pd-lab.early-exit-sweep-results.v1"
            or report["source_after"] != source or report["protected_after"] != manifest["protected"]
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or source["production_executable_sha256"] != plan["production_executable_sha256"]
            or source["files"].get(str(PLAN.relative_to(survey.REPO))) != PLAN_SHA256):
        raise ValueError("source/protected drift")
    for path, digest in source["files"].items():
        if study.digest(d.safe_file(root / "inputs/operator_source", path).read_bytes()) != digest:
            raise ValueError("operator source snapshot differs")
    seal = d.read(root / "candidate/source-seal.json")
    if (cap.variant_state(root / "candidate", seal["files"]) != seal or seal["executable_sha256"] != plan["candidate_executable_sha256"]
            or study.digest((root / "candidate/source-seal.json").read_bytes()) != manifest["candidate_source_seal_sha256"]):
        raise ValueError("retained candidate source/binary differs")
    for name, key in (("manifest.json", "candidate_manifest_sha256"), ("receipt.json", "candidate_receipt_sha256")):
        if study.digest((root / "candidate" / name).read_bytes()) != plan[key]:
            raise ValueError("candidate capture binding differs")
    candidate_receipt = d.read(root / "candidate/receipt.json")["files"]
    if study.digest((root / "candidate/source-seal.json").read_bytes()) != candidate_receipt["variant/source-seal.json"]:
        raise ValueError("unbound candidate source seal")
    frozen = cap_sweep.attempts(plan, population)
    if d.read(root / "run-start.json") != frozen or len(report["rows"]) != len(frozen):
        raise ValueError("population/ordered attempt inventory differs")
    for row, declared in zip(report["rows"], frozen):
        if any(row.get(k) != v for k, v in declared.items() if k != "status"):
            raise ValueError("attempt identity differs")
        if row["status"] not in ("not_attempted", "recorded", "runner_error", "runner_timeout", "evidence_error"):
            raise ValueError("unknown attempt status")
        data = d.safe_file(root, row["scenario_path"]).read_bytes()
        if study.digest(data) != old_inventory[row["scenario_path"]]:
            raise ValueError("changed frozen scenario")
        if row["status"] != "not_attempted" and d.read(root / "ledger" / (row["attempt_id"] + ".json")) != row:
            raise ValueError("attempt ledger differs")
        if row["status"] == "recorded" and check_record(root, row, baseline, old_inventory, plan) != row["paired"]:
            raise ValueError("paired comparison differs")
    if (report["measured_attempts"] != sum(r["status"] != "not_attempted" for r in report["rows"])
            or report["summary"] != survey.summary_for(report["rows"])
            or report["paired_summary"] != paired_summary(report["rows"])
            or (report["stopped_reason"] is None and any(r["status"] != "recorded" for r in report["rows"]))):
        raise ValueError("collection completeness/summary differs")
    imported = report.get("imported_attempts", 0)
    if report.get("fresh_attempts", report["measured_attempts"]) + imported != report["measured_attempts"]:
        raise ValueError("fresh/imported attempt accounting differs")
    if imported:
        admission = d.read(root / "continuation/admission.json")
        original_receipt = d.read(root / "continuation/receipt.json")["files"]
        original_report = d.read(root / "continuation/early-exit-sweep.json")
        original_manifest = d.read(root / "continuation/manifest.json")
        if (admission["imported_attempts"] != imported or original_report["measured_attempts"] != imported
                or admission["new_flights_in_import"] != 0
                or admission["operator_source_before"] != original_manifest["operator_source"]
                or admission["operator_source_after"] != source
                or study.digest((root / "continuation/receipt.json").read_bytes()) != admission["original_receipt_sha256"]
                or study.digest((root / "continuation/manifest.json").read_bytes()) != original_report["manifest_sha256"]):
            raise ValueError("continuation original-source/receipt accounting differs")
        for name in ("manifest.json", "early-exit-sweep.json", "plan.json"):
            if study.digest((root / "continuation" / name).read_bytes()) != original_receipt[name]:
                raise ValueError("copied original capture metadata differs")
        for path, digest in original_manifest["operator_source"]["files"].items():
            if study.digest(d.safe_file(root / "continuation/operator_source", path).read_bytes()) != digest:
                raise ValueError("original collector source snapshot differs")
        for i, row in enumerate(report["rows"]):
            if ("evidence_import" in row) != (i < imported):
                raise ValueError("import is not the exact attempted prefix")
            if i >= imported:
                continue
            old = d.read(root / "continuation/ledger" / (row["attempt_id"] + ".json"))
            if (old != original_report["rows"][i] or row["evidence_import"] != {"original_status": old["status"], "revalidated_without_execution": True}
                    or study.digest((root / "continuation/ledger" / (row["attempt_id"] + ".json")).read_bytes()) != original_receipt["ledger/" + row["attempt_id"] + ".json"]):
                raise ValueError("original failure ledger or revalidation claim differs")
            for path, digest in original_receipt.items():
                if path.startswith(row["output_dir"] + "/") or path in ("logs/" + row["attempt_id"] + ".stdout", "logs/" + row["attempt_id"] + ".stderr"):
                    if study.digest(d.safe_file(root, path).read_bytes()) != digest:
                        raise ValueError("imported raw flight evidence changed")
    return report


def run(args):
    plan = contract()
    baseline, population, inventory = baseline_inputs(plan)
    original = survey.REPO / plan["candidate_capture"]
    for name, key in (("manifest.json", "candidate_manifest_sha256"), ("receipt.json", "candidate_receipt_sha256")):
        if study.digest((original / name).read_bytes()) != plan[key]:
            raise ValueError("candidate admission changed")
    admitted = early.verify(original)
    if admitted["stopped_reason"] is not None or admitted["measured_missions"] != 9:
        raise ValueError("candidate admission incomplete")
    source, protected = cap.main_state(), handoff.protected_state()
    seal = d.read(original / "variant/source-seal.json")
    before = d.read(original / "manifest.json")["source"]["files"]
    numerical_paths = lambda files: {p for p in files if p.endswith((".rs", ".toml", ".lock"))}
    if numerical_paths(before) != numerical_paths(source["files"]):
        raise ValueError("Rust/build file inventory differs from tested candidate")
    for path, digest in before.items():
        if path.endswith((".rs", ".toml", ".lock")) and source["files"].get(path) != digest:
            raise ValueError("Rust/build inputs differ from tested candidate")
    if source["production_executable_sha256"] != plan["production_executable_sha256"] or seal["executable_sha256"] != plan["candidate_executable_sha256"]:
        raise ValueError("production/candidate binary differs")
    root = args.output.resolve()
    survey.reserve(root)
    study.write_new(root / "plan.json", PLAN.read_bytes())
    for path, digest in source["files"].items():
        data = (survey.REPO / path).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source preparation drift")
        study.write_new(root / "inputs/operator_source" / path, data)
    for path in seal["files"]:
        study.write_new(root / "candidate/source" / path, d.safe_file(original / "variant/source", path).read_bytes())
    for name in ("manifest.json", "receipt.json"):
        study.write_new(root / "candidate" / name, (original / name).read_bytes())
    study.write_new(root / "candidate/source-seal.json", (original / "variant/source-seal.json").read_bytes())
    study.write_new(root / "candidate/bin/pd-eval", (original / "variant/bin/pd-eval").read_bytes())
    (root / "candidate/bin/pd-eval").chmod(0o755)
    if cap.variant_state(root / "candidate", seal["files"]) != seal:
        raise ValueError("candidate copy differs before missions")
    for case in population["sentinel_cases"] + population["random_cases"]:
        path = case["scenario_path"]
        data = d.safe_file(baseline, path).read_bytes()
        if study.digest(data) != inventory[path]:
            raise ValueError("changed scenario during copying")
        study.write_new(root / path, data)
    study.write_new(root / "manifest.json", study.encoded({"schema": "pd-lab.early-exit-sweep-inputs.v1", "operator_source": source, "protected": protected,
                                                         "candidate_source_seal_sha256": study.digest((root / "candidate/source-seal.json").read_bytes())}))
    rows = cap_sweep.attempts(plan, population)
    study.write_new(root / "run-start.json", study.encoded(rows))
    imported = 0
    if args.continue_from is not None:
        previous_root = args.continue_from.resolve()
        previous_receipt = d.read(previous_root / "receipt.json")
        survey.check_inventory(previous_root, previous_receipt["files"])
        if survey.inventory(previous_root) != previous_receipt["files"] or (previous_root / "plan.json").read_bytes() != PLAN.read_bytes():
            raise ValueError("continuation capture receipt/plan differs")
        previous = d.read(previous_root / "early-exit-sweep.json")
        previous_manifest = d.read(previous_root / "manifest.json")
        if (previous["source_after"] != previous_manifest["operator_source"]
                or previous["manifest_sha256"] != study.digest((previous_root / "manifest.json").read_bytes())
                or previous["protected_after"] != protected
                or study.digest((previous_root / "candidate/bin/pd-eval").read_bytes()) != plan["candidate_executable_sha256"]):
            raise ValueError("continuation source/binary/protected binding differs")
        old_source = previous_manifest["operator_source"]["files"]
        if any(old_source[p] != source["files"].get(p) for p in numerical_paths(old_source)):
            raise ValueError("continuation changed numerical source")
        for name in ("manifest.json", "receipt.json", "early-exit-sweep.json", "plan.json"):
            study.write_new(root / "continuation" / name, (previous_root / name).read_bytes())
        for path in old_source:
            study.write_new(root / "continuation/operator_source" / path, d.safe_file(previous_root / "inputs/operator_source", path).read_bytes())
        for i, old in enumerate(previous["rows"]):
            if old["status"] == "not_attempted":
                continue
            if i != imported or any(old.get(k) != v for k, v in rows[i].items() if k != "status"):
                raise ValueError("continuation is not the exact ordered attempted prefix")
            if old["status"] not in ("recorded", "evidence_error") or (old["status"] == "evidence_error" and old["failure"] != "ledger or exact scenario differs"):
                raise ValueError("continuation cannot repair physical/proof/runner failures")
            for file in sorted((previous_root / old["output_dir"]).rglob("*")):
                if file.is_file():
                    study.write_new(root / file.relative_to(previous_root), file.read_bytes())
            for suffix in ("stdout", "stderr"):
                path = "logs/" + old["attempt_id"] + "." + suffix
                study.write_new(root / path, (previous_root / path).read_bytes())
            study.write_new(root / "continuation/ledger" / (old["attempt_id"] + ".json"), study.encoded(old))
            value = dict(old, status="recorded", failure=None, evidence_import={"original_status": old["status"], "revalidated_without_execution": True})
            output = root / value["output_dir"]
            value["result"] = validate_native(d.read(output / "flight.json"), d.read(output / "summary.json"), plan)
            value["paired"] = check_record(root, value, baseline, inventory, plan)
            rows[i] = value
            study.write_new(root / "ledger" / (value["attempt_id"] + ".json"), study.encoded(value))
            imported += 1
        if imported != previous["measured_attempts"]:
            raise ValueError("continuation attempt count differs")
        study.write_new(root / "continuation/admission.json", study.encoded({"original_capture": str(previous_root.relative_to(survey.REPO)),
                          "original_receipt_sha256": study.digest((previous_root / "receipt.json").read_bytes()), "imported_attempts": imported,
                          "numerical_source_and_binary_unchanged": True, "operator_source_before": previous_manifest["operator_source"],
                          "operator_source_after": source, "new_flights_in_import": 0}))
    measured, stopped, start = 0, None, time.monotonic()
    measured = imported
    try:
        with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
            for wave in survey.waves_for(plan):
                wave = [i for i in wave if i >= imported]
                if not wave:
                    continue
                if cap.main_state() != source or handoff.protected_state() != protected or study.digest((root / "candidate/bin/pd-eval").read_bytes()) != seal["executable_sha256"]:
                    raise ValueError("live source/binary/protected drift")
                if cap.variant_state(root / "candidate", seal["files"]) != seal:
                    raise ValueError("copied candidate source drift")
                remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - start)
                if remaining <= 0:
                    raise ValueError("campaign wall bound")
                futures = [(i, pool.submit(attempt, root, rows[i], baseline, inventory, plan, min(remaining, plan["case_wall_limit_s"]))) for i in wave]
                measured += len(wave)
                for i, future in futures:
                    rows[i] = future.result()
                    study.write_new(root / "ledger" / (rows[i]["attempt_id"] + ".json"), study.encoded(rows[i]))
                errors = [rows[i] for i in wave if rows[i]["status"] != "recorded"]
                if errors:
                    raise ValueError(errors[0]["attempt_id"] + ": " + (errors[0]["failure"] or errors[0]["status"]))
                if measured <= 13 or measured % 40 < len(wave) or measured == len(rows):
                    paired = paired_summary(rows)
                    print(json.dumps({"measured": measured, "summary": survey.summary_for(rows), "new_landings": len(paired["new_landings"]), "lost_landings": len(paired["lost_landings"]), "wall_s": time.monotonic() - start}), flush=True)
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        stopped = str(error)
    after, protected_after = cap.main_state(), handoff.protected_state()
    if after != source or protected_after != protected:
        stopped = stopped or "final source/protected drift"
    report = {"schema": "pd-lab.early-exit-sweep-results.v1", "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "source_after": after, "protected_after": protected_after, "rows": rows, "measured_attempts": measured,
              "imported_attempts": imported, "fresh_attempts": measured - imported,
              "collection_wall_s": time.monotonic() - start, "stopped_reason": stopped, "summary": survey.summary_for(rows), "paired_summary": paired_summary(rows)}
    study.write_new(root / "early-exit-sweep.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if stopped:
        raise RuntimeError("sweep stopped; partial evidence retained: " + stopped)
    verify(root)
    print(json.dumps({"verified": str(root), "summary": report["summary"], "paired_summary": report["paired_summary"]}), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    collect = sub.add_parser("run")
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--continue-from", type=Path)
    check = sub.add_parser("verify")
    check.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    else:
        result = verify(args.capture.resolve())
        print(json.dumps({"measured_attempts": result["measured_attempts"], "stopped_reason": result["stopped_reason"], "summary": result["summary"], "paired_summary": result["paired_summary"]}))
