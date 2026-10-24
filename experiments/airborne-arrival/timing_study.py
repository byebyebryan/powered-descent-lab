"""Opt-in timing comparison with unchanged local guidance and complete proof."""
import json
import time
from pathlib import Path

from composition_study import Budget, check_prefix, encoded, goal_program, native, read, sha, write_new
from arrival import compact as arrival_compact

FIRST = "first_feasible_v1"
FUEL = "minimum_fuel_grid_v1"
MODES = [FIRST, FUEL]


def compact(info):
    value = arrival_compact(info)
    search, observation = info["landing_search"], info["observation"]
    value.update(coast_ticks=search["chosen_coast_ticks"], duration_ticks=search["chosen_duration_ticks"],
        contact_speed_mps=(observation["velocity_mps"]["x"]**2 + observation["velocity_mps"]["y"]**2)**.5,
        contact_attitude_rad=observation["attitude_rad"], contact_angular_rate_radps=observation["angular_rate_radps"])
    return value


def eligible(row):
    r = row["result"]
    return r["success"] and r["arrival_status"] in ["direct", "reached"]


def choice(case):
    return case.get("goal_choice", case.get("choice", "arrival"))


def invoke(budget, case, mode, captured=False, inputs=None, forced=None, category="originals"):
    if mode not in MODES:
        raise ValueError("unknown timing profile")
    budget.check()
    program = goal_program(case, budget) if forced is None else forced
    descriptor = {"schema_version": 1, "implementation": "arrival-timing-bank-v1",
        "checkpoint_sha256": None, "configuration": {"timing_selection": mode,
        "actuation_profile": "native_pair_v2", "program": program,
        "native_library_sha256": sha(native.__file__), "case_id": case["id"]}}
    episode = native.ArrivalEpisode(encoded(read(case["scenario_path"])), encoded(program), captured,
        encoded(descriptor), inputs or [], actuation_profile="native_pair_v2",
        timing_selection=mode if mode == FUEL else None)
    budget.counts[category] += 1
    started, cpu = time.perf_counter(), time.process_time()
    try:
        episode.execute(budget.remaining())
    except BaseException:
        budget.consume(json.loads(episode.status_json()))
        raise
    cost = {"episode_wall_seconds": time.perf_counter()-started,
        "episode_process_cpu_seconds": time.process_time()-cpu}
    return episode, program, cost


def summary(budget, case, mode):
    episode, _, cost = invoke(budget, case, mode)
    result = json.loads(episode.finish_json())
    budget.consume(result["info"])
    return {**result, "measured_cost": cost}


def assert_fuel_selection(info):
    search = info["landing_search"]
    assert search["selector"] == "poly_native_minimum_fuel_coast_duration_grid_v1"
    passing = [c for c in search["candidates"] if c.get("result") == "predicted_target_landing"]
    if not passing:
        assert search["no_plan"] and search["chosen_coast_ticks"] is None
        return
    best = max(passing, key=lambda c: c["remaining_fuel_kg"])
    assert not search["no_plan"]
    assert (best["coast_ticks"], best["duration_ticks"]) == (search["chosen_coast_ticks"], search["chosen_duration_ticks"])
    # A forecast is accepted as a selected candidate only; native Episode is the
    # authority for the mission result. Their final states must agree exactly.
    assert info["native_success"]
    assert info["observation"]["fuel_kg"] == best["remaining_fuel_kg"]
    assert info["physics_steps"] == best["contact_tick"]


def capture(budget, case, path, mode, output, expected, allocation, retained=None):
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    inputs = [str(path), str(allocation), case["scenario_path"], str(Path(native.__file__))]
    if retained:
        inputs += [str(Path(retained)/n) for n in ["evaluation.json", "decisions.json", "seal.json"]]
    episode, program, cost = invoke(budget, case, mode, True, inputs)
    status = json.loads(episode.status_json())
    budget.consume(status)
    if budget.remaining() < 3*status["physics_steps"] + 2_000_002:
        raise RuntimeError("complete proof reservation exhausted")
    budget.counts["neutral_replay_steps"] += 3*status["physics_steps"]
    saved = json.loads(episode.save_capture(str(output)))
    info = saved["info"]
    if info != expected:
        raise RuntimeError("Summary/Captured timing execution differs")
    for forced in [program, None]:
        rebuilt, _, _ = invoke(budget, case, mode, forced=forced, category="reconstructions")
        actual = json.loads(rebuilt.finish_json())["info"]
        budget.consume(actual)
        if actual != info:
            raise RuntimeError("controller/goal reconstruction differs")
    prefix = None
    if retained and info["arrival"] is not None:
        prefix = check_prefix(output, retained)
        write_new(output/"prefix-proof.json", prefix)
    if mode == FUEL:
        assert_fuel_selection(info)
    proof = {"physical_replay": True, "controller_reconstruction": True, "planner_reconstruction": True,
        "summary_equality": True, "capture_sha256": sha(output/"evaluation.json"),
        "decisions_sha256": sha(output/"decisions.json"), "verified_target_landing": saved["verified_target_landing"],
        "mode": mode, "planner_kind": "native_witness_goal_and_named_timing_bank_v1",
        "prefix_preserved": prefix["passed"] if prefix else None}
    write_new(output/"proof.json", proof)
    return {"capture": str(output), "case": case["id"], "mode": mode, "measured_cost": cost, **proof}


def capture_roles(allocation, rows):
    keys = {tuple(k) for k in allocation["captures"]}
    if allocation.get("capture_best_and_failures"):
        states = {}
        for row in rows:
            if row["mode"] == FUEL:
                states.setdefault(row["state_id"], []).append(row)
                if not eligible(row):
                    keys.add((row["case"], FUEL))
        order = {"direct": 0, "support_short": 1, "brake_long": 2}
        for candidates in states.values():
            possible = [r for r in candidates if eligible(r)]
            if possible:
                best = min(possible, key=lambda r: (r["result"]["fuel_used_kg"], order[r["choice"]]))
                keys.add((best["case"], FUEL))
    if len(keys) > allocation["max_captures"]:
        raise RuntimeError("declared capture allocation exceeded")
    return sorted(keys)
