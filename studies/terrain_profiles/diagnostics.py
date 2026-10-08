"""Opt-in characterization of ten frozen worlds; never select or tune flights."""

import argparse
import json
from pathlib import Path
import time

import study
import survey

PLAN = survey.HERE / "diagnostics_plan.json"
FIXTURES = "fixtures/research/terrain_diagnostics_v1"
SELECTION = [
    ("departure", "failure", "random-327"),
    ("departure", "failure", "random-791"),
    ("departure", "comparison", "random-258"),
    ("acquisition", "failure", "random-024"),
    ("acquisition", "failure", "random-516"),
    ("acquisition", "comparison", "random-271"),
    ("progress", "failure", "random-030"),
    ("progress", "failure", "random-280"),
    ("progress", "comparison", "random-253"),
    ("direct", "comparison", "random-000"),
]
REPEATS = ["random-327", "random-024", "random-030"]


def read(path):
    return json.loads(path.read_bytes())


def fingerprint(flight):
    """Hash every field except the three declared finite wall timings."""
    return study.digest(study.encoded(survey.non_timing(flight)))


def safe_file(root, relative):
    path = Path(relative)
    if path.is_absolute() or not path.parts or any(p in ("..", ".") for p in path.parts):
        raise ValueError("unsafe diagnostic path")
    resolved = root / path
    if resolved.is_symlink() or not resolved.resolve().is_relative_to(root.resolve()):
        raise ValueError("diagnostic path escapes root")
    return resolved


def load_contract(path=PLAN):
    plan = read(path)
    if (plan["schema"] != "pd-lab.terrain-diagnostics-plan.v1"
            or [(c["family"], c["role"], c["case_id"]) for c in plan["cases"]] != SELECTION
            or plan["repeat_case_ids"] != REPEATS
            or [plan[k] for k in ("maximum_measured_attempts", "case_wall_limit_s", "campaign_wall_limit_s", "workers", "policy_version")]
            != [13, 120, 900, 1, 3]
            or plan["baseline"]["manifest_sha256"] != "c0a844f772f45dd2e39e362622e81e4cf5ceb04831925cdc492dd3e4989e2ee8"
            or plan["baseline"]["receipt_sha256"] != "0ae5032ac69f4428f3e79649deafed76dfefedeca1139dd4bbdc3e8fc5b22f21"):
        raise ValueError("changed bounded diagnostic contract")
    for case in plan["cases"]:
        if case["scenario_path"] != f'{FIXTURES}/{case["case_id"]}.json':
            raise ValueError("changed diagnostic scenario identity")
        for key in ("scenario_sha256", "baseline_flight_sha256", "baseline_non_timing_sha256"):
            value = case[key]
            if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
                raise ValueError("invalid diagnostic hash")
    return plan


def check_inputs(plan, root=survey.REPO):
    for case in plan["cases"]:
        path = safe_file(root, case["scenario_path"])
        if study.digest(path.read_bytes()) != case["scenario_sha256"]:
            raise ValueError("changed frozen diagnostic input: " + case["case_id"])
        scenario = read(path)
        if (scenario["metadata"]["recipe"] != case["recipe_id"]
                or scenario["id"] != case["scenario_id"]
                or scenario["id"] != f'challenge_{case["recipe_id"]}_seed_{case["seed"]}'):
            raise ValueError("diagnostic source identity differs")


