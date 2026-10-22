"""Opt-in, create-only timing characterization with unchanged native execution."""
from __future__ import annotations

import argparse
import collections
import json
import shutil
import signal
import time
from pathlib import Path

import torch
from stable_baselines3 import PPO

from coast_ignite import NATIVE_ARTIFACT, encoded, native, sha, write_new
from stopping_policy import Stopper
from timing_analysis import SLOTS, cohort_summary, fixed_action, map_summary, metric, paired_reduction, read, slot_from_info, slot_order


class Resources:
    def __init__(self, root, allocation):
        self.root, self.allocation = Path(root), allocation
        self.counts = collections.Counter()
        self.phase_counts = collections.Counter()
        self.deadline = allocation["outer_deadline"]
        self.phase_steps = allocation["max_total_native_steps"]
        self.phase_bytes = allocation["max_bytes"]
        self.phase_start_bytes = self.storage_bytes()
        self.phase_name = "startup"
        self.last_storage_check = 0.0
        signal.signal(signal.SIGALRM, self.interrupt)
        signal.setitimer(signal.ITIMER_REAL, max(.001, self.deadline - time.time()))

    @staticmethod
    def interrupt(*_):
        raise RuntimeError("declared wall budget exhausted")

    @staticmethod
    def steps(counts):
        return sum(v for k, v in counts.items() if k.endswith("_native_steps"))

    def storage_bytes(self):
        return sum(p.stat().st_size for p in self.root.rglob("*") if p.is_file())

    def count(self, name, value):
        self.counts[name] += value
        self.phase_counts[name] += value

    def remaining_steps(self):
        return max(0, min(self.allocation["max_total_native_steps"] - self.steps(self.counts), self.phase_steps - self.steps(self.phase_counts)))

    def phase(self, name, seconds, steps, max_bytes):
        self.phase_name = name
        self.phase_counts = collections.Counter()
        self.deadline = min(self.allocation["outer_deadline"], time.time() + seconds)
        self.phase_steps, self.phase_bytes = steps, max_bytes
        self.phase_start_bytes = self.storage_bytes()
        write_new(self.root / ("allocation-" + name + ".json"), {"deadline": self.deadline,
            "total_native_steps": steps, "new_bytes": max_bytes, "carried_counts": dict(self.counts)})
        signal.setitimer(signal.ITIMER_REAL, max(.001, self.deadline - time.time()))
        self.check()

    def check(self):
        if time.time() >= self.deadline:
            raise RuntimeError("wall budget exhausted")
        if self.steps(self.counts) > self.allocation["max_total_native_steps"] or self.steps(self.phase_counts) > self.phase_steps:
            raise RuntimeError("native step budget exhausted")
        if self.counts["original_episodes"] > self.allocation["max_original_episodes"]:
            raise RuntimeError("original occurrence budget exhausted")
        if shutil.disk_usage(self.root).free < self.allocation["min_free_bytes"]:
            raise RuntimeError("free space reserve exhausted")
        if time.monotonic() - self.last_storage_check > 5:
            self.last_storage_check = time.monotonic()
            size = self.storage_bytes()
            if size > self.allocation["max_bytes"] or size - self.phase_start_bytes > self.phase_bytes:
                raise RuntimeError("artifact budget exhausted")

    def receipt(self):
        return {"phase": self.phase_name, "resource_counts": dict(self.counts),
                "phase_resource_counts": dict(self.phase_counts), "bytes": self.storage_bytes()}


