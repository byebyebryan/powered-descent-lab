"""Source-sealed departure experiment; frozen 13-mission comparison only."""

import argparse
import copy
import json
from pathlib import Path
import subprocess
import time

import cap_probe as cap
import diagnostics as d
import early_exit_sweep as exits
import fresh_validation as fresh
import study
import survey

PLAN = survey.HERE / "departure_probe_plan.json"
PLAN_SHA256 = "69b6d95a5bdbb4fb6d1c8b3d4b7b23d1b7f31194f38f211d445c1e0a386c1f5b"


def contract(path=PLAN):
    if study.digest(path.read_bytes()) != PLAN_SHA256:
        raise ValueError("changed frozen departure probe contract")
    plan = d.read(path)
    diagnostics = d.load_contract()
    if (plan != d.read(PLAN) or plan["schema"] != "pd-lab.departure-clearing-probe-plan.v1"
            or plan["primary_order"] != [c["case_id"] for c in diagnostics["cases"]]
            or plan["repeat_case_ids"] != diagnostics["repeat_case_ids"]
            or plan["maximum_measured_attempts"] != 13 or plan["maximum_extra_rows"] != 56
            or plan["powered_ticks"] != [60, 120, 240, 360, 480, 720, 960]
            or plan["stage_split"] != "half_upright_half_forward_30"
            or plan["retries"] != 0 or plan["publication"]):
        raise ValueError("changed departure probe contract")
    return plan


def transform(data, plan):
    text = data.decode()
    old_id = '"piecewise_local_clearing_v2_policy_3";'
    definition = '''    pub fn revision_3() -> Self {
        Self {
            policy_id: WAYPOINT_V2_POLICY_REVISION_3_ID.into(),
            maximum_corrections: 6,
        }
    }'''
    if text.count(old_id) != 1 or text.count(definition) != 1:
        raise ValueError("unexpected sealed policy source shape")
    return text.replace(old_id, '"' + plan["candidate_policy_id"] + '";').replace(
        definition, definition.replace("maximum_corrections: 6", "maximum_corrections: 24")).encode()


def attempts(plan):
    return ([dict(case_id=cid, attempt_id=cid, cohort="primary", scenario_path=f"scenarios/{cid}.json", status="not_attempted")
             for cid in plan["primary_order"]]
            + [dict(case_id=cid, attempt_id="repeat-" + cid, cohort="repeat", scenario_path=f"scenarios/{cid}.json", status="not_attempted")
               for cid in plan["repeat_case_ids"]])


