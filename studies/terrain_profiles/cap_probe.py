"""Isolated cap-only continuation diagnostic; production policy stays sealed."""

import argparse
import copy
import json
from pathlib import Path
import subprocess
import time

import diagnostics as d
import study
import survey

PLAN = survey.HERE / "cap_probe_plan.json"
POLICY_PATH = "pd-plan/src/waypoint_v2.rs"
POLICY_SHA256 = "ae6d656f7ab5c75341ac37da28291dea2b7debb9ecd26324a1cd746d4c0e5016"
CAPS = [12, 24]
PRIMARIES = ["random-000", "random-253", "random-030", "random-280"]
SUBJECTS = ["random-030", "random-280"]


def contract(path=PLAN):
    p = d.read(path)
    if (p["schema"] != "pd-lab.correction-cap-probe-plan.v1" or p["caps"] != CAPS
            or p["primary_case_ids"] != PRIMARIES or p["subject_case_ids"] != SUBJECTS
            or p["repeat_case_ids"] != SUBJECTS or p["control_case_ids"] != PRIMARIES[:2]
            or [p[k] for k in ("maximum_measured_attempts", "case_wall_limit_s", "campaign_wall_limit_s", "build_wall_limit_s", "workers")]
            != [12, 120, 900, 900, 1]
            or p["policy_source_path"] != POLICY_PATH or p["policy_source_sha256"] != POLICY_SHA256
            or p["baseline_manifest_sha256"] != "1372da0120b7dbb9a2eb72ae84c2ebc6a8dbbdf823c6db9ddd95b50a9b814a7c"
            or p["baseline_receipt_sha256"] != "03b8ac8f74b7fd7290bf0e65cd1e306fbe47da27689631e234f881d1eb6ea09c"
            or p["production_executable_sha256"] != "fdbf20a694de3ac82b79492d119fe850e4bfea5d3c4c70a3ca854a72cfe5d2af"
            or p["second_stage_condition"] != "subject_still_at_correction_limit"):
        raise ValueError("changed bounded cap-probe contract")
    return p


def policy(cap):
    if cap not in CAPS:
        raise ValueError("undeclared diagnostic cap")
    return {"policy_id": f"piecewise_local_clearing_v2_policy_3_cap_probe_{cap}", "maximum_corrections": cap}


def transformed_policy(original, cap):
    """Mechanical generated-source transformation, never a live Rust edit."""
    declared = policy(cap)
    if study.digest(original) != POLICY_SHA256:
        raise ValueError("policy source differs from authenticated baseline")
    text = original.decode()
    old_id = '"piecewise_local_clearing_v2_policy_3";'
    old_definition = '''    pub fn revision_3() -> Self {
        Self {
            policy_id: WAYPOINT_V2_POLICY_REVISION_3_ID.into(),
            maximum_corrections: 6,
        }
    }'''
    if text.count(old_id) != 1 or text.count(old_definition) != 1:
        raise ValueError("unexpected sealed policy source shape")
    return text.replace(old_id, '"' + declared["policy_id"] + '";').replace(
        old_definition, old_definition.replace("maximum_corrections: 6", f"maximum_corrections: {cap}")).encode()


def main_state():
    files = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard"],
                           cwd=survey.REPO, capture_output=True, check=True, text=True).stdout.splitlines()
    selected = sorted({p for p in files if p in ("Cargo.toml", "Cargo.lock")
                       or (p.startswith(("pd-core/", "pd-plan/", "pd-control/", "pd-report/", "pd-eval/", "pd-cli/", "fixtures/", "studies/terrain_profiles/"))
                           and p.endswith((".rs", ".toml", ".lock", ".css", ".json", ".py")))})
    return {"files": {p: study.digest(d.safe_file(survey.REPO, p).read_bytes()) for p in selected},
            "production_executable_sha256": study.digest((survey.REPO / "target/release/pd-eval").read_bytes())}