def execute(case, task, allocation, budget, slot=None, rule=None, model=None,
            output=None, forced=None, category="original"):
    budget.check()
    slot_order(slot)
    configuration = {"kind": "frozen_policy" if rule else "fixed_slot",
        "requested_slot": slot, "decoder": rule, "reference_end": task["reference_end"],
        "physics_hz": 120, "controller_hz": 60, "policy_hz": 10,
        "controller_backend": "polynomial_landing_v2", "native_artifact_sha256": sha(NATIVE_ARTIFACT)}
    descriptor = {"schema_version": 1, "implementation": "coast-timing-map-v1",
        "checkpoint_sha256": allocation["models"]["281103"]["sha256"] if rule else None,
        "configuration": configuration}
    inputs = [case["task_path"], str(budget.root / "execution-allocation.json"),
              str(NATIVE_ARTIFACT), str(Path(__file__).resolve())]
    if rule:
        inputs.append(allocation["models"]["281103"]["path"])
    episode = native.CoastEpisode(encoded(task["scenario"]), task["reference_end"], output is not None, encoded(descriptor), inputs)
    stopper = Stopper(rule, task["physical_sha256"], allocation["randomization_key"]) if rule else None
    budget.count(category + "_episodes", 1)
    previous_steps = previous_checks = previous_constructions = 0
    while True:
        physical = json.loads(episode.physical_json())
        tick = physical["physics_step"]
        if forced is not None:
            j = len(json.loads(episode.status_json())["decisions"])
            if j >= len(forced) or forced[j][0] != tick:
                raise RuntimeError("saved controller schedule is off clock")
            action = forced[j][1]
        elif stopper is not None:
            action = stopper.choose(model, episode.observation(), tick)
        else:
            action = fixed_action(slot, tick)
        episode.set_work_limit(budget.remaining_steps())
        try:
            _, _, done, raw = episode.step(action)
        except BaseException:
            partial = json.loads(episode.status_json())
            budget.count(category + "_native_steps", partial["physics_steps"] - previous_steps)
            write_new(budget.root / ("incomplete-" + str(time.time_ns()) + ".json"), partial)
            raise
        info = json.loads(raw)
        budget.count(category + "_native_steps", info["physics_steps"] - previous_steps)
        previous_steps = info["physics_steps"]
        feedback = info["feedback"] or {}
        checks, constructions = feedback.get("admission_actuation_checks", 0), feedback.get("constructions", 0)
        budget.count("admission_checks", checks - previous_checks)
        budget.count("reference_constructions", constructions - previous_constructions)
        previous_checks, previous_constructions = checks, constructions
        budget.count(category + "_policy_decisions", 1)
        budget.check()
        if done:
            break
    if output is not None:
        if budget.remaining_steps() < 3 * info["physics_steps"]:
            raise RuntimeError("neutral replay reservation exhausted")
        # These three complete passes are native save_capture's maintained proof contract.
        budget.count("neutral_replay_native_steps", 3 * info["physics_steps"])
        budget.count("neutral_replay_passes", 3)
        result = json.loads(episode.save_capture(str(output)))
        info, verified = result["info"], result["verified_target_landing"]
    else:
        result = json.loads(episode.finish_json())
        info, verified = result["info"], None
    diagnostic = json.loads(episode.diagnostics_json())
    reference = (diagnostic.get("reference_projection") or {}).get("reference")
    budget.count("diagnostic_reference_grid_queries", reference["physics_ticks"] + 5 if reference else 0)
    budget.check()
    return {"id": case["id"], "cohort": case["cohort"], "arc": case["arc"],
        "slot": slot_from_info(info) if rule else slot, "rule": rule,
        "physical_sha256": task["physical_sha256"], "native_success": info["native_success"],
        "verified_target_landing": verified, "info": info, "diagnostics": diagnostic,
        "metrics": metric(task, info, diagnostic), "trace": stopper.trace if stopper else [],
        "capture": str(output) if output is not None else None}


