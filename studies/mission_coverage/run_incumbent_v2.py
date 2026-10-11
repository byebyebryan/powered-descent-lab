"""Fixed existing-input development baseline; never rebuilds or tunes a controller."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
DECLARATIONS = ROOT / "studies/mission_coverage/incumbent_v2"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_new(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def run_native(command, log, deadline, campaign=None):
    with log.open("x") as stream:
        process = subprocess.Popen(["rtk", "proxy", *map(str, command)], cwd=ROOT,
                                   stdout=stream, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        try:
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("declared two-hour campaign/replay limit reached")
                try:
                    code = process.wait(timeout=min(30, remaining))
                    break
                except subprocess.TimeoutExpired:
                    count = len(list(campaign.glob("attempt-*.json"))) if campaign and campaign.exists() else 0
                    print(f"{log.stem}: {count} saved attempt dispositions; still running", flush=True)
            if code:
                raise RuntimeError(f"{log.stem} exited {code}; retained log {log}")
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()


def wilson(landed, total):
    if not total:
        return None
    z = 1.959963984540054
    p = landed / total
    denominator = 1 + z*z/total
    center = (p + z*z/(2*total))/denominator
    radius = z*((p*(1-p)/total + z*z/(4*total*total))**0.5)/denominator
    return [center-radius, center+radius]


def partition(rows):
    counts = {}
    for row in rows:
        if row["verified_target_landing"]:
            key = "verified_landing"
        elif row["disposition"] != "executed":
            key = row["disposition"]
        elif row["physical_outcome"] == "flying":
            key = "stopped_before_departure" if row["sim_time_s"] == 0 else "stopped_airborne"
        else:
            key = row["physical_outcome"]
        counts[key] = counts.get(key, 0) + 1
    return counts


def analyze(page, details):
    rows = page["attempts"]
    outcomes = partition(rows)
    landings = outcomes.get("verified_landing", 0)
    supported = sum(row["disposition"] == "executed" for row in rows)
    def groups(key):
        result = {}
        for row in rows:
            result.setdefault(key(row), []).append(row)
        return {key: {"scheduled": len(items), "outcomes": partition(items)} for key, items in result.items()}
    stops = {}
    for detail in details:
        key = str(detail["approach_stop"])
        stops[key] = stops.get(key, 0) + 1
    return {"scheduled": len(rows), "executed": supported, "outcomes": outcomes,
            "verified_landing_rate": landings/len(rows), "wilson_95": wilson(landings, len(rows)),
            "support_conditional_rate": landings/supported if supported else None,
            "directions": groups(lambda r: str(r["direction"])),
            "morphologies": groups(lambda r: r["geometry"]["morphology"]),
            "barrier_bands": groups(lambda r: "<100" if r["geometry"]["chord_barrier_m"] < 100 else "100–<200" if r["geometry"]["chord_barrier_m"] < 200 else "200–<400" if r["geometry"]["chord_barrier_m"] < 400 else "≥400"),
            "approach_stops": stops}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    declaration_path = DECLARATIONS / "declaration.json"
    declaration = json.loads(declaration_path.read_text())
    for path, expected in declaration["binaries"].items():
        if digest(ROOT/path) != expected:
            raise ValueError(f"frozen executable changed: {path}")
    for path, expected in declaration["requests"].items():
        if digest(ROOT/path) != expected:
            raise ValueError(f"frozen request changed: {path}")
    if digest(Path(__file__).resolve()) != declaration["runner_sha256"]:
        raise ValueError("frozen runner source changed")
    for entry in declaration["populations"]:
        if digest(ROOT/entry["collection"]/"manifest.json") != entry["manifest_sha256"]:
            raise ValueError("original collection manifest changed")
    output.mkdir()
    started = time.monotonic()
    deadline = started + declaration["wall_limit_s"]
    write_new(output/"run-start.json", {"declaration_sha256":digest(declaration_path),
              "started_utc":time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
              "git_head":subprocess.check_output(["rtk", "proxy", "git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()})
    result = {"schema_version":1, "completed":False, "new_inputs":0, "training_transitions":0, "populations":[]}
    benchmark = ROOT/"target/release/pd-benchmark"
    verifier = ROOT/"target/release/examples/verify-collection"
    refine = ROOT/"target/release/examples/refine_comparison"

    def evaluate(population, mode, stage):
        name = population["id"]
        parent = output/(f"integration-{name}" if stage == "integration" else name)
        parent.mkdir(exist_ok=True)
        campaign = parent/mode
        request = DECLARATIONS/f"{stage}-{name}-{mode}.json"
        command = [benchmark, "--collection", ROOT/population["collection"], "--output", campaign,
                   "--request", request, "--correction-cap", "24"]
        if mode == "reflection":
            command.append("--mirror-leftward")
        run_native(command, output/f"{stage}-{name}-{mode}.log", deadline, campaign)
        receipt = parent/f"{mode}-verification.json"
        run_native([verifier, ROOT/population["collection"], campaign, receipt],
                   output/f"{stage}-{name}-{mode}-verification.log", deadline)
        verified = json.loads(receipt.read_text())
        if not verified["all_passed"] or verified["failures"]:
            raise ValueError(f"integration/evidence failure in {stage}/{name}/{mode}")
        print(f"{stage}/{name}/{mode}: {verified['verified_target_landings']}/{verified['scheduled']} verified landings, {verified['unsupported']} unsupported", flush=True)
        return verified

    try:
        for population in declaration["populations"]:
            for mode in ["raw", "reflection"]:
                evaluate(population, mode, "integration")
        print("All 32 integration dispositions and independent replay checks passed; launching main fixed schedules.", flush=True)
        jobs = [(population, mode) for population in declaration["populations"] for mode in ["raw", "reflection"]]
        with ThreadPoolExecutor(max_workers=2) as pool:
            proofs = list(pool.map(lambda job:evaluate(*job, "main"), jobs))
        for population in declaration["populations"]:
            name = population["id"]
            parent = output/name
            pages = {mode:json.loads((parent/mode/"report-input.json").read_text()) for mode in ["raw", "reflection"]}
            by_index = {mode:{r["mission_index"]:r for r in page["attempts"]} for mode, page in pages.items()}
            if set(by_index["raw"]) != set(by_index["reflection"]):
                raise ValueError("paired mission schedules differ")
            exact_right = 0
            for index, row in by_index["raw"].items():
                other = by_index["reflection"][index]
                if row["physical_sha256"] != other["physical_sha256"] or row["geometry"] != other["geometry"] or row["direction"] != other["direction"]:
                    raise ValueError("paired input metadata differs")
                if row["direction"] > 0:
                    for key in ["disposition", "stop", "physical_outcome", "mission_outcome", "verified_target_landing", "sim_time_s", "fuel_kg"]:
                        if row[key] != other[key]:
                            raise ValueError(f"rightward raw/reflection differs at {index}: {key}")
                    exact_right += 1
            metrics = {mode:analyze(page, json.loads((parent/mode/"approach-attempts.json").read_text())) for mode, page in pages.items()}
            metrics["rightward_outcomes_exact"] = exact_right
            metrics["proposal_accounting"] = population["proposal_accounting"]
            metrics["interpretation"] = "Existing exposed development inputs; backend rates separate; not a held-out qualification or solvability result."
            write_new(parent/"metrics.json", metrics)
            comparison = {"schema_version":1, "title":f"Challenging V2 · {name} · unchanged cap-24 controller · development baseline",
                          "provenance_href":"metrics.json", "approaches":[
                {"label":f"Incumbent {mode} · cap 24", "campaigns":[{"page":pages[mode], "report_href":f"{mode}/report.html"}]}
                for mode in ["reflection", "raw"]]}
            write_new(parent/"comparison-input.json", comparison)
            selected_proofs = [p for p in proofs if Path(p["directory"]).parent == parent]
            write_new(parent/"independent-verification.json", {"schema_version":1,"all_passed":True,"campaigns":selected_proofs})
            edition = output/f"{name}-review"
            source_href = "/" + str(parent.relative_to(ROOT/"outputs")) + "/"
            edition_href = "/" + str(edition.relative_to(ROOT/"outputs")) + "/"
            run_native([refine, parent, edition, source_href, edition_href], output/f"{name}-render.log", time.monotonic()+900)
            result["populations"].append({"id":name,"metrics":metrics,"review":str(edition/"report.html")})
        result["completed"] = True
    except Exception as error:
        result["error"] = str(error)
        raise
    finally:
        result["elapsed_s"] = time.monotonic()-started
        result["frozen_binaries_unchanged"] = all(digest(ROOT/path) == expected for path, expected in declaration["binaries"].items())
        write_new(output/"run-result.json", result)
    print(f"Completed baseline and rich reports: {output}", flush=True)


if __name__ == "__main__":
    main()
