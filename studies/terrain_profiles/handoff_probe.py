"""Bounded counterfactual handoff probes, not a flight sweep or planner policy."""

import argparse
from collections import Counter
import json
from pathlib import Path
import subprocess
import time

import cap_sweep
import diagnostics as d
import study
import survey

PLAN = survey.HERE / "handoff_probe_plan.json"
PLAN_SHA256 = "fef2c150ac15fd1d35d1795b9a17ced02afac4c17873226d48fd517f5bc8ad0e"
CASES = ["random-983", "random-967", "random-955", "random-024", "random-516", "random-271", "random-000"]
DISPOSITIONS = {"no_nominal", "nominal_clear", "nominal_terrain_blocked", "nominal_other_rejected"}


def contract(path=PLAN):
    plan = d.read(path)
    if (study.digest(path.read_bytes()) != PLAN_SHA256 or plan != d.read(PLAN)
            or plan["schema"] != "pd-lab.terrain-handoff-probe-plan.v1"
            or [c["case_id"] for c in plan["cases"]] != CASES
            or [plan[k] for k in ("maximum_probes", "probe_wall_limit_s", "campaign_wall_limit_s", "workers")] != [19, 120, 900, 1]
            or plan["repeats"] != ["random-967-early", "random-983-early"]
            or plan["baseline_manifest_sha256"] != "33397ffe3d99187a44326ab921baf981b60d86397585c9983e49ac5edce04315"
            or plan["baseline_receipt_sha256"] != "51808ed5b564d210e27d4e971172e49a7d7f6d005cd0667adbbf25ae3635551b"):
        raise ValueError("changed handoff diagnostic contract")
    return plan


def attempts(plan):
    rows = []
    for arm in ("original", "early", "alternative"):
        for case in plan["cases"]:
            if arm != "original" and case["role"] != "failure":
                continue
            rows.append({"attempt_id": case["case_id"] + "-" + arm, "status": "not_attempted",
                         "cohort": "primary", "role": case["role"],
                         "spec": {"case_id": case["case_id"], "arm": arm,
                                  "scenario_sha256": case["scenario_sha256"], "flight_sha256": case["flight_sha256"],
                                  "cycle_index": case["cycle_index"],
                                  "row_id": case["alternative_row_id"] if arm == "alternative" else case["original_row_id"],
                                  "physics_step": case[arm + "_step"]}})
    by_id = {r["attempt_id"]: r for r in rows}
    rows += [dict(by_id[c], attempt_id="repeat-" + c, cohort="repeat") for c in plan["repeats"]]
    if len(rows) != plan["maximum_probes"]:
        raise ValueError("handoff diagnostic allowance differs")
    return rows


def baseline_inputs(plan, complete=False):
    root = survey.REPO / plan["baseline_capture"]
    for name, key in (("manifest.json", "baseline_manifest_sha256"), ("receipt.json", "baseline_receipt_sha256")):
        if study.digest(d.safe_file(root, name).read_bytes()) != plan[key]:
            raise ValueError("changed handoff baseline identity")
    if complete:
        report = cap_sweep.verify(root)
        if report["stopped_reason"] is not None or report["measured_attempts"] != 1010:
            raise ValueError("incomplete handoff baseline")
    inventory = d.read(root / "receipt.json")["files"]
    for case in plan["cases"]:
        for path_key, hash_key in (("scenario_path", "scenario_sha256"), ("flight_path", "flight_sha256")):
            data = d.safe_file(root, case[path_key]).read_bytes()
            if inventory[case[path_key]] != case[hash_key] or study.digest(data) != case[hash_key]:
                raise ValueError("changed handoff diagnostic input")
    return root


