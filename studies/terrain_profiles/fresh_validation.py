"""Fresh frozen terrain validation using the retained early-exit executable.

No planner build, tuning, outcome filtering, retries or report-site publication.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import sys
import time

import cap_probe as cap
import challenge
import diagnostics as d
import early_exit_sweep as exits
import refinement as ref
import study
import survey

PLAN = survey.HERE / "fresh_validation_plan.json"
PLAN_SHA256 = "92db2eea735f79fb51f8ecd3df3ac93eadb426682878e61646d230ddac98133f"


def contract(path=PLAN):
    if study.digest(path.read_bytes()) != PLAN_SHA256:
        raise ValueError("changed frozen fresh validation contract")
    plan = d.read(path)
    previous = d.read(survey.HERE / "challenge_plan.json")
    if (plan != d.read(PLAN) or plan["schema"] != "pd-lab.terrain-fresh-validation-plan.v1"
            or [plan[k] for k in ("master_seed", "seed_count", "cases_per_recipe", "maximum_measured_attempts", "maximum_corrections")]
            != [2026100801, 100, 25, 108, 24]
            or plan["recipes"] != previous["recipes"] or plan["retries"] != 0 or plan["publication"]):
        raise ValueError("changed fresh validation contract")
    return plan


def excluded_seeds():
    base = d.read(survey.HERE / "plan.json")
    used = set(base["seeds"])
    used.update(survey.seeds_for(d.read(survey.HERE / "survey_plan.json")))
    for name in ("challenge_calibration_plan.json", "challenge_plan.json", "challenge_validation_1k_plan.json"):
        used.update(challenge.seeds(d.read(survey.HERE / name)))
    return used


def seeds(plan):
    return challenge.draw(plan["master_seed"], plan["seed_count"], excluded_seeds())


def profiles(plan, reference, reverse=False):
    refinement, base = ref.plans(survey.HERE / "refinement_plan.json")
    cls = study.load_reference(reference, base)
    xs = list(range(base["domain_m"][0], base["domain_m"][1] + 1, base["spacing_m"]))
    assignments = list(enumerate(seeds(plan)))
    if reverse:
        assignments.reverse()
        xs.reverse()
    result = {}
    for index, seed in assignments:
        descriptor = plan["recipes"][index // plan["cases_per_recipe"]]
        generator = study.make_refined(cls, seed, challenge.recipe(base, descriptor), base["weighted_strength"])
        generator._ridge_noise = ref.RidgeSlice(generator._ridge_noise, refinement["ridge_slice_offset"])
        result[seed] = {"variant": descriptor["id"], "seed": seed,
                        "points_m": sorted([x, generator(x + base["source_offset_m"])] for x in xs)}
    return [result[seed] for seed in seeds(plan)]


def baseline(plan):
    root = survey.REPO / plan["baseline_capture"]
    receipt = d.safe_file(root, "receipt.json").read_bytes()
    if study.digest(receipt) != plan["baseline_receipt_sha256"]:
        raise ValueError("changed retained baseline receipt")
    inventory = json.loads(receipt)["files"]

    def bound(path):
        data = d.safe_file(root, path).read_bytes()
        if inventory.get(path) != study.digest(data):
            raise ValueError("changed retained artifact: " + path)
        return data

    binary = bound("candidate/bin/pd-eval")
    seal = json.loads(bound("candidate/source-seal.json"))
    if (study.digest(binary) != plan["candidate_executable_sha256"]
            or seal["executable_sha256"] != plan["candidate_executable_sha256"]):
        raise ValueError("wrong retained executable")
    return root, bound, binary, seal


def attempts(cases, plan):
    rows = [dict(c, attempt_id=c["case_id"], status="not_attempted") for c in cases]
    primary = [c for c in cases if c["cohort"] == "random"]
    rows += [dict(primary[i], cohort="repeat", attempt_id=f"repeat-{i:03}", status="not_attempted")
             for i in plan["repeat_indices"]]
    if len(rows) != plan["maximum_measured_attempts"]:
        raise ValueError("wrong attempt allowance")
    return rows


def compare(actual, previous):
    # Same exact executable and input: no numerical/proof/hash exceptions.
    if survey.non_timing(actual) != survey.non_timing(previous):
        raise ValueError("same-executable complete non-timing flight differs")


def check_record(root, row, plan):
    output = d.safe_file(root, "runs/" + row["attempt_id"])
    flight = d.read(output / "flight.json")
    result = exits.validate_native(flight, d.read(output / "summary.json"), plan)
    if (row["exit_code"] != 0 or row["output_dir"] != "runs/" + row["attempt_id"]
            or not exits.same_scenario((output / "scenario.json").read_bytes(),
                                       d.safe_file(root, row["scenario_path"]).read_bytes())):
        raise ValueError("native input/output identity differs")
    printed = d.read(root / "logs" / (row["attempt_id"] + ".stdout"))
    for key in d.read(output / "summary.json")["result"]:
        if printed.get(key) != flight[key]:
            raise ValueError("native stdout differs: " + key)
    if row["cohort"] == "sentinel":
        compare(flight, d.read(root / "controls" / (row["case_id"] + ".json")))
        if not result["verified_landing"]:
            raise ValueError("preservation control lost its landing")
    if row["cohort"] == "repeat":
        compare(flight, d.read(root / "runs" / row["case_id"] / "flight.json"))
    return result


def attempt(root, row, plan, limit):
    command = [str(root / "candidate/bin/pd-eval"), "waypoint-v2-flight", "--scenario", str(root / row["scenario_path"]),
               "--source-pad-id", "pad_source", "--target-pad-id", "pad_main", "--output-dir", str(root / "runs" / row["attempt_id"])]
    started = time.monotonic()
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
                  result=None, failure=None, wall_s=time.monotonic() - started)
    if status == "recorded":
        try:
            actual["result"] = check_record(root, actual, plan)
        except (ValueError, KeyError, TypeError, OSError) as error:
            actual.update(status="evidence_error", failure=str(error))
    return actual


def verify(root):
    plan = contract(root / "plan.json")
    receipt = d.read(root / "receipt.json")["files"]
    if survey.inventory(root) != receipt:
        raise ValueError("capture inventory differs")
    survey.check_inventory(root, receipt)
    manifest, report = d.read(root / "manifest.json"), d.read(root / "fresh-validation.json")
    # Older captures bind their retained source seal through the source-pinned
    # manifest. New captures additionally carry a portable baseline receipt.
    if (root / "baseline-receipt.json").exists():
        saved_receipt = (root / "baseline-receipt.json").read_bytes()
        if study.digest(saved_receipt) != plan["baseline_receipt_sha256"]:
            raise ValueError("retained baseline receipt differs")
        baseline_files = json.loads(saved_receipt)["files"]
        for cid in plan["sentinels"]:
            if (study.digest((root / "controls" / (cid + ".json")).read_bytes()) != baseline_files[f"runs/{cid}/flight.json"]
                    or study.digest((root / "scenarios" / (cid + ".json")).read_bytes()) != baseline_files[f"scenarios/{cid}.json"]):
                raise ValueError("control differs from authenticated retained baseline")
    else:
        # The first closed edition predates the portable receipt copy. Verify
        # its references against the retained, contract-pinned receipt instead
        # of modifying or resealing that immutable capture.
        _, bound, _, _ = baseline(plan)
        if (root / "candidate/source-seal.json").read_bytes() != bound("candidate/source-seal.json"):
            raise ValueError("native source seal is not authenticated by the retained receipt")
        for cid in plan["sentinels"]:
            if ((root / "controls" / (cid + ".json")).read_bytes() != bound(f"runs/{cid}/flight.json")
                    or (root / "scenarios" / (cid + ".json")).read_bytes() != bound(f"scenarios/{cid}.json")):
                raise ValueError("control differs from authenticated retained baseline")
    if (manifest["seeds"] != seeds(plan) or set(manifest["seeds"]) & excluded_seeds()
            or manifest["excluded_seeds"] != sorted(excluded_seeds())
            or d.read(root / "seeds.json") != manifest["seeds"]
            or report["manifest_sha256"] != study.digest((root / "manifest.json").read_bytes())):
        raise ValueError("seed or manifest binding differs")
    seal = d.read(root / "candidate/source-seal.json")
    if (seal["executable_sha256"] != plan["candidate_executable_sha256"]
            or cap.variant_state(root / "candidate", seal["files"]) != seal
            or manifest["native_source_seal_sha256"] != study.digest((root / "candidate/source-seal.json").read_bytes())):
        raise ValueError("retained native executable/source differs")
    for path, digest in manifest["operator_source"]["files"].items():
        if study.digest(d.safe_file(root / "operator/source", path).read_bytes()) != digest:
            raise ValueError("collector source snapshot differs")
    refinement, base = ref.plans(survey.HERE / "refinement_plan.json")
    random_cases = [c for c in manifest["cases"] if c["cohort"] == "random"]
    if len(random_cases) != 100 or [c["case_id"] for c in manifest["cases"][:3]] != plan["sentinels"]:
        raise ValueError("input inventory differs")
    for i, case in enumerate(random_cases):
        profile = d.read(root / "raw" / (case["case_id"] + ".json"))
        scenario, geometry = challenge.prepared(profile, plan, refinement, base)
        if (case["case_id"] != f"random-{i:03}" or case["seed"] != manifest["seeds"][i]
                or profile["variant"] != plan["recipes"][i // 25]["id"]
                or geometry != case["geometry"] or study.encoded(scenario) != (root / case["scenario_path"]).read_bytes()
                or d.read(root / "preflight" / (case["case_id"] + ".json")) !=
                {"supported": True, "rejection": None, "reason": None, "simulation_created": False}):
            raise ValueError("frozen scenario derivation differs")
    expected = attempts(manifest["cases"], plan)
    if len(report["rows"]) != len(expected):
        raise ValueError("attempt inventory differs")
    for frozen, row in zip(expected, report["rows"]):
        if any(row[k] != v for k, v in frozen.items() if k != "status"):
            raise ValueError("attempt identity/order differs")
        if row["status"] == "recorded" and check_record(root, row, plan) != row["result"]:
            raise ValueError("saved result differs")
        if report["stopped_reason"] is None and row["status"] != "recorded":
            raise ValueError("incomplete capture marked complete")
    if survey.summary_for(report["rows"]) != report["summary"]:
        raise ValueError("saved summary differs")
    if report["stopped_reason"] is None and (report["operator_source_after"] != manifest["operator_source"]
                                             or report["protected_after"] != manifest["protected"]):
        raise ValueError("source/protected evidence drift")
    return report


def run(args):
    plan = contract()
    previous, bound, binary, seal = baseline(plan)
    root = args.output.resolve()
    source, protected = cap.main_state(), survey.protected_state()
    survey.reserve(root)
    study.write_new(root / "plan.json", PLAN.read_bytes())
    study.write_new(root / "seeds.json", study.encoded(seeds(plan)))
    study.write_new(root / "baseline-receipt.json", (previous / "receipt.json").read_bytes())
    study.write_new(root / "candidate/bin/pd-eval", binary)
    (root / "candidate/bin/pd-eval").chmod(0o755)
    study.write_new(root / "candidate/source-seal.json", bound("candidate/source-seal.json"))
    for path in seal["files"]:
        study.write_new(root / "candidate/source" / path, bound("candidate/source/" + path))
    for path, digest in source["files"].items():
        data = d.safe_file(survey.REPO, path).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("collector source preparation drift")
        study.write_new(root / "operator/source" / path, data)
    refinement, base = ref.plans(survey.HERE / "refinement_plan.json")
    for path in base["pylander"]["files"]:
        study.write_new(root / "reference" / path, d.safe_file(args.pylander_root, path).read_bytes())
    native = root / "candidate/bin/pd-eval"
    generated = profiles(plan, root / "reference")
    repeated = subprocess.run([sys.executable, "-B", str(Path(__file__).resolve()), "sample", "--reference", str(root / "reference"), "--reverse"],
                              capture_output=True, check=True, timeout=120)
    if study.encoded(generated) != study.encoded(json.loads(repeated.stdout)):
        raise ValueError("reversed fresh-process terrain generation differs")
    cases = []
    for cid in plan["sentinels"]:
        path = "scenarios/" + cid + ".json"
        study.write_new(root / path, bound(path))
        study.write_new(root / "controls" / (cid + ".json"), bound("runs/" + cid + "/flight.json"))
        cases.append(dict(case_id=cid, scenario_path=path, seed=None, cohort="sentinel"))
    for i, profile in enumerate(generated):
        cid, path = f"random-{i:03}", f"scenarios/random-{i:03}.json"
        scenario, geometry = challenge.prepared(profile, plan, refinement, base)
        study.write_new(root / "raw" / (cid + ".json"), study.encoded(profile))
        study.write_new(root / path, study.encoded(scenario))
        cases.append(dict(case_id=cid, scenario_path=path, seed=profile["seed"], geometry=geometry, cohort="random"))
    for case in cases:
        study.write_new(root / "preflight" / (case["case_id"] + ".json"), study.encoded(survey.preflight(native, root / case["scenario_path"])))
    manifest = {"schema": "pd-lab.terrain-fresh-validation-inputs.v1", "seeds": seeds(plan),
                "excluded_seeds": sorted(excluded_seeds()), "cases": cases, "operator_source": source,
                "protected": protected, "native_source_seal_sha256": study.digest((root / "candidate/source-seal.json").read_bytes()),
                "generation_repeat_sha256": study.digest(study.encoded(generated))}
    study.write_new(root / "manifest.json", study.encoded(manifest))
    rows = attempts(cases, plan)
    study.write_new(root / "run-start.json", study.encoded(rows))
    started, stopped = time.monotonic(), None
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        waves = [[i] for i in range(3)] + [list(range(i, min(i + 4, 103))) for i in range(3, 103, 4)] + [[i] for i in range(103, 108)]
        for wave in waves:
            if cap.main_state() != source or survey.protected_state() != protected:
                stopped = "source or protected evidence drift"
                break
            remaining = plan["campaign_wall_limit_s"] - (time.monotonic() - started)
            if remaining <= 0:
                stopped = "campaign wall bound"
                break
            futures = [(i, pool.submit(attempt, root, rows[i], plan, min(remaining, plan["case_wall_limit_s"]))) for i in wave]
            for i, future in futures:
                rows[i] = future.result()
                study.write_new(root / "ledger" / (rows[i]["attempt_id"] + ".json"), study.encoded(rows[i]))
                print(json.dumps({k: rows[i].get(k) for k in ("attempt_id", "status", "result", "failure")}), flush=True)
            if any(rows[i]["status"] != "recorded" for i in wave):
                stopped = "native execution/evidence failure"
                break
    after, protected_after = cap.main_state(), survey.protected_state()
    if after != source or protected_after != protected:
        stopped = stopped or "source or protected evidence drift"
    report = {"schema": "pd-lab.terrain-fresh-validation-results.v1", "rows": rows,
              "summary": survey.summary_for(rows), "stopped_reason": stopped,
              "operator_source_after": after, "protected_after": protected_after,
              "manifest_sha256": study.digest((root / "manifest.json").read_bytes())}
    study.write_new(root / "fresh-validation.json", study.encoded(report))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    verify(root)
    print(json.dumps({"capture": str(root), "summary": report["summary"], "stopped_reason": stopped}), flush=True)
    if stopped:
        raise RuntimeError("fresh validation stopped; evidence retained")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    collect = commands.add_parser("run")
    collect.add_argument("--pylander-root", type=Path, required=True)
    collect.add_argument("--output", type=Path, required=True)
    check = commands.add_parser("verify")
    check.add_argument("capture", type=Path)
    sample = commands.add_parser("sample")
    sample.add_argument("--reference", type=Path, required=True)
    sample.add_argument("--reverse", action="store_true")
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    elif args.command == "sample":
        print(study.encoded(profiles(contract(), args.reference, args.reverse)).decode(), end="")
    else:
        report = verify(args.capture)
        print(json.dumps({"verified": str(args.capture), "summary": report["summary"], "fresh_flights": 0}))
