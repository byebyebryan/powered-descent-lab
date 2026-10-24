"""First-resolution composition with exact prefix and complete native proof."""
from pathlib import Path
import hashlib
import json

from arrival import Budget, compact, encoded, native, program, read, sha, write_new


def goal_program(case, budget=None):
    if "recipe" not in case:
        return program(read(case["scenario_path"]), case["choice"])
    generated = json.loads(native.generate_witness(encoded(read(case["scenario_path"])), encoded(case["recipe"])))
    if budget is not None:
        budget.counts["native_steps"] += generated["native_steps"]
        budget.counts["goal_generation_steps"] = budget.counts.get("goal_generation_steps", 0) + generated["native_steps"]
    if not generated["admitted"] or generated["request"] != case["program"]["arrival"]:
        raise RuntimeError("independent witness goal reconstruction differs")
    return {"arrival": generated["request"], "land": case["program"]["land"]}


def execute(budget, case, mode, captured=False, inputs=None, forced=None, category="originals"):
    budget.check()
    p = goal_program(case, budget) if forced is None else forced
    descriptor = {"schema_version": 1, "implementation": "arrival-first-resolution-composition-v1",
        "checkpoint_sha256": None, "configuration": {"mode": mode, "program": p,
        "native_library_sha256": sha(native.__file__), "case_id": case["id"],
        "witness_recipe": case.get("recipe") if mode == "witness" else None}}
    ep = native.ArrivalEpisode(encoded(read(case["scenario_path"])), encoded(p), captured,
        encoded(descriptor), inputs or [], actuation_profile=None if mode == "witness" else mode,
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


def prefix_projection(evaluation, tick):
    execution = evaluation["execution"]
    return {"samples": [s for s in execution["episode"]["run"]["samples"] if s["physics_step"] <= tick],
        "updates": [u for u in execution["controller_updates"] if u["physics_step"] < tick]}


def check_prefix(folder, original):
    folder, original = Path(folder), Path(original)
    before, after = read(original / "decisions.json"), read(folder / "decisions.json")
    if before["arrival"] != after["arrival"]:
        raise RuntimeError("arrival feedback differs from retained prefix")
    tick = before["arrival"]["resolution"]["physics_step"]
    old_e, new_e = read(original / "evaluation.json"), read(folder / "evaluation.json")
    if old_e["scenario"] != new_e["scenario"]:
        raise RuntimeError("prefix scenario differs")
    old, new = prefix_projection(old_e, tick), prefix_projection(new_e, tick)
    if old != new:
        raise RuntimeError("native observations/commands differ before first resolution")
    handoff = after["arrival"]["resolution"]
    if after["landing_search"]["selection_observation"] != handoff:
        raise RuntimeError("landing selection did not start at the actual first resolution")
    return {"passed": True, "original_capture": str(original), "original_sha256": sha(original / "evaluation.json"),
        "resolution_tick": tick, "native_samples": len(new["samples"]), "controller_updates": len(new["updates"]),
        "prefix_sha256": hashlib.sha256(encoded(new).encode()).hexdigest(), "handoff": handoff}


def capture(budget, case, path, mode, output, expected, allocation):
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    inputs = [str(path), str(allocation), case["scenario_path"], str(Path(native.__file__))]
    for identity in case.get("prefix_captures", {}).values():
        inputs += [str(Path(identity["path"]) / name) for name in ["evaluation.json", "decisions.json", "seal.json"]]
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
    prefix = None
    if mode in case.get("prefix_captures", {}):
        original = case["prefix_captures"][mode]
        if sha(Path(original["path"]) / "evaluation.json") != original["sha256"]:
            raise RuntimeError("retained prefix capture drift")
        prefix = check_prefix(output, original["path"])
        write_new(output / "prefix-proof.json", prefix)
    proof = {"physical_replay": True, "controller_reconstruction": True, "planner_reconstruction": True,
        "summary_equality": True, "capture_sha256": sha(output / "evaluation.json"),
        "decisions_sha256": sha(output / "decisions.json"), "verified_target_landing": saved["verified_target_landing"],
        "mode": mode, "planner_kind": "native_witness_endpoint_rule" if "recipe" in case else "retained_input_goal_rule",
        "prefix_preserved": prefix["passed"] if prefix else None}
    write_new(output / "proof.json", proof)
    return {"capture": str(output), "case": case["id"], "mode": mode, **proof}
