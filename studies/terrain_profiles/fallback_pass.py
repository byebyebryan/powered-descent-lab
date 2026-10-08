"""Bounded failure-only timing validation over unchanged saved challenge inputs."""

import argparse
import copy
import json
from pathlib import Path
import subprocess

import intervention_pass as timing
import study
import survey

PLAN = timing.HERE / "intervention_fallback_plan.json"
BENCHMARK = survey.REPO / "outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native"


def contract():
    plan = timing.read(PLAN)
    old = timing.contract()
    expected = dict(old, schema="pd-lab.intervention-fallback-pass.v1",
                    focus_indices=[0, 13, 23, 34, 35, 43, 49, 50, 54, 56, 60, 70, 81, 85, 93, 94, 99],
                    maximum_measured_attempts=163)
    if plan != expected:
        raise ValueError("changed bounded fallback-pass contract")
    return plan


def without_query_diagnostics(flight):
    value = copy.deepcopy(flight)
    for cycle in value["cycles"]:
        if cycle.get("local_search") is not None:
            cycle["local_search"].pop("row_diagnostics", None)
    return value


def preservation(actual, baseline):
    if baseline["planning_stop"] == "landed" and timing.motion(actual) != timing.motion(baseline):
        raise ValueError("fallback changed a previously landed challenge flight")


def verify(root):
    plan = contract()
    report = timing.verify(root, plan, PLAN)
    old, _ = timing.baseline(plan)
    for row in report["rows"]:
        if row["status"] != "recorded":
            continue
        flight = timing.read(root / row["output_dir"] / "flight.json")
        preservation(flight, timing.read(old / "runs" / row["case_id"] / "flight.json"))
        total = 0
        for cycle in flight["cycles"]:
            local = cycle.get("local_search")
            if local is None:
                continue
            entries = local["entries"]
            if (len(entries) > 8 or len({e["physics_step"] for e in entries}) != len(entries)
                    or local["row_count"] != len(entries) * 42
                    or len(local["row_diagnostics"]) != local["row_count"]):
                raise ValueError("incorrect bounded or deduplicated search inventory")
            total += local["row_count"]
            fallback = [e for e in entries if e["entry_id"].startswith("fallback_")]
            if fallback:
                primary_rows = [d for d in local["row_diagnostics"]
                                if not d["row_id"].startswith("fallback_")]
                if any(d["boundary_status_counts"].get("eligible", 0) for d in primary_rows):
                    raise ValueError("fallback ran despite a successful primary query")
        if total > 2016:
            raise ValueError("flight exceeds bounded local work")
    return report


def focus_gate(root):
    report = verify(root)
    if timing.read(root / "manifest.json")["pass_phase"] != "focus" or report["stopped_reason"] is not None:
        raise ValueError("focus capture is not complete")
    controls = {f"random-{i:03}" for i in contract()["successful_focus_controls"]}
    if any(not r["result"]["verified_landing"] for r in report["rows"] if r["case_id"] in controls):
        raise ValueError("focus successful control regressed")
    old, _ = timing.baseline(contract())
    gains = [r["case_id"] for r in report["rows"] if r["result"]["verified_landing"]
             and timing.read(old / "runs" / r["case_id"] / "flight.json")["planning_stop"] != "landed"]
    if not gains:
        raise ValueError("focus has no new verified landings")
    return report, gains


def native_check(binary, root):
    result = subprocess.run([str(binary), "check-planner-v2", "--dir", str(root)],
                            capture_output=True, timeout=60)
    if result.returncode != 0:
        raise ValueError("benchmark native check failed: " + result.stderr.decode(errors="replace"))
    return json.loads(result.stdout)


