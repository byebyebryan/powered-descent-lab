"""Nine frozen early-exit flights. No sweep, publication, tuning or retries."""

import argparse
from pathlib import Path
import subprocess
import time

import cap_probe as cap
import diagnostics as d
import handoff_probe as handoff
import study
import survey

PLAN = survey.HERE / "early_exit_plan.json"
PLAN_SHA256 = "2710bda95a4a90200dde6aa2835149a3d16cc8c92d77c01513e6273bb743785b"


def contract(path=PLAN):
    if study.digest(path.read_bytes()) != PLAN_SHA256:
        raise ValueError("changed nine-mission early-exit contract")
    return d.read(path)


def attempts(plan):
    return [dict(case_id=c, attempt_id=c, status="not_attempted", cohort="primary") for c in plan["primary_order"]] + [
        dict(case_id=c, attempt_id="repeat-" + c, status="not_attempted", cohort="repeat") for c in plan["repeats"]]


def transform(data, plan):
    return cap.transformed_policy(data, plan["experimental_cap"]).replace(
        b'"piecewise_local_clearing_v2_policy_3_cap_probe_24";',
        ('"' + plan["experimental_policy_id"] + '";').encode())


def validate_native(flight, compact, plan):
    policy = {"policy_id": plan["experimental_policy_id"], "maximum_corrections": 24}
    keys = {"planning_stop", "reason", "correction_count", "initial_nominal_terrain_blocked", "integrity_passed",
            "physical_outcome", "mission_outcome", "final_source_replay_passed", "timings"}
    if (flight["policy"] != policy or compact["policy"] != policy
            or compact["schema_id"] != "waypoint_v2_flight_summary_v1"
            or not flight["input_identity"] or compact["input_identity"] != flight["input_identity"]
            or set(compact["result"]) != keys or any(compact["result"][k] != flight[k] for k in keys)
            or not flight["integrity_passed"] or not flight["final_source_replay_passed"]):
        raise ValueError("early-exit native policy/result/integrity mismatch")
    final = flight["ordinary_flight"]["final_state"]
    if (flight["physical_outcome"] != final["physical_outcome"] or flight["mission_outcome"] != final["mission_outcome"]
            or final["physics_step"] > flight["absolute_deadline_physics_step"]
            or flight["correction_count"] > 24
            or flight["correction_count"] != sum(s["kind"] == "local_correction" for s in flight["segments"])):
        raise ValueError("early-exit physical/clock/correction mismatch")
    survey.non_timing(flight)
    result = survey.projection(flight)
    if not result["verified_landing"] and (flight["planning_stop"] != "no_nominal"
            or flight["physical_outcome"] != "flying" or flight["mission_outcome"] != "in_progress"):
        raise ValueError("unexpected early-exit planning or physical stop")
    return result


def unchanged(actual, previous):
    a, b = survey.non_timing(actual), survey.non_timing(previous)
    for f in (a, b):
        del f["policy"]
        del f["input_identity"]
        for c in f["cycles"]:
            if c["local_search"]:
                c["local_search"].pop("early_exit", None)
    if a != b:
        raise ValueError("declined early exit changed complete original flight")