def compare(actual, previous, subject=False):
    if not subject:
        cap.comparison_without_policy_identity(actual, previous)
        return {"kind": "complete_except_root_identity_and_timings"}
    a, b = actual["cycles"][0], previous["cycles"][0]
    prefix_a, prefix_b = copy.deepcopy(a), copy.deepcopy(b)
    for obj in (prefix_a, prefix_b):
        obj.pop("decision")
        obj.pop("local_search")
    if prefix_a != prefix_b or actual["absolute_deadline_physics_step"] != previous["absolute_deadline_physics_step"]:
        raise ValueError("original nominal/audit/conflict/deadline changed")
    old, new = b["local_search"], a["local_search"]
    if (old["accepted_row_count"] != 0 or b["audit"]["clearance_scan"]["first_violation"]["phase"] != "source_bridge"
            or new["entries"][:len(old["entries"])] != old["entries"]
            or new["row_diagnostics"][:len(old["row_diagnostics"])] != old["row_diagnostics"]):
        raise ValueError("old exhaustive search changed or wrong departure trigger")
    extra = new["row_diagnostics"][len(old["row_diagnostics"]):]
    expected = sum(e["admitted"] for e in old["entries"]) * 7
    if (len(extra) != expected or expected > 56 or new["row_count"] != old["row_count"] + expected
            or new["boundary_count"] != old["boundary_count"] + expected * 360
            or any(not r["row_id"].startswith("departure_") for r in extra)):
        raise ValueError("departure work/row provenance differs")
    selected = new["selected"]
    if selected is None:
        if actual["ordinary_flight"] != previous["ordinary_flight"] or actual["segments"] != previous["segments"]:
            raise ValueError("no-candidate fallback changed executed flight")
    else:
        t, schedule = selected["template"], selected["schedule"]
        ticks = [60, 120, 240, 360, 480, 720, 960]
        if (t["powered_ticks"] not in ticks or t["departure_lift_ticks"] * 2 != t["powered_ticks"]
                or t["acceleration_factor"] != 1.0 or t["row_index"] != 42 + ticks.index(t["powered_ticks"])
                or new.get("early_exit") is not None or not new["handoff_source_replay_passed"]
                or not new["certificate_source_replay_passed"] or new["certificate_state"] != selected["continuation_end_state"]):
            raise ValueError("unproved or altered departure template/continuation")
        segment = next(s for s in actual["segments"] if s["kind"] == "local_correction")
        if (segment["entry_state"] != selected["entry_state"] or segment["end_state"] != selected["handoff_state"]
                or segment["updates"] != [u for u in schedule["updates"] if u["physics_step"] < schedule["handoff_physics_step"]]
                or actual["cycles"][1]["current_state"] != selected["handoff_state"]):
            raise ValueError("actual departure H/commands/next cycle differs")
    return {"kind": "old_search_exact_then_bounded_departure_fallback", "extra_rows": len(extra),
            "accepted_rows": new["accepted_row_count"], "executed_handoff": selected is not None,
            "handoff_state": selected["handoff_state"] if selected else None,
            "next_decision": actual["cycles"][1]["decision"] if selected else None}


def check_record(root, row, plan):
    output = root / "runs" / row["attempt_id"]
    flight = d.read(output / "flight.json")
    result = exits.validate_native(flight, d.read(output / "summary.json"), plan)
    if row["exit_code"] != 0 or row["output_dir"] != "runs/" + row["attempt_id"]:
        raise ValueError("native output/exit differs")
    if not exits.same_scenario((output / "scenario.json").read_bytes(), (root / row["scenario_path"]).read_bytes()):
        raise ValueError("native scenario differs")
    printed = d.read(root / "logs" / (row["attempt_id"] + ".stdout"))
    if any(printed.get(k) != flight[k] for k in d.read(output / "summary.json")["result"]):
        raise ValueError("native stdout differs")
    previous = d.read(root / "baseline" / (row["case_id"] + ".json"))
    comparison = compare(flight, previous, row["case_id"] in plan["subjects"])
    if survey.projection(previous)["verified_landing"] and not result["verified_landing"]:
        raise ValueError("current successful diagnostic lost its landing")
    if row["cohort"] == "repeat":
        fresh.compare(flight, d.read(root / "runs" / row["case_id"] / "flight.json"))
    return result, comparison


def attempt(root, row, plan, limit):
    command = [str(root / "variant/bin/pd-eval"), "waypoint-v2-flight", "--scenario", str(root / row["scenario_path"]),
               "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--output-dir", str(root / "runs" / row["attempt_id"])]
    start = time.monotonic()
    try:
        executed = subprocess.run(command, capture_output=True, timeout=limit)
        out, err, code = executed.stdout, executed.stderr, executed.returncode
        status = "recorded" if code == 0 else "runner_error"
    except subprocess.TimeoutExpired as error:
        out, err, code, status = error.stdout or b"", error.stderr or b"", None, "runner_timeout"
    except OSError as error:
        out, err, code, status = b"", str(error).encode(), None, "runner_error"
    study.write_new(root / "logs" / (row["attempt_id"] + ".stdout"), out)
    study.write_new(root / "logs" / (row["attempt_id"] + ".stderr"), err)
    actual = dict(row, exit_code=code, output_dir="runs/" + row["attempt_id"], status=status,
                  result=None, comparison=None, failure=None, wall_s=time.monotonic() - start)
    if status == "recorded":
        try:
            actual["result"], actual["comparison"] = check_record(root, actual, plan)
        except (ValueError, KeyError, TypeError, OSError, StopIteration, IndexError) as error:
            actual.update(status="evidence_error", failure=str(error))
    return actual