def check_benchmark(args):
    binary, capture, output = args.binary.resolve(), args.capture.resolve(), args.output.resolve()
    source = survey.source_state(binary)
    checks = [native_check(binary, root) for root in (BENCHMARK, capture)]
    original, actual = (timing.read(root / "summary.json") for root in (BENCHMARK, capture))
    if (actual["input_identity"] != original["input_identity"]
            or actual["provenance"]["source_before"]["executable_sha256"] != source["executable_sha256"]
            or actual["provenance"]["source_before"] != actual["provenance"]["source_after"]
            or actual["provenance"]["unchanged_during_capture"] is not True):
        raise ValueError("benchmark source/input identity differs")
    survey.reserve(output)
    rows = []
    for row in original["cases"]:
        identifier = row["case_id"]
        if row["planning_stop"] != "landed":
            continue
        paths = [root / "runs" / identifier / "flight.json" for root in (BENCHMARK, capture)]
        for name, path in zip(("baseline", "actual"), paths):
            study.write_new(output / name / (identifier + ".json"),
                            study.encoded(without_query_diagnostics(timing.read(path))))
        exceptions = survey.comparison.compare_files(
            output / "actual" / (identifier + ".json"), output / "baseline" / (identifier + ".json"),
            survey.comparison.latest_contract(), binary)
        rows.append({"case_id": identifier, "source_sha256": [study.digest(p.read_bytes()) for p in paths],
                     "comparison_exceptions": exceptions})
    report = {"schema": "pd-lab.fallback-benchmark-preservation.v1", "capture": str(capture),
              "baseline": str(BENCHMARK), "source": source, "native_checks": checks,
              "summary_sha256": study.digest((capture / "summary.json").read_bytes()), "rows": rows,
              "comparison_contract": survey.comparison.latest_contract()}
    if source != survey.source_state(binary):
        raise ValueError("source drift during benchmark comparison")
    study.write_new(output / "preservation.json", study.encoded(report))
    study.write_new(output / "receipt.json", study.encoded({"files": survey.inventory(output)}))
    print(json.dumps({"benchmark_preservation": str(output), "landed_cases_preserved": len(rows),
                      "diagnostic_exceptions": sum(len(r["comparison_exceptions"]) for r in rows)}))


def run(args):
    if args.phase == "challenge":
        report, _ = focus_gate(args.focus_capture)
        source = survey.source_state(args.binary.resolve())
        audit = timing.read(args.benchmark_audit / "preservation.json")
        survey.check_inventory(args.benchmark_audit, timing.read(args.benchmark_audit / "receipt.json")["files"])
        if (audit["schema"] != "pd-lab.fallback-benchmark-preservation.v1"
                or audit["source"] != source or report["source_before"] != source
                or audit["summary_sha256"] != study.digest((Path(audit["capture"]) / "summary.json").read_bytes())
                or len(audit["rows"]) != 38):
            raise ValueError("conditional source-frozen focus/benchmark gate did not pass")
    timing.run(args, contract(), PLAN)
    report = verify(args.output.resolve())
    if args.phase == "focus":
        _, gains = focus_gate(args.output.resolve())
        print(json.dumps({"focus_gate": "passed", "new_verified_landings": gains}))
    else:
        print(json.dumps({"full_challenge_gate": "passed", "summary": report["summary"]}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    command = sub.add_parser("run")
    command.add_argument("--phase", choices=("focus", "challenge"), required=True)
    command.add_argument("--binary", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command.add_argument("--focus-capture", type=Path)
    command.add_argument("--benchmark-audit", type=Path)
    command = sub.add_parser("verify")
    command.add_argument("capture", type=Path)
    command = sub.add_parser("check-benchmark")
    command.add_argument("--binary", type=Path, required=True)
    command.add_argument("--capture", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        if args.phase == "challenge" and (args.focus_capture is None or args.benchmark_audit is None):
            parser.error("challenge requires --focus-capture and --benchmark-audit")
        run(args)
    elif args.command == "check-benchmark":
        check_benchmark(args)
    else:
        print(json.dumps({"verified": str(args.capture), "summary": verify(args.capture)["summary"], "fresh_flights": 0}))