def check_record(root, row, plan):
    output = root / "runs" / row["attempt_id"]
    flight = d.read(output / "flight.json")
    result = validate_native(flight, d.read(output / "summary.json"), plan)
    if row["exit_code"] != 0 or row["output_dir"] != "runs/" + row["attempt_id"] or result != row["result"]:
        raise ValueError("early-exit ledger/runner disagreement")
    case = row["case_id"]
    if (output / "scenario.json").read_bytes() != (root / "references/inputs" / case / "scenario.json").read_bytes():
        raise ValueError("changed frozen scenario bytes")
    printed = d.read(root / "logs" / (row["attempt_id"] + ".stdout"))
    if any(printed.get(k) != flight[k] for k in result if k != "nominal_class" and k != "verified_landing"):
        raise ValueError("native stdout/result differs")
    old = d.read(root / "references/inputs" / case / "flight.json")
    early = [c["local_search"]["early_exit"] for c in flight["cycles"] if c["local_search"] and c["local_search"].get("early_exit")]
    if case in plan["fallback_cases"] or case == "random-000":
        unchanged(flight, old)
    if case == "random-000":
        if early or flight["correction_count"] != 0 or not result["verified_landing"]:
            raise ValueError("direct control entered early correction branch")
    elif case != "random-271":
        if len(early) != 1 or flight["cycles"][0]["local_search"]["selected"] != old["cycles"][0]["local_search"]["selected"]:
            raise ValueError("selected row changed or more than one early query")
        e = early[0]
        reference = root / "references/runs" / (case + "-early")
        probe = d.read(reference / "probe.json")
        if (e["query_state"] != probe["probe_state"] or e["nominal_search"] != d.read(reference / "nominal-search.json")
                or e["audit"] != d.read(reference / "terrain-audit.json")):
            raise ValueError("unchanged constructor/query state/audit differs from frozen contrast")
        tick = e["query_state"]["physics_step"]
        for key in ("actions", "events", "samples"):
            prefix = lambda f: [x for x in f["ordinary_flight"][key] if x["physics_step"] < tick or (key == "samples" and x["physics_step"] == tick)]
            if prefix(flight) != prefix(old):
                raise ValueError("selected-maneuver ordinary prefix changed: " + key)
        if case in plan["fallback_cases"]:
            if e["disposition"] != "terrain_blocked":
                raise ValueError("declined blocked query was committed or relabeled")
        else:
            if e["disposition"] != "committed" or not result["verified_landing"]:
                raise ValueError("intended early exit did not execute and land")
    elif not result["verified_landing"]:
        raise ValueError("multi-waypoint landing control regressed")
    for cycle in flight["cycles"]:
        local = cycle["local_search"]
        if not local or not local.get("early_exit") or local["early_exit"]["disposition"] != "committed":
            continue
        e = local["early_exit"]
        nxt = flight["cycles"][cycle["cycle_index"] + 1]
        selected = e["nominal_search"]["selected"]
        segment = next(s for s in flight["segments"] if s["kind"] == "local_correction" and s["start_physics_step"] == local["selected"]["entry_state"]["physics_step"])
        if (segment["end_state"] != e["query_state"] or not local["handoff_source_replay_passed"]
                or not local["certificate_source_replay_passed"] or nxt["current_state"] != e["query_state"]
                or nxt["nominal_search_identity"] != e["nominal_search"]["identity"]
                or nxt["nominal_proposal_identity"] != selected["identity"]
                or nxt["nominal_updates"] != selected["updates"] or nxt["audit"] != e["audit"]):
            raise ValueError("actual early end/next exact nominal/proof binding differs")
        html = (output / "report.html").read_text()
        if "query evidence, not an executed waypoint" not in html or "Optional early query: committed" not in html:
            raise ValueError("common rich report omitted actual-versus-witness annotation")
    if row["cohort"] == "repeat":
        if survey.non_timing(flight) != survey.non_timing(d.read(root / "runs" / case / "flight.json")):
            raise ValueError("complete numerical repeat differs")
    return {"early_dispositions": [e["disposition"] for e in early],
            "query_steps": [e["query_state"]["physics_step"] if e["query_state"] else None for e in early],
            "witness_steps": [e["witness_handoff_physics_step"] for e in early], "final_state": flight["ordinary_flight"]["final_state"]}