def protected_state():
    paths = dict(survey.protected_state())
    for relative in ("outputs/reports/eval/planner_v2_random_terrain/recheck-cap-sweep-20261007-v6",
                     "outputs/eval/planner_v2_random_terrain/capture-cap-sweep-20261007-v1",
                     "outputs/eval/planner_v2_random_terrain/capture-validation-1k-20261007-v1"):
        root = survey.REPO / relative
        if root.name.startswith("capture-"):
            paths[relative + "/receipt.json"] = study.digest((root / "receipt.json").read_bytes())
        else:
            paths.update({str(p.relative_to(survey.REPO)): study.digest(p.read_bytes())
                          for p in sorted(root.rglob("*")) if p.is_file()})
    for relative in ("fixtures/reports/report_navigation.json", "target/release/pd-eval"):
        paths[relative] = study.digest((survey.REPO / relative).read_bytes())
    return paths


def validate_probe(result, row):
    if (result["schema"] != "pd-lab.terrain-handoff-probe.v1" or result["request"] != row["spec"]
            or result["evidence_kind"] != "counterfactual_query" or result["executed_mission_landing_claimed"] is not False
            or result["prefix_source_replay_passed"] is not True
            or result["probe_state"]["physics_step"] != row["spec"]["physics_step"]
            or result["probe_state"]["sim_time_s"] != row["spec"]["physics_step"] / 120
            or result["absolute_deadline_physics_step"] != 9600
            or result["disposition"] not in DISPOSITIONS
            or result["original_result_matched"] != (row["spec"]["arm"] == "original")
            or result["production_boundary_admitted"] != (row["spec"]["arm"] != "early")):
        raise ValueError("invalid or relabeled handoff query")
    feasible = result["nominal_proposal_identity"] is not None
    if feasible != (result["disposition"] != "no_nominal") or (feasible and result["audit_passed"] is None):
        raise ValueError("nominal feasibility/audit contradiction")
    if result["disposition"] == "nominal_clear" and result["audit_passed"] is not True:
        raise ValueError("clear nominal lacks passed audit")
    if not feasible and (result["audit_passed"] is not None or result["first_terrain_violation"] is not None):
        raise ValueError("missing nominal was relabeled as terrain blockage")
    if row["spec"]["arm"] == "original":
        expected = "no_nominal" if row["role"] == "failure" else "nominal_clear"
        if result["disposition"] != expected:
            raise ValueError("original diagnostic/control result changed")
    return result


