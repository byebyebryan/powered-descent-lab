"""Opt-in frozen-weight stopping study. Allocation and outputs are create-only."""
from __future__ import annotations

import argparse
import collections
from fractions import Fraction
import hashlib
import json
import shutil
import signal
import time
from pathlib import Path

import numpy as np
import torch
from stable_baselines3 import PPO

from coast_ignite import ArcSampler, NATIVE_ARTIFACT, encoded, native, sha, write_new
from stopping_policy import Stopper
from run_loop import binomial_interval


class Resources:
    def __init__(self, root, allocation):
        self.root, self.allocation = Path(root), allocation
        self.counts = collections.Counter()
        self.phase_counts = collections.Counter()
        self.phase_name = None
        self.phase_deadline = allocation["outer_deadline"]
        self.phase_max_steps = allocation["max_total_native_steps"]
        self.phase_max_original_steps = allocation["max_total_native_steps"]
        self.phase_max_bytes = allocation["max_bytes"]
        self.last_storage_check = 0
        signal.signal(signal.SIGALRM, self.interrupt)
        signal.setitimer(signal.ITIMER_REAL, max(.001, allocation["outer_deadline"] - time.time()))

    @staticmethod
    def interrupt(*_):
        raise RuntimeError("declared outer/phase wall budget exhausted")

    def phase(self, name, seconds, steps, max_bytes, original_steps=None):
        self.phase_name = name
        self.phase_counts = collections.Counter()
        self.phase_started = time.monotonic()
        self.phase_deadline = min(time.time() + seconds, self.allocation["outer_deadline"])
        self.phase_max_steps = steps
        self.phase_max_original_steps = original_steps if original_steps is not None else steps
        self.phase_max_bytes = max_bytes
        self.phase_start_bytes = self.storage_bytes()
        write_new(self.root / ("allocation-" + name + ".json"),
                  {"seconds": seconds, "total_native_steps": steps,
                   "original_native_steps": self.phase_max_original_steps,
                   "new_bytes": max_bytes, "deadline": self.phase_deadline,
                   "carried_resource_counts": dict(self.counts)})
        signal.setitimer(signal.ITIMER_REAL, max(.001, self.phase_deadline - time.time()))
        self.check()

    def count(self, key, value):
        self.counts[key] += value
        self.phase_counts[key] += value

    @staticmethod
    def steps(counts):
        return sum(v for k, v in counts.items() if k.endswith("_native_steps"))

    def remaining_steps(self):
        return max(0, min(self.allocation["max_total_native_steps"] - self.steps(self.counts),
                          self.phase_max_steps - self.steps(self.phase_counts)))

    def storage_bytes(self):
        return sum(p.stat().st_size for p in self.root.rglob("*") if p.is_file())

    def check(self):
        if time.time() >= self.phase_deadline or self.remaining_steps() < 0:
            raise RuntimeError("wall/native budget exhausted")
        if self.steps(self.counts) > self.allocation["max_total_native_steps"]:
            raise RuntimeError("outer native budget exhausted")
        if self.steps(self.phase_counts) > self.phase_max_steps:
            raise RuntimeError("phase native budget exhausted")
        if self.phase_counts["original_native_steps"] > self.phase_max_original_steps:
            raise RuntimeError("pilot original-step budget exhausted")
        if self.counts["original_episodes"] > self.allocation["max_original_episodes"]:
            raise RuntimeError("original episode allocation exhausted")
        if shutil.disk_usage(self.root).free < 50 * 2**30:
            raise RuntimeError("free-space reserve exhausted")
        if time.monotonic() - self.last_storage_check > 5:
            size = self.storage_bytes()
            self.last_storage_check = time.monotonic()
            if size > self.allocation["max_bytes"] or size - self.phase_start_bytes > self.phase_max_bytes:
                raise RuntimeError("artifact budget exhausted")

    def receipt(self):
        return {"phase": self.phase_name, "resource_counts": dict(self.counts),
                "phase_resource_counts": dict(self.phase_counts),
                "phase_wall_seconds": time.monotonic() - self.phase_started,
                "bytes": self.storage_bytes()}