def observations(flight, scenario):
    """Compact explanations, derived from full evidence; no feasibility oracle."""
    target = next(p for p in scenario["world"]["landing_pads"] if p["id"] == scenario["mission"]["goal"]["target_pad_id"])
    cycles = flight["cycles"]
    handoffs = []
    previous_x = scenario["initial_state"]["position_m"]["x"]
    for index, cycle in enumerate(cycles):
        search = cycle.get("local_search")
        selected = search.get("selected") if search else None
        if selected is None:
            continue
        exit_record = search.get("early_exit")
        early_committed = exit_record is not None and exit_record["disposition"] == "committed"
        h = exit_record["query_state"] if early_committed else selected["handoff_state"]
        next_cycle = cycles[index + 1] if index + 1 < len(cycles) else None
        conflict = next_cycle.get("conflict_state") if next_cycle else None
        # The original witness H and its room estimate were not flown when an
        # early exit commits. Do not attribute either to the actual earlier end.
        row = next(r for r in search["row_diagnostics"] if r["row_id"] == selected["row_id"])
        room = None if early_committed else row["eligible_handoff"].get("braking_room")
        handoffs.append({"physics_step": h["physics_step"], "x_m": h["position_m"]["x"],
                         "height_above_target_m": h["position_m"]["y"] - target["surface_y_m"],
                         "vx_mps": h["velocity_mps"]["x"], "vy_mps": h["velocity_mps"]["y"],
                         "x_gain_from_previous_h_m": h["position_m"]["x"] - previous_x,
                         "braking_room_m": room["remaining_room_m"] if room else None,
                         "next_conflict_after_h_s": conflict["sim_time_s"] - h["sim_time_s"] if conflict else None})
        previous_x = h["position_m"]["x"]
    final = cycles[-1]
    search = final.get("local_search")
    rows = search["row_diagnostics"] if search else []
    boundaries = search["boundary_status_counts"] if search else {}
    state = final["current_state"]
    first_conflict = cycles[0].get("conflict_state")
    return {"first_conflict_x_m": first_conflict["position_m"]["x"] if first_conflict else None,
            "final_cycle_x_m": state["position_m"]["x"],
            "remaining_to_target_m": target["center_x_m"] - state["position_m"]["x"],
            "remaining_deadline_s": (flight["absolute_deadline_physics_step"] - state["physics_step"]) / scenario["sim"]["physics_hz"],
            "fuel_at_final_cycle_kg": state["fuel_kg"],
            "handoffs": handoffs,
            "last_nominal_physical_attempt_count": sum(final["nominal_attempt_status_counts"].values()),
            "last_nominal_rejection_reason_counts": final["nominal_rejection_reason_counts"],
            "last_local_entry_count": len(search["entries"]) if search else 0,
            "last_local_propagated_row_count": sum(r["physically_propagated"] for r in rows),
            "last_local_accepted_row_count": search["accepted_row_count"] if search else 0,
            "last_local_supported_progress_count": sum(v for k, v in boundaries.items()
                                                       if k in ("unsafe_continuation", "continuation_unsupported", "eligible")),
            "last_local_boundary_status_counts": boundaries,
            "last_local_row_stop_reason_counts": search["row_stop_reason_counts"] if search else {}}


def check_family(case, result, observed):
    failure = case["role"] == "failure"
    family = case["family"]
    if failure:
        if (result["verified_landing"] or result["physical_outcome"] != "flying"
                or result["mission_outcome"] != "in_progress"):
            raise ValueError("finite diagnostic stop was relabeled")
    elif not result["verified_landing"]:
        raise ValueError("successful diagnostic comparison regressed")
    if family == "departure":
        valid = 0 <= observed["first_conflict_x_m"] <= 42
        if failure:
            valid = (valid and result["planning_stop"] == "no_clearing" and result["correction_count"] == 0
                     and observed["last_local_propagated_row_count"] > 0
                     and observed["last_local_accepted_row_count"] == 0
                     and observed["last_local_supported_progress_count"] == 0)
        else:
            valid = valid and result["correction_count"] > 0
    elif family == "acquisition":
        h = observed["handoffs"][-1] if observed["handoffs"] else None
        valid = h is not None and h["x_m"] >= 900 and h["vx_mps"] >= 60
        if failure:
            valid = (valid and result["planning_stop"] == "no_nominal"
                     and observed["last_nominal_physical_attempt_count"] == 0
                     and bool(observed["last_nominal_rejection_reason_counts"]))
    elif family == "progress":
        valid = result["correction_count"] == 6 and len(observed["handoffs"]) == 6
        if failure:
            valid = valid and result["planning_stop"] == "correction_limit" and observed["final_cycle_x_m"] < 600
    elif family == "direct":
        valid = result["nominal_class"] == "clear" and result["correction_count"] == 0 and observed["handoffs"] == []
    else:
        valid = False
    if not valid:
        raise ValueError("mission does not demonstrate declared diagnostic family")


def source_state(binary, plan):
    source = survey.source_state(binary)
    for relative in [str(PLAN.relative_to(survey.REPO)), *(c["scenario_path"] for c in plan["cases"])]:
        source["files"][relative] = study.digest(safe_file(survey.REPO, relative).read_bytes())
    return source


def attempts(plan):
    rows = [dict(c, attempt_id=c["case_id"], cohort="random", status="not_attempted") for c in plan["cases"]]
    by_id = {c["case_id"]: c for c in plan["cases"]}
    rows += [dict(by_id[c], attempt_id="repeat-" + c, cohort="repeat", status="not_attempted") for c in plan["repeat_case_ids"]]
    if len(rows) != plan["maximum_measured_attempts"]:
        raise ValueError("diagnostic allowance differs")
    return rows