def check_record(root, row):
    output = d.safe_file(root, "runs/" + row["attempt_id"])
    if row["exit_code"] != 0 or row["output_dir"] != "runs/" + row["attempt_id"]:
        raise ValueError("handoff query runner/output differs")
    result = validate_probe(d.read(output / "probe.json"), row)
    if (result != row["result"] or d.read(root / "logs" / (row["attempt_id"] + ".stdout")) != result
            or d.read(output / "request.json") != row["spec"]):
        raise ValueError("handoff query stdout/spec/ledger differs")
    proof = d.read(output / "prefix-proof.json")
    if proof["failure"] is not None or proof["final_state"] != result["probe_state"]:
        raise ValueError("source replay does not reach recorded probe state")
    segments = d.read(output / "prefix-segments.json")
    updates = [u for s in segments for u in s["updates"]]
    actions = proof["run"]["actions"]
    if ([(u["physics_step"], u["command"]) for u in updates] != [(a["physics_step"], a["command"]) for a in actions]
            or [a["physics_step"] for a in actions] != list(range(0, row["spec"]["physics_step"], 2))):
        raise ValueError("source proof commands differ from exclusive prefix pieces")
    for index, segment in enumerate(segments):
        if (segment["start_physics_step"] != segment["entry_state"]["physics_step"]
                or segment["end_physics_step"] != segment["end_state"]["physics_step"]
                or [u["physics_step"] for u in segment["updates"]] != list(range(segment["start_physics_step"], segment["end_physics_step"], 2))
                or (index and segments[index - 1]["end_state"] != segment["entry_state"])):
            raise ValueError("prefix piece clock/state discontinuity")
    if (segments and segments[-1]["end_state"] != result["probe_state"]) or (not segments and row["spec"]["physics_step"] != 0):
        raise ValueError("prefix piece endpoint differs")
    baseline = d.read(root / "inputs" / row["spec"]["case_id"] / "flight.json")
    arm = row["spec"]["arm"]
    if arm != "alternative":
        old = [a for a in baseline["ordinary_flight"]["actions"] if a["physics_step"] < row["spec"]["physics_step"]]
        if actions != old:
            raise ValueError("same-maneuver query changed baseline prefix commands")
    if row["spec"]["case_id"] != "random-000":
        clearing = d.read(output / "clearing-query.json")
        cycle = baseline["cycles"][row["spec"]["cycle_index"]]
        if clearing["row_id"] != row["spec"]["row_id"] or clearing["goal"] != cycle["local_search"]["selected"]["goal"]:
            raise ValueError("clearing query row/goal differs")
        if arm != "alternative" and clearing != cycle["local_search"]["selected"]:
            raise ValueError("same selected clearing proposal differs")
        if arm == "alternative":
            eligible = [r for r in cycle["local_search"]["row_diagnostics"] if r.get("eligible_handoff")
                        and r["eligible_handoff"].get("braking_room")]
            chosen = min(eligible, key=lambda r: (-r["eligible_handoff"]["braking_room"]["remaining_room_m"], r["row_id"]))
            if (chosen["row_id"] != clearing["row_id"] or chosen["eligible_handoff"]["state"] != clearing["handoff_state"]
                    or chosen["eligible_handoff"]["actual_fuel_burn_to_handoff_kg"] != clearing["actual_fuel_burn_to_handoff_kg"]):
                raise ValueError("alternative is not the preselected accepted row")
        if arm != "early" and clearing["handoff_state"] != result["probe_state"]:
            raise ValueError("handoff query full state differs")
        if result["clearing_witness_physics_step"] != clearing["handoff_state"]["physics_step"]:
            raise ValueError("clearing witness clock differs")
    search = d.read(output / "nominal-search.json")
    if search["identity"] != result["nominal_search_identity"]:
        raise ValueError("nominal search is not bound to the diagnostic result")
    if row["spec"]["case_id"] != "random-000" and any(result["probe_state"].get(k) != v for k, v in search["incoming_state"].items()):
        raise ValueError("nominal incoming state differs from original-source probe state")
    selected = search["selected"]
    if (selected["identity"] if selected else None) != result["nominal_proposal_identity"]:
        raise ValueError("selected proposal is not bound to the diagnostic result")
    if selected:
        audit = d.read(output / "terrain-audit.json")
        if (audit["passed"] != result["audit_passed"] or not audit["ordinary_neutral_parity"]
                or (not audit["commands_match"] and result["disposition"] != "nominal_terrain_blocked")
                or audit["clearance_scan"]["first_violation"] != result["first_terrain_violation"]):
            raise ValueError("terrain audit is not bound to the diagnostic result")
    elif (output / "terrain-audit.json").exists():
        raise ValueError("invented terrain audit without a nominal")
    if row["cohort"] == "repeat":
        primary = d.read(root / "runs" / row["attempt_id"].removeprefix("repeat-") / "probe.json")
        if result != primary:
            raise ValueError("handoff diagnostic repeat differs")
        primary_dir = root / "runs" / row["attempt_id"].removeprefix("repeat-")
        for name in ("prefix-proof.json", "prefix-segments.json", "clearing-query.json", "nominal-search.json", "terrain-audit.json"):
            if (output / name).exists() != (primary_dir / name).exists() or ((output / name).exists()
                    and (output / name).read_bytes() != (primary_dir / name).read_bytes()):
                raise ValueError("complete repeated query evidence differs")
    return result


def summary(rows):
    return {"recorded_probes": sum(r["status"] == "recorded" for r in rows),
            "dispositions": dict(Counter(r["result"]["disposition"] for r in rows if r["status"] == "recorded")),
            "executed_mission_landings": 0, "new_sweep_worlds": 0}