def stage_attempts(cap):
    policy(cap)
    return [dict(case_id=cid, attempt_id=cid, cohort="primary", status="not_attempted") for cid in PRIMARIES] + [
        dict(case_id=cid, attempt_id="repeat-" + cid, cohort="repeat", status="not_attempted") for cid in SUBJECTS]


def stage_two_needed(rows):
    return any(r["cohort"] == "primary" and r["case_id"] in SUBJECTS and r["status"] == "recorded"
               and r["result"]["planning_stop"] == "correction_limit" for r in rows)


def comparison_without_policy_identity(actual, previous):
    """Only explicit cross-policy identities and wall timings may differ."""
    a, b = survey.non_timing(actual), survey.non_timing(previous)
    for obj in (a, b):
        del obj["policy"]
        del obj["input_identity"]
    if a != b:
        raise ValueError("non-cap-stopped flight changed beyond policy/input identity")


def compare_prefix(actual, previous):
    if previous["planning_stop"] != "correction_limit":
        comparison_without_policy_identity(actual, previous)
        return {"kind": "complete_except_policy_identity", "through_physics_step": previous["ordinary_flight"]["final_state"]["physics_step"]}
    count = previous["correction_count"]
    if (count != previous["policy"]["maximum_corrections"] or len(previous["cycles"]) != count + 1
            or len(actual["cycles"]) < count + 1 or actual["cycles"][:count] != previous["cycles"][:count]
            or actual["segments"][:len(previous["segments"])] != previous["segments"]
            or actual["absolute_deadline_physics_step"] != previous["absolute_deadline_physics_step"]):
        raise ValueError("executed cap-bound planning/segment prefix changed")
    old_state = previous["ordinary_flight"]["final_state"]
    tick = old_state["physics_step"]
    if actual["cycles"][count]["current_state"] != old_state:
        raise ValueError("actual state at prior cap boundary changed")
    # The cap test happens after the next fixed nominal is built/audited.
    a, b = copy.deepcopy(actual["cycles"][count]), copy.deepcopy(previous["cycles"][count])
    for c in (a, b):
        for key in ("decision", "conflict_state", "conflict_incoming_contact", "local_search"):
            c.pop(key)
    if a != b:
        raise ValueError("next nominal before cap decision changed")
    for key in ("actions", "events", "samples"):
        end_inclusive = key == "samples"
        prefix = [r for r in actual["ordinary_flight"][key] if r["physics_step"] < tick or (end_inclusive and r["physics_step"] == tick)]
        if prefix != previous["ordinary_flight"][key]:
            raise ValueError("actual ordinary source prefix changed: " + key)
    return {"kind": "exact_executed_cap_prefix", "corrections": count, "through_physics_step": tick}


def validate_native(flight, compact, cap):
    if (flight["policy"] != policy(cap) or compact["policy"] != flight["policy"]
            or compact["schema_id"] != "waypoint_v2_flight_summary_v1"
            or compact["input_identity"] != flight["input_identity"]
            or not flight["input_identity"]):
        raise ValueError("probe policy/native input identity differs")
    keys = {"planning_stop", "reason", "correction_count", "initial_nominal_terrain_blocked", "integrity_passed",
            "physical_outcome", "mission_outcome", "final_source_replay_passed", "timings"}
    if set(compact["result"]) != keys or any(compact["result"][k] != flight[k] for k in keys):
        raise ValueError("probe full/compact native result disagreement")
    if (flight["integrity_passed"] is not True or flight["final_source_replay_passed"] is not True
            or flight["correction_count"] > cap):
        raise ValueError("probe integrity/replay/cap failure")
    survey.non_timing(flight)
    final = flight["ordinary_flight"]["final_state"]
    if (flight["physical_outcome"] != final["physical_outcome"] or flight["mission_outcome"] != final["mission_outcome"]
            or final["physics_step"] > flight["absolute_deadline_physics_step"]
            or flight["correction_count"] != sum(s["kind"] == "local_correction" for s in flight["segments"])):
        raise ValueError("probe outcome/clock/correction inventory disagreement")
    result = survey.projection(flight)
    stop = flight["planning_stop"]
    if stop == "correction_limit" and flight["correction_count"] != cap:
        raise ValueError("probe correction limit is not actually binding")
    if stop == "landed":
        if not result["verified_landing"]:
            raise ValueError("incomplete probe landing tuple")
    elif stop in ("no_clearing", "no_nominal", "correction_limit", "nominal_rejected", "deadline"):
        running = flight["physical_outcome"] == "flying" and flight["mission_outcome"] == "in_progress"
        timed_out = stop == "deadline" and flight["physical_outcome"] == "timed_out" and flight["mission_outcome"] == "failed_timeout"
        if not running and not timed_out:
            raise ValueError("unexpected probe physical/mission stop")
    else:
        raise ValueError("unexpected probe planning stop")
    return result