def verify(root):
    plan = contract(root / "plan.json")
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("departure capture inventory differs")
    survey.check_inventory(root, receipt)
    manifest, report = d.read(root / "manifest.json"), d.read(root / "departure-probe.json")
    for filename, key in (("baseline-receipt.json", "baseline_receipt_sha256"), ("fresh-receipt.json", "fresh_receipt_sha256")):
        if study.digest((root / "references" / filename).read_bytes()) != plan[key]:
            raise ValueError("authenticated prerequisite/baseline receipt differs")
    baseline_files = d.read(root / "references/baseline-receipt.json")["files"]
    source = manifest["source"]
    expected_files = dict(source["files"])
    for path, digest in source["files"].items():
        if study.digest(d.safe_file(root / "inputs/source", path).read_bytes()) != digest:
            raise ValueError("frozen production source differs")
    expected_files[cap.POLICY_PATH] = study.digest(transform((root / "inputs/source" / cap.POLICY_PATH).read_bytes(), plan))
    seal = cap.variant_state(root / "variant", source["files"])
    if seal != d.read(root / "variant/source-seal.json") or seal["files"] != expected_files or seal != report["variant_after"]:
        raise ValueError("candidate source/executable differs")
    for case in d.load_contract()["cases"]:
        if study.digest((root / "scenarios" / (case["case_id"] + ".json")).read_bytes()) != case["scenario_sha256"]:
            raise ValueError("frozen fixture input differs")
        if study.digest((root / "baseline" / (case["case_id"] + ".json")).read_bytes()) != manifest["baseline_flight_hashes"][case["case_id"]]:
            raise ValueError("paired baseline flight differs")
        if manifest["baseline_flight_hashes"][case["case_id"]] != baseline_files[f'runs/{case["case_id"]}/flight.json']:
            raise ValueError("paired baseline is not receipt-authenticated")
    if (report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or len(report["rows"]) != 13):
        raise ValueError("manifest/attempt count differs")
    for frozen, row in zip(attempts(plan), report["rows"]):
        if any(row[k] != v for k, v in frozen.items() if k != "status"):
            raise ValueError("frozen attempt identity/order differs")
        if row["status"] == "recorded" and check_record(root, row, plan) != (row["result"], row["comparison"]):
            raise ValueError("saved comparison/result differs")
        if report["stopped_reason"] is None and row["status"] != "recorded":
            raise ValueError("incomplete capture marked complete")
    if report["stopped_reason"] is None and (report["source_after"] != source or report["protected_after"] != manifest["protected"]):
        raise ValueError("production/protected evidence drift")
    return report