def check_record(root, row, case, candidate=False):
    expected_output = "runs/" + row["attempt_id"]
    if row["output_dir"] != expected_output:
        raise ValueError("diagnostic output identity differs")
    output = safe_file(root, expected_output)
    flight, compact = read(output / "flight.json"), read(output / "summary.json")
    if row["exit_code"] != 0 or row.get("comparison_exceptions", []) != []:
        raise ValueError("diagnostic execution/comparison did not pass")
    result = survey.validate_record(flight, compact)
    printed = read(root / "logs" / (row["attempt_id"] + ".stdout"))
    if any(printed.get(k) != flight[k] for k in ("planning_stop", "correction_count", "integrity_passed", "final_source_replay_passed", "physical_outcome", "mission_outcome")):
        raise ValueError("diagnostic stdout disagrees with native evidence")
    if result != row["result"] or read(output / "scenario.json") != read(safe_file(root, case["scenario_path"])):
        raise ValueError("diagnostic result/input disagreement")
    exact = fingerprint(flight) == case["baseline_non_timing_sha256"]
    if not candidate and not exact:
        raise ValueError("complete non-timing diagnostic baseline differs")
    observed = observations(flight, read(output / "scenario.json"))
    if not candidate and (observed != case["baseline_observations"] or result != case["baseline_result"]):
        raise ValueError("diagnostic observations differ from frozen source")
    if not candidate:
        check_family(case, result, observed)
    elif case["role"] == "comparison":
        if not result["verified_landing"] or (case["family"] == "direct" and (result["correction_count"] != 0 or result["nominal_class"] != "clear")):
            raise ValueError("successful diagnostic comparison regressed")
    elif (result["planning_stop"] not in ("landed", "no_clearing", "no_nominal", "correction_limit")
          or (result["planning_stop"] == "landed" and not result["verified_landing"])
          or (result["planning_stop"] != "landed" and (result["physical_outcome"] != "flying" or result["mission_outcome"] != "in_progress"))):
        raise ValueError("unexpected diagnostic physical/mission stop")
    if row["cohort"] == "repeat":
        survey.comparison.compare(flight, read(root / "runs" / case["case_id"] / "flight.json"))
    return {"baseline_exact_non_timing": exact, "observed": observed}


def verify(root):
    root = root.resolve()
    plan = load_contract(root / "plan.json")
    manifest, report, receipt = (read(root / name) for name in ("manifest.json", "diagnostics.json", "receipt.json"))
    if survey.inventory(root) != receipt["files"]:
        raise ValueError("diagnostic capture inventory differs")
    survey.check_inventory(root, receipt["files"])
    source = manifest["source"]
    candidate = manifest["mode"] == "candidate"
    if (manifest["schema"] != "pd-lab.terrain-diagnostics-inputs.v1"
            or (root / "plan.json").read_bytes() != PLAN.read_bytes()
            or manifest["cases"] != plan["cases"]
            or source["files"].get(str(PLAN.relative_to(survey.REPO))) != study.digest((root / "plan.json").read_bytes())
            or manifest["mode"] not in ("baseline", "candidate")
            or (not candidate and source["executable_sha256"] != plan["baseline"]["executable_sha256"])
            or report["schema"] != "pd-lab.terrain-diagnostics-results.v1"
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or report["source_before"] != source or report["source_after"] != source
            or report["protected_after"] != manifest["protected"]):
        raise ValueError("diagnostic plan/source/protected evidence disagreement")
    for relative, digest in source["files"].items():
        if study.digest(safe_file(root / "inputs/source", relative).read_bytes()) != digest:
            raise ValueError("diagnostic source snapshot differs")
    check_inputs(plan, root)
    for case in plan["cases"]:
        if (source["files"].get(case["scenario_path"]) != case["scenario_sha256"]
                or read(root / "preflight" / (case["case_id"] + ".json")) !=
                {"supported": True, "rejection": None, "reason": None, "simulation_created": False}):
            raise ValueError("diagnostic input/preflight is not source-bound")
    expected = attempts(plan)
    if read(root / "run-start.json") != {"attempts": expected}:
        raise ValueError("diagnostic predeclared attempts differ")
    if len(report["rows"]) != len(expected):
        raise ValueError("diagnostic attempt count differs")
    recorded = {}
    for row, frozen in zip(report["rows"], expected):
        if any(row.get(k) != v for k, v in frozen.items() if k != "status"):
            raise ValueError("diagnostic attempt identity/order differs")
        if row["status"] not in ("recorded", "not_attempted", "runner_error", "runner_timeout", "evidence_error"):
            raise ValueError("unknown diagnostic attempt status")
        if row["status"] != "not_attempted" and read(root / "ledger" / (row["attempt_id"] + ".json")) != row:
            raise ValueError("diagnostic ledger differs")
        if row["status"] == "recorded":
            recorded[row["attempt_id"]] = check_record(root, row, frozen, candidate)
        elif report["stopped_reason"] is None:
            raise ValueError("unaccounted diagnostic attempt")
    if (report["observations"] != recorded or report["summary"] != survey.summary_for(report["rows"])
            or (report["stopped_reason"] is None and len(recorded) != 13)):
        raise ValueError("diagnostic summary/observations differ")
    return report