def execute(task, task_path, rule, seed, replicate, allocation, budget, model=None,
            output=None, schedule=None, category="original"):
    model_spec = allocation["models"].get(str(seed)) if seed else None
    descriptor = {"schema_version": 1, "implementation": "coast-stopping-v1",
                  "checkpoint_sha256": model_spec["sha256"] if model_spec else None,
                  "configuration": {"rule": rule, "replicate": replicate,
                  "randomization_key": allocation["randomization_key"],
                  "reference_end": task["reference_end"], "physics_hz": 120,
                  "controller_hz": 60, "policy_hz": 10,
                  "controller_backend": "polynomial_landing_v2",
                  "native_artifact_sha256": sha(NATIVE_ARTIFACT)}}
    inputs = [str(task_path), str(NATIVE_ARTIFACT), str(Path(__file__).resolve()),
              str(budget.root / "execution-allocation.json")]
    if model_spec:
        inputs.append(model_spec["path"])
    ep = native.CoastEpisode(encoded(task["scenario"]), task["reference_end"],
                             output is not None, encoded(descriptor), inputs)
    stopper = Stopper(rule, task["physical_sha256"], allocation["randomization_key"], replicate)
    budget.count(category + "_episodes", 1)
    previous_steps = previous_checks = previous_constructions = 0
    while True:
        physical = json.loads(ep.physical_json())
        tick = physical["physics_step"]
        if schedule is not None:
            j = len(json.loads(ep.status_json())["decisions"])
            if schedule[j][0] != tick:
                raise RuntimeError("saved controller schedule is off-clock")
            action = schedule[j][1]
        else:
            action = stopper.choose(model, ep.observation(), tick)
        ep.set_work_limit(budget.remaining_steps())
        try:
            _, _, done, raw = ep.step(action)
        except BaseException:
            partial = json.loads(ep.status_json())
            budget.count(category + "_native_steps", partial["physics_steps"] - previous_steps)
            write_new(budget.root / ("incomplete-" + str(time.time_ns()) + ".json"), partial)
            raise
        info = json.loads(raw)
        budget.count(category + "_native_steps", info["physics_steps"] - previous_steps)
        previous_steps = info["physics_steps"]
        checks = (info["feedback"] or {}).get("admission_actuation_checks", 0)
        constructions = (info["feedback"] or {}).get("constructions", 0)
        budget.count("admission_checks", checks - previous_checks)
        budget.count("reference_constructions", constructions - previous_constructions)
        previous_checks, previous_constructions = checks, constructions
        budget.count(category + "_policy_decisions", 1)
        budget.check()
        if done:
            break
    if output is not None:
        # save_capture performs three neutral command replays. Reserve all first.
        if budget.remaining_steps() < 3 * info["physics_steps"]:
            raise RuntimeError("insufficient reserved neutral replay work")
        result = json.loads(ep.save_capture(str(output)))
        budget.count("neutral_replay_native_steps", 3 * info["physics_steps"])
        budget.count("neutral_replay_passes", 3)
        info = result["info"]
        verified = result["verified_target_landing"]
    else:
        result = json.loads(ep.finish_json())
        info, verified = result["info"], None
    diagnostic = json.loads(ep.diagnostics_json())
    ref = diagnostic.get("reference_projection") or {}
    budget.count("diagnostic_reference_grid_queries", (ref.get("reference") or {}).get("physics_ticks", -5) + 5)
    budget.check()
    return info, diagnostic, stopper.trace, verified


def capture(task, task_path, rule, seed, replicate, allocation, budget, output, model):
    info, diagnostic, trace, verified = execute(task, task_path, rule, seed, replicate,
                                               allocation, budget, model, output)
    write_new(output / "diagnostics.json", diagnostic)
    stopping = {"schema_version": 1, "rule": rule, "seed": seed, "replicate": replicate,
                "randomization_key": allocation["randomization_key"], "trace": trace}
    write_new(output / "stopping.json", stopping)
    for category, schedule in [("controller_reconstruction", info["decisions"]),
                               ("planner_reconstruction", None)]:
        reloaded = PPO.load(allocation["models"][str(seed)]["path"], device="cpu") if seed and schedule is None else model
        other, other_diagnostic, other_trace, _ = execute(
            task, task_path, rule, seed, replicate, allocation, budget, reloaded,
            schedule=schedule, category=category)
        if other != info or other_diagnostic != diagnostic or (schedule is None and other_trace != trace):
            raise RuntimeError(category + " does not reconstruct live physical/diagnostic execution")
    proof = {"physical_replay": True, "controller_reconstruction": True,
             "planner_reconstruction": True, "capture_sha256": sha(output / "evaluation.json"),
             "decisions_sha256": sha(output / "decisions.json"),
             "native_steps_each_pass": info["physics_steps"],
             "verified_target_landing": verified}
    write_new(output / "proof.json", proof)
    write_new(output / "stopping-proof.json", {**proof, "diagnostics_sha256": sha(output / "diagnostics.json"),
              "stopping_sha256": sha(output / "stopping.json"), "random_stream_reconstructed": True})
    return info, diagnostic, trace, verified