def run(args):
    plan = contract()
    if 'departure-clearing-probe = ' not in (survey.REPO / "pd-eval/Cargo.toml").read_text():
        raise ValueError("departure runtime retired after the negative probe; use verify on the retained source-sealed capture")
    checkpoint = survey.REPO / plan["fresh_capture"]
    if study.digest((checkpoint / "receipt.json").read_bytes()) != plan["fresh_receipt_sha256"]:
        raise ValueError("fresh checkpoint receipt differs")
    validated = fresh.verify(checkpoint)
    if validated["stopped_reason"] is not None or validated["summary"]["recorded_count"] != 100:
        raise ValueError("fresh checkpoint incomplete")
    previous, bound, _, _ = fresh.baseline(fresh.contract())
    d.check_inputs(d.load_contract())
    root = args.output.resolve()
    source, protected = cap.main_state(), survey.protected_state()
    survey.reserve(root)
    study.write_new(root / "plan.json", PLAN.read_bytes())
    study.write_new(root / "references/baseline-receipt.json", (previous / "receipt.json").read_bytes())
    study.write_new(root / "references/fresh-receipt.json", (checkpoint / "receipt.json").read_bytes())
    for path, digest in source["files"].items():
        data = d.safe_file(survey.REPO, path).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source preparation drift")
        study.write_new(root / "inputs/source" / path, data)
        study.write_new(root / "variant/source" / path, transform(data, plan) if path == cap.POLICY_PATH else data)
    hashes = {}
    for case in d.load_contract()["cases"]:
        cid = case["case_id"]
        scenario, flight = bound(f"scenarios/{cid}.json"), bound(f"runs/{cid}/flight.json")
        if study.digest(scenario) != case["scenario_sha256"]:
            raise ValueError("latest baseline scenario differs from frozen fixture")
        study.write_new(root / "scenarios" / (cid + ".json"), scenario)
        study.write_new(root / "baseline" / (cid + ".json"), flight)
        hashes[cid] = study.digest(flight)
    manifest = {"schema": "pd-lab.departure-probe-inputs.v1", "source": source, "protected": protected,
                "baseline_flight_hashes": hashes}
    study.write_new(root / "manifest.json", study.encoded(manifest))
    command = ["rtk", "proxy", "cargo", "build", "--release", "--locked", "--offline", "-p", "pd-eval",
               "--features", plan["feature"], "--manifest-path", str(root / "variant/source/Cargo.toml"),
               "--target-dir", str(args.build_target.resolve()), "--jobs", "4"]
    study.write_new(root / "variant/build-command.json", study.encoded(command))
    with (root / "variant/build.log").open("xb") as log:
        built = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=plan["build_wall_limit_s"])
    if built.returncode:
        raise RuntimeError("candidate build failed; no measured missions started")
    study.write_new(root / "variant/bin/pd-eval", (args.build_target / "release/pd-eval").read_bytes())
    native = root / "variant/bin/pd-eval"
    native.chmod(0o755)
    seal = cap.variant_state(root / "variant", source["files"])
    study.write_new(root / "variant/source-seal.json", study.encoded(seal))
    for cid in plan["primary_order"]:
        study.write_new(root / "preflight" / (cid + ".json"), study.encoded(survey.preflight(native, root / "scenarios" / (cid + ".json"))))
    rows = attempts(plan)
    study.write_new(root / "run-start.json", study.encoded(rows))
    start, stopped = time.monotonic(), None
    for i, row in enumerate(rows):
        if cap.main_state() != source or survey.protected_state() != protected or cap.variant_state(root / "variant", source["files"]) != seal:
            stopped = "source/executable/protected evidence drift"
            break
        remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - start)
        if remaining <= 0:
            stopped = "collection wall bound"
            break
        rows[i] = attempt(root, row, plan, min(remaining, plan["case_wall_limit_s"]))
        study.write_new(root / "ledger" / (row["attempt_id"] + ".json"), study.encoded(rows[i]))
        print(json.dumps({k: rows[i].get(k) for k in ("attempt_id", "status", "result", "failure")}), flush=True)
        if rows[i]["status"] != "recorded":
            stopped = row["attempt_id"] + ": " + str(rows[i]["failure"] or rows[i]["status"])
            break
    after, protected_after = cap.main_state(), survey.protected_state()
    if after != source or protected_after != protected:
        stopped = stopped or "source/protected evidence drift"
    report = {"schema": "pd-lab.departure-probe-results.v1", "rows": rows, "stopped_reason": stopped,
              "manifest_sha256": study.digest((root / "manifest.json").read_bytes()), "source_after": after,
              "protected_after": protected_after, "variant_after": cap.variant_state(root / "variant", source["files"])}
    study.write_new(root / "departure-probe.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    verify(root)
    print(json.dumps({"capture": str(root), "measured_attempts": sum(r["status"] != "not_attempted" for r in rows), "stopped_reason": stopped}), flush=True)
    if stopped:
        raise RuntimeError("departure pass stopped; evidence retained")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    collect = commands.add_parser("run")
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--build-target", type=Path, required=True)
    check = commands.add_parser("verify")
    check.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    else:
        report = verify(args.capture)
        print(json.dumps({"verified": str(args.capture), "measured_attempts": sum(r["status"] != "not_attempted" for r in report["rows"]), "fresh_flights": 0}))
