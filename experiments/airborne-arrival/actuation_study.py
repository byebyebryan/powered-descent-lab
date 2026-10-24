"""Opt-in allocation comparison and independent witness proof."""
from pathlib import Path
import json

from arrival import Budget, compact, encoded, native, program, read, sha, write_new


def scenario(case):
    return read(case["scenario_path"])


def goal_program(case, budget=None):
    s = scenario(case)
    if "recipe" not in case:
        return program(s, case["choice"])
    generated = json.loads(native.generate_witness(encoded(s), encoded(case["recipe"])))
    if budget:
        budget.counts["native_steps"] += generated["native_steps"]
        budget.counts["goal_generation_steps"] = budget.counts.get("goal_generation_steps", 0) + generated["native_steps"]
    if not generated["admitted"] or generated["request"] != case["program"]["arrival"]:
        raise RuntimeError("witness goal reconstruction differs")
    return {"arrival": generated["request"], "land": False}


def execute(budget, case, mode, captured=False, inputs=None, forced=None, category="originals"):
    budget.check()
    p = forced if forced is not None else goal_program(case, budget)
    descriptor = {"schema_version": 1, "implementation": "arrival-actuation-study-v1",
                  "checkpoint_sha256": None, "configuration": {"mode": mode, "program": p,
                  "native_library_sha256": sha(native.__file__), "case_id": case["id"],
                  "witness_recipe": case.get("recipe") if mode == "witness" else None}}
    ep = native.ArrivalEpisode(encoded(scenario(case)), encoded(p), captured,
        encoded(descriptor), inputs or [],
        actuation_profile=None if mode == "witness" else mode,
        witness_recipe_json=encoded(case["recipe"]) if mode == "witness" else None)
    budget.counts[category] += 1
    try:
        ep.execute(budget.remaining())
    except BaseException:
        budget.consume(json.loads(ep.status_json()))
        raise
    return ep, p


def summary(budget, case, mode):
    ep, _ = execute(budget, case, mode)
    result = json.loads(ep.finish_json())
    budget.consume(result["info"])
    return result


def capture(budget, case, path, mode, output, expected, allocation):
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    inputs = [str(path), str(allocation), case["scenario_path"], str(Path(native.__file__))]
    ep, p = execute(budget, case, mode, True, inputs)
    status = json.loads(ep.status_json())
    budget.consume(status)
    if budget.remaining() < 3 * status["physics_steps"] + 2_000_002:
        raise RuntimeError("proof reservation exhausted")
    budget.counts["neutral_replay_steps"] += 3 * status["physics_steps"]
    saved = json.loads(ep.save_capture(str(output)))
    info = saved["info"]
    if info != expected:
        raise RuntimeError("Summary/Captured differs")
    for forced in [p, None]:
        rebuilt, _ = execute(budget, case, mode, forced=forced, category="reconstructions")
        actual = json.loads(rebuilt.finish_json())["info"]
        budget.consume(actual)
        if actual != info:
            raise RuntimeError("controller/goal reconstruction differs")
    proof = {"physical_replay": True, "controller_reconstruction": True, "planner_reconstruction": True,
             "summary_equality": True, "capture_sha256": sha(output / "evaluation.json"),
             "decisions_sha256": sha(output / "decisions.json"),
             "verified_target_landing": saved["verified_target_landing"], "mode": mode,
             "planner_kind": "independent_native_witness_recipe" if "recipe" in case else "retained_input_goal_rule"}
    write_new(output / "proof.json", proof)
    return {"capture": str(output), "case": case["id"], "mode": mode, **proof}


def finding(rows):
    result = {}
    for mode in sorted({r["mode"] for r in rows}):
        group = [r for r in rows if r["mode"] == mode]
        result[mode] = {"occurrences": len(group),
            "arrival_attempts": sum(r["result"]["arrival_status"] != "direct" for r in group),
            "reached": sum(r["result"]["arrival_status"] == "reached" for r in group),
            "native_successes": sum(r["result"]["success"] for r in group),
            "reached_and_native_success": sum(r["result"]["arrival_status"] == "reached" and r["result"]["success"] for r in group)}
    return result
