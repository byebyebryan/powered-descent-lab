"""Explicit frozen random survey; collect native flights, never select trajectories."""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
import math
from pathlib import Path
import random
import subprocess
import sys
import time

import refinement as ref
import study
import sentinel_comparison as comparison
import challenge

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
OUTPUTS = REPO / "outputs/eval/planner_v2_random_terrain"
PLAN = HERE / "survey_plan.json"


def load_plan(path=PLAN):
    plan = json.loads(path.read_bytes())
    refinement, base = ref.plans(HERE / "refinement_plan.json")
    if plan.get("schema") == challenge.SCHEMA:
        return challenge.validate(plan, refinement, base), refinement, base
    if (plan["schema"] != "pd-lab.random-terrain-survey-plan.v1"
            or plan["master_seed"] != 2026100601 or plan["seed_count"] != 100 or plan["seed_max"] != 2147483246
            or plan["excluded_seeds"] != base["seeds"]
            or plan["repeat_indices"] != list(range(5))
            or plan["sentinels"] != ["v2_clear_685", "v2_ridge_early", "v2_plateau_reference_900"]
            or [plan[k] for k in ("workers", "checkpoint_count", "case_wall_limit_s", "campaign_wall_limit_s", "maximum_measured_attempts", "policy_version")]
            != [4, 10, 300, 10800, 108, 3]
            or plan["control_plan_sha256"] != study.digest((HERE / "plan.json").read_bytes())
            or plan["refinement_plan_sha256"] != study.digest((HERE / "refinement_plan.json").read_bytes())):
        raise ValueError("changed frozen survey contract")
    return plan, refinement, base


def seeds_for(plan):
    if plan.get("schema") == challenge.SCHEMA:
        return challenge.seeds(plan)
    rng = random.Random(plan["master_seed"])
    used = set(plan["excluded_seeds"])
    seeds = []
    while len(seeds) < plan["seed_count"]:
        seed = rng.randrange(plan["seed_max"] + 1)
        if seed not in used:
            used.add(seed)
            seeds.append(seed)
    return seeds


def source_state(binary):
    files = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard"],
                           cwd=REPO, capture_output=True, check=True, text=True).stdout.splitlines()
    selected = [p for p in files if (p.startswith(("pd-core/", "pd-plan/", "pd-control/", "pd-eval/", "pd-report/"))
                                    and p.endswith((".rs", "Cargo.toml")))
                or p in ("Cargo.toml", "Cargo.lock")
                or (p.startswith("studies/terrain_profiles/") and p.endswith((".py", ".json")))]
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, check=True, text=True).stdout.strip()
    return {"git_commit": commit, "executable_sha256": study.digest(binary.read_bytes()),
            "files": {p: study.digest((REPO / p).read_bytes()) for p in sorted(selected)}}


def protected_state():
    """Accepted selection and its report bodies must survive the new survey."""
    site = REPO / "outputs/reports/eval/planner_v2_lab_suite"
    paths = sorted(p for p in site.rglob("*") if p.is_file())
    paths.append(REPO / "outputs/eval/planner_v2_lab_suite/current.json")
    return {str(p.relative_to(REPO)): study.digest(p.read_bytes()) for p in paths}


def sample(plan, base, refinement, root, reverse=False):
    if plan.get("schema") == challenge.SCHEMA:
        return challenge.sample(plan, base, refinement, root, reverse)
    cls = study.load_reference(root, base)
    seeds = seeds_for(plan)
    left, right = base["domain_m"]
    xs = list(range(left, right + 1, base["spacing_m"]))
    if reverse:
        seeds.reverse()
        xs.reverse()
    result = []
    for seed in seeds:
        generator = study.make_refined(cls, seed, base["refined"], base["weighted_strength"])
        generator._ridge_noise = ref.RidgeSlice(generator._ridge_noise, refinement["ridge_slice_offset"])
        points = sorted([x, generator(x + base["source_offset_m"])] for x in xs)
        if any(not math.isfinite(y) for _, y in points):
            raise ValueError("nonfinite generated terrain")
        result.append({"variant": "refined", "seed": seed, "points_m": points})
    return sorted(result, key=lambda p: p["seed"])