def jobs(count):
    result = []
    for arc in range(count):
        for seed in [281101, 281102, 281103]:
            for rule in ["G", "M", "S"]:
                for replicate in range(8 if rule == "S" else 1):
                    result.append({"arc": arc, "seed": seed, "rule": rule, "replicate": replicate})
        result.extend({"arc": arc, "seed": 0, "rule": rule, "replicate": 0}
                      for rule in ["delay", "immediate"])
    return result


def perform(job, tasks, allocation, budget, models, capture_root=None):
    task, path = tasks[job["arc"]]
    output = capture_root / ("case-" + str(job["arc"]) + "-" + str(job["seed"]) + "-" + job["rule"] + "-" + str(job["replicate"])) if capture_root else None
    args = (task, path, job["rule"], job["seed"], job["replicate"], allocation, budget)
    model = models.get(job["seed"])
    info, diagnostic, trace, verified = (capture(*args, output, model) if output else execute(*args, model=model))
    return {**job, "task": str(path), "physical_sha256": task["physical_sha256"],
            "capture": str(output) if output else None, "native_success": info["native_success"],
            "verified_target_landing": verified, "physics_steps": info["physics_steps"],
            "ignition_tick": next((tick for tick, action in info["decisions"] if action), None),
            "info": info, "diagnostics": diagnostic, "trace": trace}


def append(stream, row):
    stream.write(encoded(row) + "\n")
    stream.flush()


def summarize(rows, count):
    scores = {}
    for seed in [281101, 281102, 281103]:
        scores[str(seed)] = {}
        for rule in ["G", "M", "S"]:
            items = [r for r in rows if r["seed"] == seed and r["rule"] == rule]
            denominator = count * (8 if rule == "S" else 1)
            if len(items) != denominator:
                raise RuntimeError("incomplete phase inventory")
            scores[str(seed)][rule] = {"successes": sum(r["native_success"] for r in items),
                                      "occurrences": denominator, "independent_arcs": count}
    exact = {rule: sum(Fraction(scores[str(seed)][rule]["successes"], scores[str(seed)][rule]["occurrences"])
                      for seed in [281101, 281102, 281103]) / 3 for rule in ["G", "M", "S"]}
    winner = max(["G", "M", "S"], key=lambda rule: exact[rule])
    primary = scores["281103"]
    lift = exact[winner] - exact["G"]
    primary_lift = Fraction(primary[winner]["successes"], primary[winner]["occurrences"]) - Fraction(primary["G"]["successes"], primary["G"]["occurrences"])
    nominated = winner != "G" and lift >= Fraction(1, 200) and primary_lift >= Fraction(1, 200)
    return {"complete": True, "independent_arcs": count, "scores": scores,
            "aggregate_rates": {k: float(v) for k, v in exact.items()}, "winner": winner,
            "nomination_passed": nominated, "aggregate_lift": float(lift),
            "primary_lift": float(primary_lift), "optimizer_steps": 0}


def prepare(root, declaration, anchor, budget, seen):
    sampler = ArcSampler(root, anchor, declaration["seed"], declaration["cohort"], budget, seen,
                         requested_arcs=declaration["arcs"])
    tasks = [sampler.next() for _ in range(declaration["arcs"])]
    sampler.log.close()
    return tasks