def check_record(root, cap, row):
    stage = root / "variants" / f"cap-{cap}"
    if row["output_dir"] != "runs/" + row["attempt_id"] or row["exit_code"] != 0:
        raise ValueError("probe output/exit identity differs")
    output = d.safe_file(stage, row["output_dir"])
    flight, compact = d.read(output / "flight.json"), d.read(output / "summary.json")
    result = validate_native(flight, compact, cap)
    if result != row["result"] or d.read(output / "scenario.json") != d.read(root / "scenarios" / (row["case_id"] + ".json")):
        raise ValueError("probe result or executed input differs")
    printed = d.read(stage / "logs" / (row["attempt_id"] + ".stdout"))
    if any(printed.get(k) != flight[k] for k in ("planning_stop", "correction_count", "integrity_passed", "final_source_replay_passed", "physical_outcome", "mission_outcome")):
        raise ValueError("probe CLI stdout disagrees")
    previous = d.read(root / "baseline/runs" / row["case_id"] / "flight.json")
    comparisons = [compare_prefix(flight, previous)]
    if cap == 24:
        earlier = d.read(root / "variants/cap-12/runs" / row["case_id"] / "flight.json")
        comparisons.append(compare_prefix(flight, earlier))
    if row["case_id"] in PRIMARIES[:2] and (not result["verified_landing"] or (row["case_id"] == "random-000" and result["correction_count"] != 0)):
        raise ValueError("probe preservation control regressed")
    if row["cohort"] == "repeat":
        survey.comparison.compare(flight, d.read(stage / "runs" / row["case_id"] / "flight.json"))
    final = flight["ordinary_flight"]["final_state"]
    scenario = d.read(output / "scenario.json")
    return {"comparisons": comparisons, "endpoint": final,
            "remaining_deadline_s": (flight["absolute_deadline_physics_step"] - final["physics_step"]) / scenario["sim"]["physics_hz"],
            "observations": d.observations(flight, scenario)}


def attempt(root, cap, row, limit):
    stage = root / "variants" / f"cap-{cap}"
    output = stage / "runs" / row["attempt_id"]
    binary = stage / "bin/pd-eval"
    command = [str(binary), "waypoint-v2-flight", "--scenario", str(root / "scenarios" / (row["case_id"] + ".json")),
               "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--output-dir", str(output)]
    start = time.monotonic()
    try:
        result = subprocess.run(command, capture_output=True, timeout=limit)
        stdout, stderr, code = result.stdout, result.stderr, result.returncode
        status = "recorded" if code == 0 else "runner_error"
    except subprocess.TimeoutExpired as e:
        stdout, stderr, code, status = e.stdout or b"", e.stderr or b"", None, "runner_timeout"
    except OSError as e:
        stdout, stderr, code, status = b"", str(e).encode(), None, "runner_error"
    study.write_new(stage / "logs" / (row["attempt_id"] + ".stdout"), stdout)
    study.write_new(stage / "logs" / (row["attempt_id"] + ".stderr"), stderr)
    recorded = dict(row, status=status, exit_code=code, wall_s=time.monotonic() - start,
                    output_dir="runs/" + row["attempt_id"], result=None, diagnostic=None, failure=None)
    if status == "recorded":
        try:
            recorded["result"] = validate_native(d.read(output / "flight.json"), d.read(output / "summary.json"), cap)
            recorded["diagnostic"] = check_record(root, cap, recorded)
        except (ValueError, KeyError, TypeError, OSError) as e:
            recorded.update(status="evidence_error", failure=str(e))
    return recorded