def verify(root):
    plan = contract(root / "plan.json")
    manifest, report, receipt = (d.read(root / n) for n in ("manifest.json", "early-exit.json", "receipt.json"))
    survey.check_inventory(root, receipt["files"])
    if survey.inventory(root) != receipt["files"]:
        raise ValueError("early-exit closed receipt inventory differs")
    source = manifest["source"]
    if (report["schema"] != "pd-lab.terrain-early-exit-results.v1"
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or report["source_after"] != source or report["protected_after"] != manifest["protected"]
            or source["production_executable_sha256"] != plan["production_executable_sha256"]
            or source["files"][str(PLAN.relative_to(survey.REPO))] != PLAN_SHA256):
        raise ValueError("early-exit source/protected provenance differs")
    for path, expected in source["files"].items():
        data = d.safe_file(root / "inputs/source", path).read_bytes()
        if study.digest(data) != expected:
            raise ValueError("copied source differs")
    refs = root / "references"
    for name, key in (("manifest.json", "handoff_probe_manifest_sha256"), ("receipt.json", "handoff_probe_receipt_sha256")):
        if study.digest((refs / name).read_bytes()) != plan[key]:
            raise ValueError("prior diagnostic provenance differs")
    inventory = d.read(refs / "receipt.json")["files"]
    if manifest["references"] != {str(p.relative_to(refs)): study.digest(p.read_bytes()) for p in sorted(refs.rglob("*")) if p.is_file()}:
        raise ValueError("copied reference inventory differs")
    for path, digest in manifest["references"].items():
        if path not in ("manifest.json", "receipt.json") and inventory[path] != digest:
            raise ValueError("reference not authenticated by prior receipt")
    stage = root / "variant"
    expected = dict(source["files"])
    expected[cap.POLICY_PATH] = study.digest(transform((root / "inputs/source" / cap.POLICY_PATH).read_bytes(), plan))
    seal = cap.variant_state(stage, source["files"])
    if seal != d.read(stage / "source-seal.json") or seal["files"] != expected or report["variant_after"] != seal:
        raise ValueError("isolated binary/source seal differs")
    frozen, rows = attempts(plan), report["rows"]
    if len(rows) != 9 or report["measured_missions"] != sum(r["status"] != "not_attempted" for r in rows):
        raise ValueError("measured allowance differs")
    gap = False
    for f, row in zip(frozen, rows):
        if any(f[k] != row[k] for k in f if k != "status") or row["status"] not in ("not_attempted", "recorded", "runner_error", "runner_timeout", "evidence_error"):
            raise ValueError("frozen attempt identity/status differs")
        if row["status"] == "not_attempted":
            gap = True
        elif gap:
            raise ValueError("measured flight after collection stopped")
        if row["status"] == "recorded" and check_record(root, row, plan) != row["diagnostic"]:
            raise ValueError("recorded comparisons differ")
        if row["status"] not in ("recorded", "not_attempted"):
            gap = True
    if report["stopped_reason"] is None and any(r["status"] != "recorded" for r in rows):
        raise ValueError("incomplete matrix labeled successful")
    return report