def fresh_statistics(rows, allocation):
    primary = [r for r in rows if r["seed"] == 281103 and r["rule"] == "candidate" and r["replicate"] == 0]
    baseline = [r for r in rows if r["rule"] == "baseline"]
    if len(primary) != 1200 or len(baseline) != 1200:
        raise RuntimeError("incomplete primary confirmation")
    primary.sort(key=lambda r: r["arc"]); baseline.sort(key=lambda r: r["arc"])
    diffs = np.asarray([int(a["native_success"]) - int(b["native_success"]) for a,b in zip(primary, baseline)])
    gains, losses = int(sum(diffs == 1)), int(sum(diffs == -1))
    from math import comb
    discordant = gains + losses
    p = sum(comb(discordant, k) for k in range(gains, discordant + 1)) / 2**discordant if discordant else 1.
    rng = np.random.default_rng(allocation["bootstrap_seed"])
    # Each draw is a resample of 1,200 paired arcs; multinomial counts are equivalent.
    probabilities = np.asarray([sum(diffs == -1), sum(diffs == 0), sum(diffs == 1)]) / 1200
    bs = rng.multinomial(1200, probabilities, size=20000)
    ci = np.quantile((bs[:, 2] - bs[:, 0])/1200, [.025, .975]).tolist()
    successes = sum(r["native_success"] for r in primary)
    interval = binomial_interval(successes, 1200)
    reliability = successes >= 1194
    return {"complete": True, "primary_arcs": 1200, "primary_successes": successes,
            "baseline_successes": sum(r["native_success"] for r in baseline),
            "primary_exact_two_sided95": interval, "gains": gains, "losses": losses,
            "lift": float(diffs.mean()), "paired_bootstrap95": ci, "one_sided_p": p,
            "demonstrated_improvement": gains > losses and p < .05,
            "reliability99_one_sided95_passed": reliability}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    args = parser.parse_args()
    root = Path(args.root).resolve()
    allocation = json.loads((root / "execution-allocation.json").read_text())
    if (root / "campaign").exists():
        raise ValueError("campaign destination already exists; no implicit resume")
    if shutil.disk_usage(root).free < allocation["minimum_free_bytes"]:
        raise RuntimeError("insufficient free space before start")
    campaign = root / "campaign"; campaign.mkdir()
    torch.set_num_threads(1)
    budget = Resources(root, allocation)
    models = {}
    for seed, spec in allocation["models"].items():
        if sha(spec["path"]) != spec["sha256"]:
            raise RuntimeError("frozen model hash mismatch")
        models[int(seed)] = PPO.load(spec["path"], device="cpu")
    anchor = json.loads((Path(__file__).resolve().parents[2] / "fixtures/scenarios/flat_terminal_descent.json").read_text())
    old = Path(allocation["original_learning"])
    seen = set()
    for path in old.parents[1].rglob("arc-*.json"):
        value = json.loads(path.read_text())
        if "physical_sha256" in value:
            seen.add(value["physical_sha256"])
    write_new(campaign / "excluded-prior-identities.json", sorted(seen))
    report_rows = []
    try:
        budget.phase("preservation", 300, 1_000_000, 2*2**30)
        preservation = campaign / "preservation"; preservation.mkdir()
        with (preservation / "results.jsonl").open("x") as stream:
            for index in allocation["preservation_indices"]:
                path = old / f"fresh-inputs/arc-{index}.json";task = json.loads(path.read_text())
                row = perform({"arc":index,"seed":281103,"rule":"G","replicate":0},
                              {index:(task,path)}, allocation, budget, models, preservation)
                expected = json.loads((old/f"confirmation/case-{index}-rl/decisions.json").read_text())
                if row["info"] != expected:
                    raise RuntimeError("preservation differs from frozen physical/command fingerprint")
                append(stream,row);report_rows.append({**row,"population":"Preservation"})
        write_new(preservation / "result.json", {"complete":True,"exact_cases":16,**budget.receipt()})
        print("Preservation: 16/16 exact physical/decision fingerprints",flush=True)
        for phase in ["development", "validation"]:
            folder=campaign/phase;folder.mkdir()
            budget.phase(phase+"-inputs",60,0,2**30)
            tasks=prepare(folder/"inputs",allocation["streams"][phase],anchor,budget,seen)
            schedule=jobs(len(tasks));write_new(folder/"schedule.json",schedule)
            rows=[]
            with (folder/"results.jsonl").open("x") as stream:
                if phase=="development":
                    budget.phase("pilot",allocation["pilot_seconds"],8_000_000,allocation["pilot_bytes"],allocation["pilot_original_steps"])
                else:
                    budget.phase("validation",900,35_000_000,4*2**30)
                for index,job in enumerate(schedule):
                    if phase=="development" and index==128:
                        receipt=budget.receipt();write_new(folder/"pilot-result.json",receipt)
                        budget.phase("development",900,20_000_000,4*2**30)
                    captured=phase=="development" and job["arc"]<4
                    row=perform(job,tasks,allocation,budget,models,folder if captured else None)
                    append(stream,row);rows.append(row)
                    if captured:report_rows.append({**row,"population":"Development cost pilot"})
                    if (index+1)%512==0:print(phase,index+1,"/",len(schedule),flush=True)
            summary=summarize(rows,len(tasks));write_new(folder/"result.json",{**summary,**budget.receipt()})
            print(phase,"rates",summary["aggregate_rates"],"winner",summary["winner"],flush=True)
        selected=summary["winner"]
        freeze={"selected_rule":selected,"nomination_passed":summary["nomination_passed"],
                "validation":summary,"models":allocation["models"],"optimizer_steps":0,
                "native_source":json.loads(native.source_identity_json()),
                "native_sha256":sha(NATIVE_ARTIFACT),"allocation_sha256":sha(root/"execution-allocation.json")}
        write_new(campaign/"selection-freeze.json",freeze)
        budget.phase("validation-proof",300,1_000_000,2*2**30)
        proof_folder=campaign/"validation-proof";proof_folder.mkdir()
        for index in range(8):
            for rule,label in [(selected,"Selected rule"),("G","Current greedy")]:
                # Label is part of the capture path when the two rules coincide.
                target=proof_folder/label.replace(" ","-");target.mkdir(exist_ok=True)
                row=perform({"arc":index,"seed":281103,"rule":rule,"replicate":0},tasks,allocation,budget,models,target)
                report_rows.append({**row,"population":"Validation proof","display_label":label})
        write_new(proof_folder/"result.json",{"complete":True,"captured_occurrences":16,**budget.receipt()})
        if summary["nomination_passed"]:
            # Release only after proof readiness and explicit whole-fresh reservation.
            pilot=json.loads((campaign/"development/pilot-result.json").read_text())
            pilot_captures=list((campaign/"development").glob("case-*/evaluation.json"))
            max_case_bytes=max(sum(p.stat().st_size for p in x.parent.iterdir() if p.is_file()) for x in pilot_captures)
            fresh_count=6000+(384 if selected=="S" else 0)
            reservation={"occurrences":fresh_count,"maximum_native_steps":fresh_count*3604*6,
                         "capture_and_report_bytes":fresh_count*(max_case_bytes+4*2**20),
                         "pilot_measured_phase":pilot}
            if reservation["maximum_native_steps"]>allocation["max_total_native_steps"]-Resources.steps(budget.counts) or budget.storage_bytes()+reservation["capture_and_report_bytes"]>allocation["max_bytes"]:
                raise RuntimeError("fresh capture/proof/report reservation does not fit outer caps")
            write_new(campaign/"fresh-release.json",reservation)
            folder=campaign/"fresh";folder.mkdir()
            budget.phase("fresh",1800,150_000_000,40*2**30)
            tasks=prepare(folder/"inputs",allocation["streams"]["fresh"],anchor,budget,seen)
            schedule=[]
            for index in range(1200):
                schedule.extend({"arc":index,"seed":seed,"rule":selected,"role":"candidate","replicate":0} for seed in [281101,281102,281103])
                schedule.extend([{ "arc":index,"seed":281103,"rule":"G","role":"baseline","replicate":0},
                                 {"arc":index,"seed":0,"rule":"delay","role":"heuristic","replicate":0}])
                if selected=="S" and index<128:
                    schedule.extend({"arc":index,"seed":281103,"rule":"S","role":"secondary","replicate":replicate} for replicate in [1,2,3])
            write_new(folder/"schedule.json",schedule);fresh_rows=[]
            with (folder/"results.jsonl").open("x") as stream:
                for index,job in enumerate(schedule):
                    row=perform(job,tasks,allocation,budget,models,folder)
                    row["role"]=job["role"];append(stream,row);fresh_rows.append({**row,"rule":job["role"]})
                    report_rows.append({**row,"population":"Fresh confirmation"})
                    if (index+1)%256==0:print("fresh",index+1,"/",len(schedule),flush=True)
            result=fresh_statistics(fresh_rows,allocation);write_new(folder/"result.json",{**result,**budget.receipt()})
            verdict={"complete":True,"fresh_released":True,"selected_rule":selected,"fresh":result}
        else:
            verdict={"complete":True,"fresh_released":False,"selected_rule":selected,
                     "reason":"prospective validation nomination threshold not met; frozen current package retained"}
        write_new(campaign/"report-captures.json",report_rows)
        write_new(campaign/"result.json",{**verdict,"optimizer_steps":0,**budget.receipt()})
        print("Study complete",verdict,flush=True)
    except BaseException as exc:
        write_new(campaign/("fault-"+str(time.time_ns())+".json"),{"error":repr(exc),**budget.receipt()})
        raise
    finally:
        signal.setitimer(signal.ITIMER_REAL,0)


if __name__=="__main__":
    main()