def attempt(binary, root, row, limit):
    case = row["spec"]["case_id"]
    command = [str(binary), "probe-terrain-handoff", "--scenario", str(root / "inputs" / case / "scenario.json"),
               "--baseline-flight", str(root / "inputs" / case / "flight.json"),
               "--probe-spec", str(root / "specs" / (row["attempt_id"] + ".json")),
               "--output-dir", str(root / "runs" / row["attempt_id"])]
    started = time.monotonic()
    try:
        completed = subprocess.run(command, capture_output=True, timeout=limit)
        stdout, stderr, code = completed.stdout, completed.stderr, completed.returncode
        status = "recorded" if code == 0 else "runner_error"
    except subprocess.TimeoutExpired as error:
        stdout, stderr, code, status = error.stdout or b"", error.stderr or b"", None, "runner_timeout"
    except OSError as error:
        stdout, stderr, code, status = b"", str(error).encode(), None, "runner_error"
    study.write_new(root / "logs" / (row["attempt_id"] + ".stdout"), stdout)
    study.write_new(root / "logs" / (row["attempt_id"] + ".stderr"), stderr)
    actual = dict(row, status=status, exit_code=code, wall_s=time.monotonic() - started,
                  output_dir="runs/" + row["attempt_id"], result=None, failure=None)
    if status == "recorded":
        try:
            actual["result"] = d.read(root / actual["output_dir"] / "probe.json")
            check_record(root, actual)
        except (ValueError, KeyError, TypeError, OSError) as error:
            actual.update(status="evidence_error", failure=str(error))
    else:
        actual["failure"] = stderr.decode(errors="replace")[-2000:] or status
    return actual


def verify(root):
    root = root.resolve()
    plan = contract(root / "plan.json")
    manifest, report, receipt = (d.read(root / name) for name in ("manifest.json", "handoff-probes.json", "receipt.json"))
    if survey.inventory(root) != receipt["files"]:
        raise ValueError("handoff diagnostic inventory differs")
    survey.check_inventory(root, receipt["files"])
    if (manifest["schema"] != "pd-lab.terrain-handoff-probe-inputs.v1"
            or report["schema"] != "pd-lab.terrain-handoff-probe-results.v1"
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or report["source_after"] != manifest["source"] or report["protected_after"] != manifest["protected"]):
        raise ValueError("handoff diagnostic source/protection binding differs")
    if (root / "plan.json").read_bytes() != PLAN.read_bytes():
        raise ValueError("handoff diagnostic plan bytes differ")
    if manifest["source"]["files"][str(PLAN.relative_to(survey.REPO))] != study.digest(PLAN.read_bytes()):
        raise ValueError("handoff diagnostic plan is not source-bound")
    if study.digest(d.safe_file(root, "inputs/bin/pd-eval").read_bytes()) != manifest["source"]["executable_sha256"]:
        raise ValueError("handoff diagnostic executable differs")
    for relative, expected in manifest["source"]["files"].items():
        if study.digest(d.safe_file(root / "inputs/source", relative).read_bytes()) != expected:
            raise ValueError("handoff diagnostic source snapshot differs")
    for case in plan["cases"]:
        for name, key in (("scenario.json", "scenario_sha256"), ("flight.json", "flight_sha256")):
            if study.digest(d.safe_file(root, "inputs/" + case["case_id"] + "/" + name).read_bytes()) != case[key]:
                raise ValueError("handoff diagnostic retained input differs")
    for name, key in (("manifest.json", "baseline_manifest_sha256"), ("receipt.json", "baseline_receipt_sha256")):
        if study.digest((root / "baseline" / name).read_bytes()) != plan[key]:
            raise ValueError("handoff diagnostic retained baseline identity differs")
    frozen = attempts(plan)
    if d.read(root / "run-start.json") != frozen or len(report["rows"]) != len(frozen):
        raise ValueError("handoff diagnostic attempt order/count differs")
    for row, declared in zip(report["rows"], frozen):
        if any(row.get(k) != v for k, v in declared.items() if k != "status"):
            raise ValueError("handoff diagnostic attempt identity differs")
        if d.read(root / "specs" / (row["attempt_id"] + ".json")) != row["spec"]:
            raise ValueError("handoff diagnostic retained spec differs")
        if row["status"] not in {"not_attempted", "recorded", "runner_error", "runner_timeout", "evidence_error"}:
            raise ValueError("unknown handoff diagnostic status")
        if row["status"] != "not_attempted" and d.read(root / "ledger" / (row["attempt_id"] + ".json")) != row:
            raise ValueError("handoff diagnostic ledger differs")
        if row["status"] == "recorded":
            check_record(root, row)
        elif report["stopped_reason"] is None:
            raise ValueError("unaccounted handoff diagnostic attempt")
    measured = sum(r["status"] != "not_attempted" for r in report["rows"])
    if measured != report["measured_probes"] or measured > 19 or report["summary"] != summary(report["rows"]):
        raise ValueError("handoff diagnostic totals differ")
    return report