def run(args):
    plan = load_contract()
    check_inputs(plan)
    binary, root = args.binary.resolve(), args.output.resolve()
    source, protected = source_state(binary, plan), survey.protected_state()
    if not args.candidate and source["executable_sha256"] != plan["baseline"]["executable_sha256"]:
        raise ValueError("diagnostic characterization requires its frozen baseline executable")
    # Bounds and inputs are admitted before a create-only reservation or flights.
    rows = attempts(plan)
    survey.reserve(root)
    study.write_new(root / "plan.json", PLAN.read_bytes())
    for relative, digest in source["files"].items():
        data = safe_file(survey.REPO, relative).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed before diagnostic snapshot")
        study.write_new(root / "inputs/source" / relative, data)
    for case in plan["cases"]:
        study.write_new(root / case["scenario_path"], safe_file(survey.REPO, case["scenario_path"]).read_bytes())
        study.write_new(root / "preflight" / (case["case_id"] + ".json"),
                        study.encoded(survey.preflight(binary, root / case["scenario_path"])))
    manifest = {"schema": "pd-lab.terrain-diagnostics-inputs.v1", "source": source,
                "mode": "candidate" if args.candidate else "baseline",
                "protected": protected, "cases": plan["cases"]}
    study.write_new(root / "manifest.json", study.encoded(manifest))
    study.write_new(root / "run-start.json", study.encoded({"attempts": rows}))
    started, stopped, observed = time.monotonic(), None, {}
    for i, row in enumerate(rows):
        if source_state(binary, plan) != source or survey.protected_state() != protected:
            stopped = "source/executable/protected evidence drift"
            break
        remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - started)
        if remaining <= 0:
            stopped = "diagnostic collection wall bound"
            break
        actual = survey.attempt(binary, root, row, min(remaining, plan["case_wall_limit_s"]))
        if actual["status"] == "recorded":
            try:
                observed[row["attempt_id"]] = check_record(root, actual, row, args.candidate)
            except (ValueError, KeyError, TypeError, OSError) as error:
                actual.update(status="evidence_error", failure=str(error))
        rows[i] = actual
        study.write_new(root / "ledger" / (row["attempt_id"] + ".json"), study.encoded(actual))
        print(json.dumps({k: actual.get(k) for k in ("attempt_id", "status", "result", "failure")}), flush=True)
        if actual["status"] != "recorded":
            stopped = row["attempt_id"] + ": " + (actual["failure"] or actual["status"])
            break
    after, protected_after = source_state(binary, plan), survey.protected_state()
    if after != source or protected_after != protected:
        stopped = stopped or "source/executable/protected evidence drift"
    report = {"schema": "pd-lab.terrain-diagnostics-results.v1",
              "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "source_before": source, "source_after": after, "protected_after": protected_after,
              "stopped_reason": stopped, "rows": rows, "observations": observed,
              "summary": survey.summary_for(rows)}
    study.write_new(root / "diagnostics.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if stopped:
        raise RuntimeError("diagnostic pass stopped; evidence retained: " + stopped)
    verify(root)
    print(json.dumps({"verified": str(root), "summary": report["summary"], "measured_attempts": len(rows)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    command = sub.add_parser("run")
    command.add_argument("--binary", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command.add_argument("--candidate", action="store_true", help="explicit future comparison; keep inputs/allowance fixed, permit changed failure trajectories")
    command = sub.add_parser("verify")
    command.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    else:
        report = verify(args.capture)
        print(json.dumps({"verified": str(args.capture), "summary": report["summary"],
                          "completed": report["stopped_reason"] is None,
                          "stopped_reason": report["stopped_reason"], "fresh_flights": 0}))