def run(args):
    plan = contract()
    prior = survey.REPO / plan["handoff_probe_capture"]
    previous = handoff.verify(prior)
    if previous["stopped_reason"] is not None or previous["measured_probes"] != 19:
        raise ValueError("prior diagnostic is incomplete")
    source, protected = cap.main_state(), handoff.protected_state()
    if source["production_executable_sha256"] != plan["production_executable_sha256"]:
        raise ValueError("ordinary release drift")
    survey.reserve(args.output)
    root = args.output
    study.write_new(root / "plan.json", PLAN.read_bytes())
    for path, digest in source["files"].items():
        data = (survey.REPO / path).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source preparation drift")
        study.write_new(root / "inputs/source" / path, data)
    reference_paths = ["manifest.json", "receipt.json", "plan.json"]
    for case in plan["primary_order"]:
        reference_paths += [f"inputs/{case}/{name}" for name in ("scenario.json", "flight.json")]
        if case not in plan["control_cases"]:
            reference_paths += [f"runs/{case}-early/{name}" for name in ("probe.json", "nominal-search.json", "terrain-audit.json")]
    for path in reference_paths:
        study.write_new(root / "references" / path, d.safe_file(prior, path).read_bytes())
    refs = {str(p.relative_to(root / "references")): study.digest(p.read_bytes()) for p in sorted((root / "references").rglob("*")) if p.is_file()}
    study.write_new(root / "manifest.json", study.encoded({"schema": "pd-lab.terrain-early-exit-inputs.v1", "source": source, "protected": protected, "references": refs}))
    stage = root / "variant"
    for path in source["files"]:
        data = (root / "inputs/source" / path).read_bytes()
        study.write_new(stage / "source" / path, transform(data, plan) if path == cap.POLICY_PATH else data)
    command = ["rtk", "proxy", "cargo", "build", "--release", "--locked", "--offline", "-p", "pd-eval", "--manifest-path", str(stage / "source/Cargo.toml"), "--target-dir", str(args.build_target), "--jobs", "4"]
    study.write_new(stage / "build-command.json", study.encoded(command))
    with (stage / "build.log").open("xb") as log:
        built = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=plan["build_wall_limit_s"])
    if built.returncode:
        raise RuntimeError("isolated candidate build failed; no missions started")
    study.write_new(stage / "bin/pd-eval", (args.build_target / "release/pd-eval").read_bytes())
    (stage / "bin/pd-eval").chmod(0o755)
    seal = cap.variant_state(stage, source["files"])
    expected = dict(source["files"])
    expected[cap.POLICY_PATH] = study.digest(transform((root / "inputs/source" / cap.POLICY_PATH).read_bytes(), plan))
    if seal["files"] != expected or cap.main_state() != source or handoff.protected_state() != protected:
        raise ValueError("source/protected state changed before any mission")
    study.write_new(stage / "source-seal.json", study.encoded(seal))
    for case in plan["primary_order"]:
        study.write_new(root / "preflight" / (case + ".json"), study.encoded(survey.preflight(stage / "bin/pd-eval", root / "references/inputs" / case / "scenario.json")))
    rows, stopped, start = attempts(plan), None, time.monotonic()
    study.write_new(root / "run-start.json", study.encoded(rows))
    for i, row in enumerate(rows):
        if time.monotonic() - start + plan["case_wall_limit_s"] > plan["campaign_wall_limit_s"]:
            stopped = "campaign wall bound before next attempt"
            break
        command = [str(stage / "bin/pd-eval"), "waypoint-v2-flight", "--scenario", str(root / "references/inputs" / row["case_id"] / "scenario.json"), "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--output-dir", str(root / "runs" / row["attempt_id"])]
        begin = time.monotonic()
        try:
            run = subprocess.run(command, capture_output=True, timeout=plan["case_wall_limit_s"])
            stdout, stderr, code = run.stdout, run.stderr, run.returncode
            status = "recorded" if code == 0 else "runner_error"
        except subprocess.TimeoutExpired as e:
            stdout, stderr, code, status = e.stdout or b"", e.stderr or b"", None, "runner_timeout"
        except OSError as e:
            stdout, stderr, code, status = b"", str(e).encode(), None, "runner_error"
        study.write_new(root / "logs" / (row["attempt_id"] + ".stdout"), stdout)
        study.write_new(root / "logs" / (row["attempt_id"] + ".stderr"), stderr)
        value = dict(row, status=status, exit_code=code, wall_s=time.monotonic() - begin, output_dir="runs/" + row["attempt_id"], result=None, diagnostic=None, failure=None)
        if status == "recorded":
            try:
                output = root / value["output_dir"]
                value["result"] = validate_native(d.read(output / "flight.json"), d.read(output / "summary.json"), plan)
                value["diagnostic"] = check_record(root, value, plan)
            except (ValueError, KeyError, TypeError, OSError) as e:
                value.update(status="evidence_error", failure=str(e))
        rows[i] = value
        study.write_new(root / "checkpoints" / (row["attempt_id"] + ".json"), study.encoded(value))
        print(row["attempt_id"], value["status"], value["result"], value["failure"], flush=True)
        if value["status"] != "recorded":
            stopped = row["attempt_id"] + ": " + (value["failure"] or value["status"])
            break
    report = {"schema": "pd-lab.terrain-early-exit-results.v1", "manifest_sha256": study.digest((root / "manifest.json").read_bytes()), "rows": rows, "measured_missions": sum(r["status"] != "not_attempted" for r in rows), "stopped_reason": stopped, "source_after": cap.main_state(), "protected_after": handoff.protected_state(), "variant_after": cap.variant_state(stage, source["files"])}
    study.write_new(root / "early-exit.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    verify(root)
    if stopped:
        raise RuntimeError(stopped)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    collect = sub.add_parser("run")
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--build-target", type=Path, required=True)
    check = sub.add_parser("verify")
    check.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        args.output, args.build_target = args.output.resolve(), args.build_target.resolve()
        run(args)
    else:
        result = verify(args.capture.resolve())
        print({"measured_missions": result["measured_missions"], "stopped_reason": result["stopped_reason"]})


if __name__ == "__main__":
    main()