def run(args):
    plan = contract()
    baseline = baseline_inputs(plan, complete=True)
    root, binary = args.output.resolve(), args.binary.resolve()
    source, protected = survey.source_state(binary), protected_state()
    rows = attempts(plan)
    survey.reserve(root)
    study.write_new(root / "plan.json", PLAN.read_bytes())
    study.write_new(root / "inputs/bin/pd-eval", binary.read_bytes())
    for relative, expected in source["files"].items():
        data = d.safe_file(survey.REPO, relative).read_bytes()
        if study.digest(data) != expected:
            raise ValueError("source drift during handoff preparation")
        study.write_new(root / "inputs/source" / relative, data)
    for case in plan["cases"]:
        for name, key in (("scenario.json", "scenario_path"), ("flight.json", "flight_path")):
            study.write_new(root / "inputs" / case["case_id"] / name, d.safe_file(baseline, case[key]).read_bytes())
    for name in ("manifest.json", "receipt.json"):
        study.write_new(root / "baseline" / name, (baseline / name).read_bytes())
    for row in rows:
        study.write_new(root / "specs" / (row["attempt_id"] + ".json"), study.encoded(row["spec"]))
    manifest = {"schema": "pd-lab.terrain-handoff-probe-inputs.v1", "source": source, "protected": protected}
    study.write_new(root / "manifest.json", study.encoded(manifest))
    study.write_new(root / "run-start.json", study.encoded(rows))
    started, stopped = time.monotonic(), None
    for i, row in enumerate(rows):
        if survey.source_state(binary) != source or protected_state() != protected:
            stopped = "handoff diagnostic source/executable/protected drift"
            break
        remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - started)
        if remaining <= 0:
            stopped = "handoff diagnostic campaign wall bound"
            break
        rows[i] = attempt(binary, root, row, min(remaining, plan["probe_wall_limit_s"]))
        study.write_new(root / "ledger" / (row["attempt_id"] + ".json"), study.encoded(rows[i]))
        print(json.dumps({"attempt": row["attempt_id"], "status": rows[i]["status"],
                          "disposition": rows[i]["result"]["disposition"] if rows[i]["result"] else None,
                          "failure": rows[i]["failure"]}), flush=True)
        if rows[i]["status"] != "recorded":
            stopped = row["attempt_id"] + ": " + rows[i]["failure"]
            break
    after, protected_after = survey.source_state(binary), protected_state()
    if after != source or protected_after != protected:
        stopped = stopped or "handoff diagnostic final source/executable/protected drift"
    report = {"schema": "pd-lab.terrain-handoff-probe-results.v1",
              "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "source_after": after, "protected_after": protected_after,
              "stopped_reason": stopped, "measured_probes": sum(r["status"] != "not_attempted" for r in rows),
              "collection_wall_s": time.monotonic() - started, "rows": rows, "summary": summary(rows)}
    study.write_new(root / "handoff-probes.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if stopped:
        raise RuntimeError("handoff diagnostic stopped; partial evidence retained: " + stopped)
    verify(root)
    print(json.dumps({"verified": str(root), "summary": report["summary"]}), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    command = sub.add_parser("run")
    command.add_argument("--binary", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command = sub.add_parser("verify")
    command.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    else:
        result = verify(args.capture)
        print(json.dumps({"verified": str(args.capture), "summary": result["summary"],
                          "stopped_reason": result["stopped_reason"], "fresh_probes": 0}))