def capture(case, task, allocation, budget, output, expected, slot=None, rule=None, model=None):
    result = execute(case, task, allocation, budget, slot, rule, model, output)
    if result["info"] != expected["info"]:
        raise RuntimeError("capture differs from original Summary/retained policy")
    if "diagnostics" in expected and result["diagnostics"] != expected["diagnostics"]:
        raise RuntimeError("capture diagnostic differs from original Summary")
    for category, forced in [("controller_reconstruction", result["info"]["decisions"]),
                             ("planner_reconstruction", None)]:
        reloaded = PPO.load(allocation["models"]["281103"]["path"], device="cpu") if rule and forced is None else model
        other = execute(case, task, allocation, budget, slot, rule, reloaded, forced=forced, category=category)
        if any(other[k] != result[k] for k in ["info", "diagnostics", "metrics"]):
            raise RuntimeError(category + " does not reproduce physical and diagnostic execution")
        if forced is None and other["trace"] != result["trace"]:
            raise RuntimeError("live policy/scheduler trace differs")
    write_new(output / "diagnostics.json", result["diagnostics"])
    write_new(output / "timing.json", {"requested_slot": slot, "rule": rule,
        "metrics": result["metrics"], "trace": result["trace"], "scheduler": "fixed_slot" if not rule else "frozen_policy"})
    proof = {"physical_replay": True, "controller_reconstruction": True,
        "planner_reconstruction": True, "planner_kind": "frozen_model_" + rule if rule else "fixed_slot_scheduler",
        "summary_equality": True, "native_steps_each_pass": result["info"]["physics_steps"],
        "capture_sha256": sha(output / "evaluation.json"), "decisions_sha256": sha(output / "decisions.json"),
        "verified_target_landing": result["verified_target_landing"]}
    write_new(output / "proof.json", proof)
    write_new(output / "timing-proof.json", {**proof, "diagnostics_sha256": sha(output / "diagnostics.json"),
        "timing_sha256": sha(output / "timing.json")})
    return result


def append(stream, row):
    stream.write(encoded(row) + "\n")
    stream.flush()