def prepared(profile, plan, refinement, base):
    if plan.get("schema") == challenge.SCHEMA:
        return challenge.prepared(profile, plan, refinement, base)
    template = json.loads((HERE / "scenario_template.json").read_bytes())
    value, geometry = ref.scenario(profile, refinement, base, template, PLAN.read_bytes())
    identifier = f'random_terrain_seed_{profile["seed"]}'
    value.update(id=identifier, name=identifier, description="Unseen random procedural terrain; fixed local pads, no corridor clearing.",
                 tags=["procedural_terrain", "random_survey"],
                 metadata={"study": plan["schema"], "master_seed": str(plan["master_seed"]),
                           "terrain_recipe": "pylander_refined_ridge_67.37"})
    points = [p for p in profile["points_m"] if 0 <= p[0] <= 1200]
    heights = [p[1] for p in points]
    geometry.update(relief_m=max(heights) - min(heights), endpoint_rise_m=heights[-1] - heights[0],
                    maximum_slope=max(abs((b[1] - a[1]) / (b[0] - a[0])) for a, b in zip(points, points[1:])))
    return value, geometry


def inventory(root):
    return {str(p.relative_to(root)): study.digest(p.read_bytes())
            for p in sorted(root.rglob("*")) if p.is_file() and p != root / "receipt.json"}


def check_inventory(root, files):
    for relative, expected in files.items():
        path = root / relative
        if (Path(relative).is_absolute() or ".." in Path(relative).parts or path.is_symlink()
                or not path.resolve().is_relative_to(root.resolve()) or study.digest(path.read_bytes()) != expected):
            raise ValueError(f"changed or unsafe artifact: {relative}")


def reserve(root):
    if not root.resolve().is_relative_to(OUTPUTS) or root.is_symlink():
        raise ValueError("survey capture must be beneath its isolated outputs/eval root")
    root.mkdir(parents=True, exist_ok=False)


def preflight(binary, path):
    result = subprocess.run([str(binary), "waypoint-v2-flight", "--scenario", str(path),
                             "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--preflight-only"],
                            capture_output=True, check=True, timeout=60)
    value = json.loads(result.stdout)
    if value != {"supported": True, "rejection": None, "reason": None, "simulation_created": False}:
        raise ValueError("unexpected native input-only preflight")
    return value


def prepare(args):
    plan_path = getattr(args, "plan", PLAN).resolve()
    if plan_path not in [PLAN.resolve(), *((HERE / challenge.plan_filename(phase)).resolve()
                                         for phase in ("calibration", "held_out", "validation_1k"))]:
        raise ValueError("unknown campaign plan source")
    plan, refinement, base = load_plan(plan_path)
    binary = args.binary.resolve()
    before = source_state(binary)
    # Authenticate the existing benchmark before copying its three controls.
    subprocess.run([str(binary), "check-planner-v2", "--dir", str(args.sentinels)], capture_output=True, check=True, timeout=60)
    old = json.loads((args.sentinels / "summary.json").read_bytes())
    reserve(args.output)
    manifest = {"schema": "pd-lab.random-terrain-inputs.v1", "master_seed": plan["master_seed"],
                "seeds": seeds_for(plan), "source": before, "protected": protected_state(),
                "sentinel_comparison": comparison.latest_contract(), "random_cases": [], "sentinel_cases": []}
    if plan_path != PLAN.resolve():
        manifest["plan_source_path"] = str(plan_path.relative_to(REPO))
    # Seed selection is persisted before any new terrain is generated/viewed.
    study.write_new(args.output / "seeds.json", study.encoded(manifest["seeds"]))
    study.write_new(args.output / "plan.json", plan_path.read_bytes())
    for path in ("study.py", "refinement.py", "survey.py", "sentinel_comparison.py", "sentinel_comparison.json", "sentinel_comparison_v2.json",
                 "plan.json", "refinement_plan.json", "survey_plan.json", "scenario_template.json",
                 "challenge.py", "challenge_calibration_plan.json", "challenge_plan.json", "challenge_validation_1k_plan.json"):
        study.write_new(args.output / "inputs/tools" / path, (HERE / path).read_bytes())
    for path in base["pylander"]["files"]:
        study.write_new(args.output / "inputs/reference" / path, (args.pylander_root / path).read_bytes())
    for path, digest in before["files"].items():
        data = (REPO / path).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed before snapshot")
        study.write_new(args.output / "inputs/source" / path, data)
    profiles = sample(plan, base, refinement, args.pylander_root)
    repeated = subprocess.run([sys.executable, "-B", str(HERE / "survey.py"), "sample", "--pylander-root",
                              str(args.output / "inputs/reference"), "--plan", str(args.output / "plan.json"), "--reverse"],
                             capture_output=True, check=True, timeout=1200 if plan.get("phase") == "validation_1k" else 120)
    if study.encoded(profiles) != study.encoded(json.loads(repeated.stdout)):
        raise ValueError("fresh-process reversed generation differs")
    lookup = {p["seed"]: p for p in profiles}
    for index, seed in enumerate(manifest["seeds"]):
        case_id = f"random-{index:03}"
        scenario, geometry = prepared(lookup[seed], plan, refinement, base)
        path = f"scenarios/{case_id}.json"
        study.write_new(args.output / f"raw/{case_id}.json", study.encoded(lookup[seed]))
        study.write_new(args.output / path, study.encoded(scenario))
        study.write_new(args.output / f"preflight/{case_id}.json", study.encoded(preflight(binary, args.output / path)))
        manifest["random_cases"].append({"case_id": case_id, "seed": seed, "scenario_path": path, "geometry": geometry})
    for case_id in plan["sentinels"]:
        case = next(c for c in old["cases"] if c["case_id"] == case_id)
        for name in ("scenario.json", "flight.json"):
            relative = f"runs/{case_id}/{name}"
            data = (args.sentinels / relative).read_bytes()
            if study.digest(data) != case["artifact_sha256"][relative]:
                raise ValueError("changed sentinel evidence")
            destination = f"scenarios/{case_id}.json" if name == "scenario.json" else f"controls/{case_id}.json"
            study.write_new(args.output / destination, data)
        manifest["sentinel_cases"].append({"case_id": case_id, "seed": None, "scenario_path": f"scenarios/{case_id}.json"})
    if source_state(binary) != before:
        raise ValueError("source changed during input preparation")
    manifest["generation_repeat_sha256"] = study.digest(study.encoded(profiles))
    study.write_new(args.output / "manifest.json", study.encoded(manifest))
    study.write_new(args.output / "inputs-receipt.json", study.encoded({"files": inventory(args.output)}))
    verify_inputs(args.output, binary)
    print(json.dumps({"capture": str(args.output), "random_inputs": plan["seed_count"], "native_preflights": plan["seed_count"], "flight_runs": 0}))