def variant_state(stage, files):
    return {"files": {p: study.digest(d.safe_file(stage / "source", p).read_bytes()) for p in files},
            "executable_sha256": study.digest((stage / "bin/pd-eval").read_bytes())}


def build_variant(root, cap, source, plan, build_target):
    stage = root / "variants" / f"cap-{cap}"
    for relative, expected in source["files"].items():
        data = d.safe_file(root / "inputs/source", relative).read_bytes()
        if study.digest(data) != expected:
            raise ValueError("changed source during probe preparation")
        if relative == POLICY_PATH:
            data = transformed_policy(data, cap)
        study.write_new(stage / "source" / relative, data)
    command = ["rtk", "proxy", "cargo", "build", "--release", "--locked", "--offline", "-p", "pd-eval",
               "--manifest-path", str(stage / "source/Cargo.toml"), "--target-dir", str(build_target), "--jobs", "4"]
    study.write_new(stage / "build-command.json", study.encoded(command))
    log = stage / "build.log"
    with log.open("xb") as stream:
        result = subprocess.run(command, cwd=stage / "source", stdout=stream, stderr=subprocess.STDOUT,
                                timeout=plan["build_wall_limit_s"])
    if result.returncode != 0:
        raise RuntimeError("isolated probe build failed; inspect " + str(log))
    study.write_new(stage / "bin/pd-eval", (build_target / "release/pd-eval").read_bytes())
    (stage / "bin/pd-eval").chmod(0o755)
    state = variant_state(stage, source["files"])
    expected = dict(source["files"])
    expected[POLICY_PATH] = study.digest(transformed_policy((root / "inputs/source" / POLICY_PATH).read_bytes(), cap))
    if state["files"] != expected:
        raise ValueError("probe source differs beyond declared transformation")
    study.write_new(stage / "source-seal.json", study.encoded(state))
    for cid in PRIMARIES:
        study.write_new(stage / "preflight" / (cid + ".json"), study.encoded(survey.preflight(stage / "bin/pd-eval", root / "scenarios" / (cid + ".json"))))
    return state