def capture_key(case, slot, rule):
    return case["id"] + "-" + (rule or ("never" if slot is None else f"slot-{slot:02d}"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    args = parser.parse_args()
    root = Path(args.root).resolve()
    allocation = read(root / "execution-allocation.json")
    if sha(NATIVE_ARTIFACT) != allocation["native_artifact_sha256"]:
        raise RuntimeError("native artifact does not match sealed allocation")
    for path, expected_hash in allocation["retained_hashes"].items():
        if sha(path) != expected_hash:
            raise RuntimeError("retained input/model/source changed: " + path)
    for spec in allocation["models"].values():
        if sha(spec["path"]) != spec["sha256"]:
            raise RuntimeError("frozen model changed")
    torch.set_num_threads(1)
    campaign = root / "campaign"
    campaign.mkdir()
    (campaign / "captures").mkdir()
    write_new(campaign / "paired-efficiency.json", paired_reduction(allocation["old_root"]))
    budget = Resources(root, allocation)
    cases = allocation["cases"]
    tasks = {c["id"]: read(c["task_path"]) for c in cases}
    maps, summaries, captures, roles = {}, [], {}, []
    model = PPO.load(allocation["models"]["281103"]["path"], device="cpu")
    summary_stream = (campaign / "results.jsonl").open("x")
    capture_stream = (campaign / "captures.jsonl").open("x")

    def full(case, slot, role, expected, rule=None):
        key = capture_key(case, slot, rule)
        if key not in captures:
            if len(captures) >= 256:
                raise RuntimeError("capture quota exhausted")
            result = capture(case, tasks[case["id"]], allocation, budget, campaign / "captures" / key,
                             expected, slot, rule, model if rule else None)
            captures[key] = result
            append(capture_stream, result)
        roles.append({"id": case["id"], "role": role, "capture_key": key,
                      "slot": captures[key]["slot"]})
        return captures[key]

    def scan(case):
        rows = []
        for slot in SLOTS:
            row = execute(case, tasks[case["id"]], allocation, budget, slot=slot)
            append(summary_stream, row)
            rows.append(row)
        summary = map_summary(case, rows)
        maps[case["id"]] = rows
        summaries.append(summary)
        write_new(campaign / "maps" / (case["id"] + ".json"), summary)
        print("map", case["id"], "powered successes", summary["successful_powered_schedules"], flush=True)

    try:
        pilot_started = time.monotonic()
        budget.phase("pilot", 300, 5_000_000, 2**30)
        for case in cases[:4]:
            for rule in ["M", "G"]:
                full(case, slot_from_info(case["policies"][rule]["info"]), "preservation-" + rule,
                     case["policies"][rule], rule=rule)
            scan(case)
            for slot in [0, 30, 60, None]:
                full(case, slot, "pilot-" + str(slot), next(r for r in maps[case["id"]] if r["slot"] == slot))
        from render_timing_map import calibrate
        calibration = calibrate(root, list(captures.values()))
        budget.check()
        pilot = {"complete": True, "arcs": 4, "summaries": 328,
                 "captures": len(captures), "elapsed_seconds": time.monotonic() - pilot_started,
                 "calibration": calibration, **budget.receipt()}
        write_new(campaign / "pilot-result.json", pilot)
        max_capture = max(sum(p.stat().st_size for p in Path(r["capture"]).iterdir() if p.is_file()) for r in captures.values())
        remaining_captures = 256 - len(captures)
        reservation = {"remaining_summaries": (158 - 4) * 82,
            "remaining_capture_maximum": remaining_captures,
            "native_steps": (158 - 4) * 82 * 4800 + remaining_captures * 6 * 4800,
            "new_bytes": int(1.10 * (remaining_captures * (max_capture + calibration["maximum_rich_bundle_bytes"] + 128 * 1024)
                                     + (158 - 4) * 82 * 32768 + 32 * 2**20)),
            "pilot_result_sha256": sha(campaign / "pilot-result.json")}
        if reservation["native_steps"] > allocation["max_total_native_steps"] - Resources.steps(budget.counts):
            raise RuntimeError("full native reservation does not fit")
        if reservation["new_bytes"] + budget.storage_bytes() > allocation["max_bytes"]:
            raise RuntimeError("full capture/report byte reservation does not fit")
        seconds_per_arc = pilot["elapsed_seconds"] / 4
        if time.time() + seconds_per_arc * 154 * 1.5 + 300 > allocation["outer_deadline"]:
            raise RuntimeError("full time reservation does not fit")
        write_new(campaign / "full-release.json", reservation)
        budget.phase("complete-maps", allocation["outer_deadline"] - time.time(),
                     allocation["max_total_native_steps"] - Resources.steps(budget.counts),
                     allocation["max_bytes"] - budget.storage_bytes())
        for case in cases[4:]:
            scan(case)
        write_new(campaign / "map-summary.json", {"maps": summaries, "cohorts": cohort_summary(summaries)})
        for case, summary in zip(cases, summaries):
            rows = maps[case["id"]]
            if summary["has_successful_point"]:
                slot = summary["best_slot"]
                full(case, slot, "best", next(r for r in rows if r["slot"] == slot))
            if case["cohort"] == "diagnostic":
                for rule in ["M", "G"]:
                    slot = slot_from_info(case["policies"][rule]["info"])
                    full(case, slot, "policy-" + rule, next(r for r in rows if r["slot"] == slot), rule=rule)
                if not case["policies"]["M"]["native_success"] and summary["has_successful_point"] and summary["best_slot"] is not None:
                    for slot in [summary["best_slot"] - 1, summary["best_slot"] + 1]:
                        if 0 <= slot <= 80:
                            full(case, slot, "best-neighbor", next(r for r in rows if r["slot"] == slot))
        write_new(campaign / "capture-roles.json", roles)
        write_new(campaign / "report-captures.json", list(captures.values()))
        write_new(campaign / "result.json", {"complete": True, "original_summary_executions": len(cases) * 82,
            "captured_occurrences": len(captures), "optimizer_steps": 0,
            "cohorts": cohort_summary(summaries), **budget.receipt()})
        print("COMPLETE", encoded({"maps": len(summaries), "captures": len(captures), **budget.receipt()}), flush=True)
    except BaseException as exc:
        write_new(campaign / ("fault-" + str(time.time_ns()) + ".json"), {"error": repr(exc), **budget.receipt()})
        raise
    finally:
        summary_stream.close()
        capture_stream.close()
        signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == "__main__":
    main()