def input_comparison_contract(root, manifest):
    comparison.validate_contract(manifest.get("sentinel_comparison"))
    if "sentinel_comparison" in manifest:
        filename = comparison.contract_filename(manifest["sentinel_comparison"])
        frozen_contract = (root / "inputs/tools" / filename).read_bytes()
        contract_path = "studies/terrain_profiles/" + filename
        if (manifest["sentinel_comparison"] is None
                or json.loads(frozen_contract) != manifest["sentinel_comparison"]
                or manifest["source"]["files"].get(contract_path) != study.digest(frozen_contract)):
            raise ValueError("sentinel comparison contract is not source-bound")
    return manifest.get("sentinel_comparison")


def verify_inputs(root, binary=None):
    plan, refinement, base = load_plan(root / "plan.json")
    manifest = json.loads((root / "manifest.json").read_bytes())
    input_comparison_contract(root, manifest)
    check_inventory(root, json.loads((root / "inputs-receipt.json").read_bytes())["files"])
    plan_bytes = (root / "plan.json").read_bytes()
    if plan.get("schema") == challenge.SCHEMA:
        source_path = manifest.get("plan_source_path")
        expected_path = "studies/terrain_profiles/" + challenge.plan_filename(plan["phase"])
        if (source_path != expected_path or manifest["source"]["files"].get(source_path) != study.digest(plan_bytes)
                or (root / "inputs/source" / source_path).read_bytes() != plan_bytes):
            raise ValueError("challenge plan is not bound to frozen source")
    elif plan_bytes != PLAN.read_bytes():
        raise ValueError("changed historical survey contract")
    if (manifest["seeds"] != seeds_for(plan)
            or json.loads((root / "seeds.json").read_bytes()) != manifest["seeds"]
            or len(manifest["random_cases"]) != plan["seed_count"] or len(manifest["sentinel_cases"]) != 3):
        raise ValueError("input manifest differs from frozen plan")
    if binary is not None and source_state(binary) != manifest["source"]:
        raise ValueError("source/executable drift from prepared inputs")
    if protected_state() != manifest["protected"]:
        raise ValueError("accepted benchmark selection/report bodies changed")
    for index, row in enumerate(manifest["random_cases"]):
        if row["case_id"] != f"random-{index:03}" or row["seed"] != manifest["seeds"][index]:
            raise ValueError("seed identity/order differs")
        profile = json.loads((root / f'raw/{row["case_id"]}.json').read_bytes())
        variant = (plan["recipes"][index // plan["cases_per_recipe"]]["id"] if plan.get("schema") == challenge.SCHEMA else "refined")
        if profile["seed"] != row["seed"] or profile["variant"] != variant:
            raise ValueError("raw profile identity differs")
        scenario, geometry = prepared(profile, plan, refinement, base)
        if ((root / row["scenario_path"]).read_bytes() != study.encoded(scenario) or geometry != row["geometry"]
                or json.loads((root / f'preflight/{row["case_id"]}.json').read_bytes()) !=
                {"supported": True, "rejection": None, "reason": None, "simulation_created": False}):
            raise ValueError("scenario/pad derivation or native preflight differs")
    if [r["case_id"] for r in manifest["sentinel_cases"]] != plan["sentinels"]:
        raise ValueError("sentinel identities differ")
    return plan, manifest


def non_timing(flight):
    return comparison.non_timing(flight)


def projection(flight):
    first = flight["cycles"][0] if flight["cycles"] else None
    audit = first.get("audit") if first else None
    nominal = ("not_established" if audit is None else "clear" if audit["passed"]
               else "blocked" if flight["initial_nominal_terrain_blocked"] else "rejected")
    landed = (flight["planning_stop"] == "landed" and flight["physical_outcome"] == "landed_on_target"
              and flight["mission_outcome"] == "success" and flight["integrity_passed"] is True
              and flight["final_source_replay_passed"] is True)
    return {"nominal_class": nominal, "verified_landing": landed,
            **{k: flight[k] for k in ("planning_stop", "physical_outcome", "mission_outcome", "correction_count",
                                     "integrity_passed", "final_source_replay_passed")}}


def validate_record(flight, summary):
    if summary["schema_id"] != "waypoint_v2_flight_summary_v1" or summary["input_identity"] != flight["input_identity"] or summary["policy"] != flight["policy"]:
        raise ValueError("native summary input/policy disagreement")
    keys = {"planning_stop", "reason", "correction_count", "initial_nominal_terrain_blocked", "integrity_passed",
            "physical_outcome", "mission_outcome", "final_source_replay_passed", "timings"}
    if set(summary["result"]) != keys:
        raise ValueError("native compact result inventory differs")
    for key in keys:
        if flight.get(key) != summary["result"][key]:
            raise ValueError(f"native summary disagrees with flight: {key}")
    if (flight["policy"]["policy_id"] != "piecewise_local_clearing_v2_policy_3"
            or flight["integrity_passed"] is not True or flight["final_source_replay_passed"] is not True
            or flight["planning_stop"] in ("unsupported", "invalid_input", "implementation_error")):
        raise ValueError("native implementation/integrity/replay failure")
    return projection(flight)


def attempt(binary, root, row, limit, contract=None):
    output = root / "runs" / row["attempt_id"]
    command = [str(binary), "waypoint-v2-flight", "--scenario", str(root / row["scenario_path"]),
               "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--output-dir", str(output)]
    started = time.monotonic()
    try:
        result = subprocess.run(command, capture_output=True, timeout=limit)
        stdout, stderr, exit_code = result.stdout, result.stderr, result.returncode
        status = "recorded" if exit_code == 0 else "runner_error"
    except subprocess.TimeoutExpired as error:
        stdout, stderr, exit_code = error.stdout or b"", error.stderr or b"", None
        status = "runner_timeout"
    except OSError as error:
        stdout, stderr, exit_code = b"", str(error).encode(), None
        status = "runner_error"
    log = root / "logs" / row["attempt_id"]
    study.write_new(log.with_suffix(".stdout"), stdout)
    study.write_new(log.with_suffix(".stderr"), stderr)
    value = dict(row, status=status, exit_code=exit_code, wall_s=time.monotonic() - started,
                 output_dir=f'runs/{row["attempt_id"]}', result=None, failure=None)
    if status == "recorded":
        try:
            flight = json.loads((output / "flight.json").read_bytes())
            summary = json.loads((output / "summary.json").read_bytes())
            value["result"] = validate_record(flight, summary)
            printed = json.loads(stdout)
            if any(printed.get(k) != flight[k] for k in ("planning_stop", "correction_count", "integrity_passed", "final_source_replay_passed", "physical_outcome", "mission_outcome")):
                raise ValueError("CLI stdout disagrees with flight")
            if row["cohort"] == "sentinel":
                value["comparison_exceptions"] = comparison.compare_files(output / "flight.json", root / f'controls/{row["case_id"]}.json', contract, binary)
            elif row["cohort"] == "repeat":
                original = json.loads((root / f'runs/{row["case_id"]}/flight.json').read_bytes())
                value["comparison_exceptions"] = comparison.compare(flight, original)
        except (ValueError, KeyError, TypeError, OSError) as error:
            value.update(status="evidence_error", failure=str(error))
    return value


def summary_for(rows):
    primary = [r for r in rows if r["cohort"] == "random"]
    results = [r["result"] for r in primary if r.get("result") is not None and r["status"] == "recorded"]
    return {"primary_count": len(primary), "recorded_count": len(results),
            "verified_landings": sum(r["verified_landing"] for r in results),
            "dispositions": dict(sorted(Counter(r["status"] for r in primary).items())),
            "nominal_classes": dict(sorted(Counter(r["nominal_class"] for r in results).items())),
            "planning_stops": dict(sorted(Counter(r["planning_stop"] for r in results).items())),
            "physical_outcomes": dict(sorted(Counter(str(r["physical_outcome"]) for r in results).items())),
            "corrections": dict(sorted(Counter(str(r["correction_count"]) for r in results).items())),
            "sentinels_recorded": sum(r["cohort"] == "sentinel" and r["status"] == "recorded" for r in rows),
            "repeats_recorded": sum(r["cohort"] == "repeat" and r["status"] == "recorded" for r in rows)}


def waves_for(plan):
    first_end = 3 + min(plan["checkpoint_count"], plan["seed_count"])
    primary_end = 3 + plan["seed_count"]
    total = primary_end + len(plan["repeat_indices"])
    waves = [[i] for i in range(3)]
    for start, end in ((3, first_end), (first_end, primary_end), (primary_end, total)):
        waves += [list(range(a, min(a + plan["workers"], end))) for a in range(start, end, plan["workers"])]
    return waves


def run(args):
    binary = args.binary.resolve()
    plan, manifest = verify_inputs(args.capture, binary)
    rows = [dict(r, attempt_id=r["case_id"], cohort="sentinel", status="not_attempted") for r in manifest["sentinel_cases"]]
    rows += [dict(r, attempt_id=r["case_id"], cohort="random", status="not_attempted") for r in manifest["random_cases"]]
    rows += [dict(manifest["random_cases"][i], attempt_id=f"repeat-{i:03}", cohort="repeat", status="not_attempted") for i in plan["repeat_indices"]]
    study.write_new(args.capture / "run-start.json", study.encoded({"source": manifest["source"], "attempts": rows}))
    started = time.monotonic()
    stopped = None
    # No speculative submissions: each wave is fully checked before more launch.
    waves = waves_for(plan)
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        for wave in waves:
            if source_state(binary) != manifest["source"]:
                stopped = "source/executable drift"
                break
            remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - started)
            if remaining <= 0:
                stopped = "campaign wall budget exhausted"
                break
            futures = [(i, pool.submit(attempt, binary, args.capture, rows[i], min(plan["case_wall_limit_s"], remaining),
                                      manifest.get("sentinel_comparison"))) for i in wave]
            for index, future in futures:
                rows[index] = future.result()
                study.write_new(args.capture / f'ledger/{rows[index]["attempt_id"]}.json', study.encoded(rows[index]))
                print(json.dumps({k: rows[index].get(k) for k in ("attempt_id", "cohort", "status", "result", "failure", "comparison_exceptions")}), flush=True)
            failures = [rows[i] for i in wave if rows[i]["status"] != "recorded"]
            if failures:
                stopped = f'{failures[0]["attempt_id"]}: {failures[0]["failure"] or failures[0]["status"]}'
                break
    after = source_state(binary)
    if after != manifest["source"]:
        stopped = stopped or "source/executable drift"
    report = {"schema": "pd-lab.random-terrain-survey.v1", "manifest_sha256": study.digest((args.capture / "manifest.json").read_bytes()),
              "source_before": manifest["source"], "source_after": after, "stopped_reason": stopped,
              "rows": rows, "summary": summary_for(rows)}
    study.write_new(args.capture / "survey.json", study.encoded(report))
    study.write_new(args.capture / "receipt.json", study.encoded({"files": inventory(args.capture)}))
    verify(args.capture)
    print(json.dumps({"capture": str(args.capture), "summary": report["summary"], "stopped_reason": stopped}), flush=True)
    if stopped:
        raise RuntimeError("survey stopped; all evidence retained")


def verify(root):
    plan, manifest = verify_inputs(root)
    receipt = json.loads((root / "receipt.json").read_bytes())
    if inventory(root) != receipt["files"]:
        raise ValueError("survey inventory differs")
    check_inventory(root, receipt["files"])
    report = json.loads((root / "survey.json").read_bytes())
    if (report["schema"] != "pd-lab.random-terrain-survey.v1" or len(report["rows"]) != plan["maximum_measured_attempts"]
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())
            or report["source_before"] != manifest["source"]
            or (report["stopped_reason"] is None and report["source_after"] != report["source_before"])):
        raise ValueError("survey identity/count disagreement")
    expected_rows = [(r, "sentinel", r["case_id"]) for r in manifest["sentinel_cases"]]
    expected_rows += [(r, "random", r["case_id"]) for r in manifest["random_cases"]]
    expected_rows += [(manifest["random_cases"][i], "repeat", f"repeat-{i:03}") for i in plan["repeat_indices"]]
    for row, (expected, cohort, attempt_id) in zip(report["rows"], expected_rows):
        if (row["attempt_id"] != attempt_id or row["cohort"] != cohort
                or any(row[k] != expected[k] for k in ("case_id", "seed", "scenario_path"))
                or row.get("geometry") != expected.get("geometry")
                or (row["status"] != "recorded" and report["stopped_reason"] is None)):
            raise ValueError("survey row differs from frozen input/order")
        if row["status"] == "recorded":
            flight = json.loads((root / row["output_dir"] / "flight.json").read_bytes())
            compact = json.loads((root / row["output_dir"] / "summary.json").read_bytes())
            if validate_record(flight, compact) != row["result"]:
                raise ValueError("survey projection differs from full flight")
            if (root / row["output_dir"] / "scenario.json").read_bytes() != (root / row["scenario_path"]).read_bytes():
                # Native serialization is typed, so compare values rather than formatting.
                if json.loads((root / row["output_dir"] / "scenario.json").read_bytes()) != json.loads((root / row["scenario_path"]).read_bytes()):
                    raise ValueError("flight used another scenario")
            if row["cohort"] != "random":
                reference = (root / "controls" / f'{row["case_id"]}.json' if row["cohort"] == "sentinel"
                             else root / "runs" / row["case_id"] / "flight.json")
                contract = manifest.get("sentinel_comparison") if row["cohort"] == "sentinel" else None
                exceptions = comparison.compare_files(root / row["output_dir"] / "flight.json", reference, contract)
                if study.encoded(exceptions) != study.encoded(row.get("comparison_exceptions", [])):
                    raise ValueError("recorded comparison exception list disagrees")
    if summary_for(report["rows"]) != report["summary"]:
        raise ValueError("survey totals differ from rows")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    cmd = sub.add_parser("sample")
    cmd.add_argument("--pylander-root", type=Path, required=True)
    cmd.add_argument("--reverse", action="store_true")
    cmd.add_argument("--plan", type=Path, default=PLAN)
    cmd = sub.add_parser("prepare")
    cmd.add_argument("--pylander-root", type=Path, required=True)
    cmd.add_argument("--binary", type=Path, required=True)
    cmd.add_argument("--sentinels", type=Path, required=True)
    cmd.add_argument("--output", type=Path, required=True)
    cmd.add_argument("--plan", type=Path, default=PLAN)
    cmd = sub.add_parser("run")
    cmd.add_argument("--capture", type=Path, required=True)
    cmd.add_argument("--binary", type=Path, required=True)
    cmd = sub.add_parser("verify")
    cmd.add_argument("capture", type=Path)
    args = parser.parse_args()
    if args.command == "sample":
        plan, refinement, base = load_plan(args.plan)
        print(study.encoded(sample(plan, base, refinement, args.pylander_root, args.reverse)).decode(), end="")
    elif args.command == "prepare":
        prepare(args)
    elif args.command == "run":
        run(args)
    else:
        report = verify(args.capture)
        print(json.dumps({"verified": str(args.capture), "summary": report["summary"], "fresh_flights": 0}))


if __name__ == "__main__":
    main()