def verify(root):
    root = root.resolve()
    plan, manifest, report, receipt = (d.read(root / n) for n in ("plan.json", "manifest.json", "cap-probe.json", "receipt.json"))
    plan = contract(root / "plan.json")
    if (root / "plan.json").read_bytes() != PLAN.read_bytes() or survey.inventory(root) != receipt["files"]:
        raise ValueError("probe plan or complete receipt differs")
    survey.check_inventory(root, receipt["files"])
    if (manifest["schema"] != "pd-lab.correction-cap-probe-inputs.v1"
            or report["schema"] != "pd-lab.correction-cap-probe-results.v1"
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or report["main_source_after"] != manifest["main_source_before"]
            or report["protected_after"] != manifest["protected"]
            or manifest["main_source_before"]["production_executable_sha256"] != plan["production_executable_sha256"]
            or manifest["main_source_before"]["files"].get(str(PLAN.relative_to(survey.REPO))) != study.digest((root / "plan.json").read_bytes())):
        raise ValueError("probe source/default executable/protected evidence drift")
    source = manifest["main_source_before"]
    for relative, expected in source["files"].items():
        if study.digest(d.safe_file(root / "inputs/source", relative).read_bytes()) != expected:
            raise ValueError("probe main source snapshot differs")
    for filename, key in (("manifest.json", "baseline_manifest_sha256"), ("receipt.json", "baseline_receipt_sha256")):
        if study.digest((root / "baseline" / filename).read_bytes()) != plan[key]:
            raise ValueError("unbound probe baseline")
    old_receipt = d.read(root / "baseline/receipt.json")["files"]
    if study.digest((root / "baseline/plan.json").read_bytes()) != old_receipt["plan.json"]:
        raise ValueError("unbound original diagnostic plan")
    old_plan = d.read(root / "baseline/plan.json")
    old_cases = {c["case_id"]: c for c in old_plan["cases"]}
    for cid in PRIMARIES:
        for name in ("flight.json", "scenario.json"):
            relative = f"runs/{cid}/{name}"
            if study.digest((root / "baseline" / relative).read_bytes()) != old_receipt[relative]:
                raise ValueError("changed original probe baseline artifact")
        if study.digest((root / "scenarios" / (cid + ".json")).read_bytes()) != old_cases[cid]["scenario_sha256"]:
            raise ValueError("probe scenario differs from frozen diagnostic")
    expected_caps = [12, 24] if report["stages"] and stage_two_needed(report["stages"][0]["rows"]) else [12]
    if report["stopped_reason"] is None and [s["cap"] for s in report["stages"]] != expected_caps:
        raise ValueError("probe stage admission/count differs")
    attempts = 0
    for index, record in enumerate(report["stages"]):
        cap = record["cap"]
        if cap != CAPS[index]:
            raise ValueError("probe stage order differs")
        stage = root / "variants" / f"cap-{cap}"
        seal = d.read(stage / "source-seal.json")
        expected = dict(source["files"])
        expected[POLICY_PATH] = study.digest(transformed_policy((root / "inputs/source" / POLICY_PATH).read_bytes(), cap))
        if seal["files"] != expected or variant_state(stage, seal["files"]) != seal:
            raise ValueError("probe source/binary seal differs")
        for cid in PRIMARIES:
            if d.read(stage / "preflight" / (cid + ".json")) != {"supported": True, "rejection": None, "reason": None, "simulation_created": False}:
                raise ValueError("probe preflight differs")
        frozen = stage_attempts(cap)
        if d.read(stage / "run-start.json") != frozen or len(record["rows"]) != len(frozen):
            raise ValueError("probe predeclared population differs")
        for row, original in zip(record["rows"], frozen):
            if any(row.get(k) != v for k, v in original.items() if k != "status"):
                raise ValueError("probe attempt identity/order differs")
            if row["status"] != "not_attempted":
                if row["status"] not in ("recorded", "runner_error", "runner_timeout", "evidence_error"):
                    raise ValueError("unknown probe attempt status")
                attempts += 1
                if d.read(stage / "ledger" / (row["attempt_id"] + ".json")) != row:
                    raise ValueError("probe ledger differs")
            if row["status"] == "recorded":
                if check_record(root, cap, row) != row["diagnostic"]:
                    raise ValueError("probe diagnostic differs from actual native record")
            elif report["stopped_reason"] is None:
                raise ValueError("incomplete probe stage")
    if attempts > plan["maximum_measured_attempts"] or attempts != report["measured_attempts"]:
        raise ValueError("probe measured allowance differs")
    return report


