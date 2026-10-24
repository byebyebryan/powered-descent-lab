"""Frozen timing references, measurements sidecars and complete native proof."""
import json
import time
from pathlib import Path

from composition_study import Budget, check_prefix, encoded, goal_program, native, read, sha, write_new
from arrival import compact as arrival_compact
from timing_study import FIRST, FUEL, assert_fuel_selection

EXTENDED = "minimum_fuel_extended_grid_v1"
MODES = [FIRST, FUEL, EXTENDED]


def compact(info):
    value = arrival_compact(info)
    search = info["landing_search"]
    value.update(coast_ticks=search["chosen_coast_ticks"], duration_ticks=search["chosen_duration_ticks"])
    return value


def assert_selection(info, mode):
    if mode == FIRST:
        return
    if mode == FUEL:
        assert_fuel_selection(info)
        return
    # Reuse the exact same ranking/execution predicate with the named superset.
    assert info["landing_search"]["selector"] == "poly_native_minimum_fuel_extended_coast_duration_grid_v1"
    normalized = {**info, "landing_search": {**info["landing_search"],
        "selector": "poly_native_minimum_fuel_coast_duration_grid_v1"}}
    assert_fuel_selection(normalized)
    assert len(info["landing_search"]["candidates"]) == 132


def invoke(budget, case, mode, captured=False, inputs=None, forced=None,
           category="originals", measured=False):
    if mode not in MODES:
        raise ValueError("unknown reference")
    budget.check()
    program = goal_program(case, budget) if forced is None else forced
    descriptor = {"schema_version": 1, "implementation": "timing-reference-v1",
        "checkpoint_sha256": None, "configuration": {"timing_selection": mode,
        "actuation_profile": "native_pair_v2", "program": program,
        "native_library_sha256": sha(native.__file__), "case_id": case["id"]}}
    episode = native.ArrivalEpisode(encoded(read(case["scenario_path"])), encoded(program), captured,
        encoded(descriptor), inputs or [], actuation_profile="native_pair_v2",
        timing_selection=None if mode == FIRST else mode, measure_timing=measured)
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


def summary(budget, case, mode, measured=False):
    episode, _, cost = invoke(budget, case, mode, measured=measured)
    result = json.loads(episode.finish_json())
    budget.consume(result["info"])
    assert_selection(result["info"], mode)
    return {**result, "measured_cost": cost,
        "measurements": json.loads(episode.measurements_json())}


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
        raise RuntimeError("Summary/Captured reference execution differs")
    for forced in [program, None]:
        rebuilt, _, _ = invoke(budget, case, mode, forced=forced, category="reconstructions")
        actual = json.loads(rebuilt.finish_json())["info"]
        budget.consume(actual)
        if actual != info:
            raise RuntimeError("controller/input-goal reconstruction differs")
    prefix = None
    if retained and info["arrival"] is not None:
        prefix = check_prefix(output, retained)
        write_new(output/"prefix-proof.json", prefix)
    assert_selection(info, mode)
    proof = {"physical_replay": True, "controller_reconstruction": True, "planner_reconstruction": True,
        "summary_equality": True, "capture_sha256": sha(output/"evaluation.json"),
        "decisions_sha256": sha(output/"decisions.json"), "verified_target_landing": saved["verified_target_landing"],
        "mode": mode, "planner_kind": "frozen_input_and_named_timing_reference_v1",
        "prefix_preserved": prefix["passed"] if prefix else None}
    write_new(output/"proof.json", proof)
    return {"capture": str(output), "case": case["id"], "mode": mode, "measured_cost": cost, **proof}