def run(args):
    plan = contract()
    baseline = survey.REPO / plan["baseline_capture"]
    d.verify(baseline)
    for filename, key in (("manifest.json", "baseline_manifest_sha256"), ("receipt.json", "baseline_receipt_sha256")):
        if study.digest((baseline / filename).read_bytes()) != plan[key]:
            raise ValueError("changed authenticated diagnostic baseline")
    source, protected = main_state(), survey.protected_state()
    if source["production_executable_sha256"] != plan["production_executable_sha256"]:
        raise ValueError("production evaluator changed before cap probe")
    if source["files"].get(POLICY_PATH) != POLICY_SHA256:
        raise ValueError("sealed production policy source changed before cap probe")
    root = args.output.resolve()
    build_target = survey.REPO / "target" / ("cap-probe-" + root.name)
    if build_target.exists() or build_target.is_symlink():
        raise ValueError("cap-probe build target must be new")
    survey.reserve(root)
    study.write_new(root / "plan.json", PLAN.read_bytes())
    for relative, digest in source["files"].items():
        data = d.safe_file(survey.REPO, relative).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("main source drift before cap-probe snapshot")
        study.write_new(root / "inputs/source" / relative, data)
    for name in ("manifest.json", "receipt.json", "plan.json"):
        study.write_new(root / "baseline" / name, (baseline / name).read_bytes())
    old_cases = {c["case_id"]: c for c in d.read(baseline / "plan.json")["cases"]}
    for cid in PRIMARIES:
        study.write_new(root / "scenarios" / (cid + ".json"), (survey.REPO / old_cases[cid]["scenario_path"]).read_bytes())
        for name in ("flight.json", "scenario.json"):
            study.write_new(root / "baseline/runs" / cid / name, (baseline / "runs" / cid / name).read_bytes())
    manifest = {"schema": "pd-lab.correction-cap-probe-inputs.v1", "main_source_before": source,
                "protected": protected, "build_target": str(build_target)}
    study.write_new(root / "manifest.json", study.encoded(manifest))
    stages, stopped, spent_s, measured = [], None, 0.0, 0
    try:
        for cap in CAPS:
            if cap == 24 and not stage_two_needed(stages[0]["rows"]):
                break
            print(json.dumps({"building_isolated_cap": cap, "production_default": 6}), flush=True)
            seal = build_variant(root, cap, source, plan, build_target)
            stage = root / "variants" / f"cap-{cap}"
            rows = stage_attempts(cap)
            study.write_new(stage / "run-start.json", study.encoded(rows))
            stages.append({"cap": cap, "rows": rows})
            for i, row in enumerate(rows):
                if (main_state() != source or survey.protected_state() != protected
                        or variant_state(stage, source["files"]) != seal):
                    raise ValueError("cap-probe source/executable/protected drift")
                remaining = plan["campaign_wall_limit_s"] - spent_s
                if remaining <= 0:
                    raise ValueError("cap-probe collection wall bound")
                measured += 1
                started = time.monotonic()
                actual = attempt(root, cap, row, min(remaining, plan["case_wall_limit_s"]))
                spent_s += time.monotonic() - started
                rows[i] = actual
                study.write_new(stage / "ledger" / (row["attempt_id"] + ".json"), study.encoded(actual))
                print(json.dumps({"cap": cap, **{k: actual.get(k) for k in ("attempt_id", "status", "result", "failure")}}), flush=True)
                if actual["status"] != "recorded":
                    raise ValueError("cap-probe attempt failed: " + row["attempt_id"] + ": " + (actual["failure"] or actual["status"]))
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        stopped = str(error)
    after, protected_after = main_state(), survey.protected_state()
    if after != source or protected_after != protected:
        stopped = stopped or "main source/default executable/protected drift"
    report = {"schema": "pd-lab.correction-cap-probe-results.v1", "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "main_source_after": after, "protected_after": protected_after, "stages": stages,
              "measured_attempts": measured, "collection_wall_s": spent_s, "stopped_reason": stopped}
    study.write_new(root / "cap-probe.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if stopped:
        raise RuntimeError("cap probe stopped; partial evidence retained: " + stopped)
    verify(root)
    print(json.dumps({"verified": str(root), "caps": [s["cap"] for s in stages], "measured_attempts": measured}))


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
        print(json.dumps({"verified": str(args.capture), "completed": report["stopped_reason"] is None,
                          "measured_attempts": report["measured_attempts"], "stopped_reason": report["stopped_reason"], "fresh_flights": 0}))
