"""Read-only independent schedule, predicate, timing, prefix and accounting audit."""
import argparse
import hashlib
import json
import math
from pathlib import Path

from audit_actuation import observed_arrival
from composition_study import check_prefix

FIRST, FUEL = "first_feasible_v1", "minimum_fuel_grid_v1"


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def audit(root):
    totals = {"captures": 0, "verified_landings": 0, "prefix_proofs": 0, "preserved_summary_executions": 0,
        "native_steps": 0, "model_steps": 0, "forecast_steps": 0, "neutral_replay_steps": 0,
        "reconstructions": 0, "originals": 0, "source_bindings": 0}
    contacts = []
    for phase in ["pilot", "comparison"]:
        allocation, result = read(root/(phase+"-allocation.json")), read(root/phase/"result.json")
        schedule = {(read(i["path"])["id"], mode) for i in allocation["cases"] for mode in i["modes"]}
        assert len(schedule) == len(result["rows"])
        assert schedule == {(r["case"], r["mode"]) for r in result["rows"]}
        runtime = read(root/phase/"runtime.json")
        assert sha(root/(phase+"-allocation.json")) == runtime["allocation_sha256"]
        assert sha(runtime["native_path"]) == allocation["native_sha256"] == runtime["native_sha256"]
        for item in runtime["compiled_sources"]["source_files"]:
            assert sha(item["path"]) == item["sha256"]
            totals["source_bindings"] += 1
        for item in allocation["cases"]:
            assert sha(item["path"]) == item["sha256"]
            case = read(item["path"])
            assert sha(case["scenario_path"]) == case["scenario_sha256"]
            if "recipe" in case:
                construction = read(case["construction_path"])
                assert construction["admitted"] and construction["request"] == case["program"]["arrival"]
        counts = {k: allocation.get("carried_counts", {}).get(k, 0) for k in ["native_steps", "model_steps", "forecast_steps", "neutral_replay_steps"]}
        goal_steps = 0
        lookup = {(r["case"], r["mode"]): r for r in result["rows"]}
        for row in result["rows"]:
            case = read(row["case_path"])
            info = read(root/phase/"summaries"/(row["case"]+"-"+row["mode"]+".json"))["info"]
            for key, field in [("native_steps", "physics_steps"), ("model_steps", "model_steps"), ("forecast_steps", "forecast_steps")]:
                counts[key] += info[field]
            goal_steps += case.get("recipe", {}).get("physics_ticks", 0)
            previous = read(Path(allocation["retained_root"])/"confirmation/summaries"/(case["id"]+"-native_pair_v2.json"))["info"]
            if row["mode"] == FIRST:
                assert info == previous
                totals["preserved_summary_executions"] += 1
            else:
                assert info["arrival"] == previous["arrival"]
                search = info["landing_search"]
                assert search["selector"] == "poly_native_minimum_fuel_coast_duration_grid_v1"
                assert {(c["coast_ticks"], c["duration_ticks"]) for c in search["candidates"]} == {
                    (coast, duration) for coast in [0, 60, 120, 240, 360, 480, 720]
                    for duration in [720, 960, 1200, 1440, 1920, 2400, 2880, 3600, 4320]}
                assert len(search["candidates"]) == 63
                passing = [c for c in search["candidates"] if c.get("result") == "predicted_target_landing"]
                if passing:
                    selected = max(passing, key=lambda c: c["remaining_fuel_kg"])
                    assert (selected["coast_ticks"], selected["duration_ticks"]) == (search["chosen_coast_ticks"], search["chosen_duration_ticks"])
                    assert selected["remaining_fuel_kg"] == info["observation"]["fuel_kg"]
                    assert selected["contact_tick"] == info["physics_steps"]
        roles = {tuple(k) for k in allocation["captures"]}
        if allocation.get("capture_best_and_failures"):
            states = {}
            for row in result["rows"]:
                if row["mode"] == FUEL:
                    states.setdefault(row["state_id"], []).append(row)
                    if not row["result"]["success"] or row["result"]["arrival_status"] not in ["direct", "reached"]:
                        roles.add((row["case"], FUEL))
            for candidates in states.values():
                good = [r for r in candidates if r["result"]["success"] and r["result"]["arrival_status"] in ["direct", "reached"]]
                if good:
                    best = min(good, key=lambda r: (r["result"]["fuel_used_kg"], ["direct", "support_short", "brake_long"].index(r["choice"])))
                    roles.add((best["case"], FUEL))
        assert roles == {(c["case"], c["mode"]) for c in result["captures"]}
        assert len(roles) == len(result["captures"]) <= allocation["max_captures"]
        for capture in result["captures"]:
            folder = Path(capture["capture"])
            proof, seal = read(folder/"proof.json"), read(folder/"seal.json")
            assert sha(folder/"evaluation.json") == proof["capture_sha256"] == seal["evaluation_sha256"] == capture["capture_sha256"]
            assert sha(folder/"decisions.json") == proof["decisions_sha256"] == capture["decisions_sha256"]
            assert all(proof[k] for k in ["physical_replay", "controller_reconstruction", "planner_reconstruction", "summary_equality"])
            evaluation, info = read(folder/"evaluation.json"), read(folder/"decisions.json")
            row = lookup[(capture["case"], capture["mode"])];case = read(row["case_path"])
            assert info == read(root/phase/"summaries"/(row["case"]+"-"+row["mode"]+".json"))["info"]
            assert evaluation["integrity_passed"] and evaluation["replay"]["complete_episode_verified"]
            assert evaluation["verified_target_landing"] == proof["verified_target_landing"] == info["native_success"]
            assert evaluation["execution"]["descriptor"]["configuration"]["timing_selection"] == row["mode"]
            old = Path(allocation["retained_root"])/"confirmation/captures"/(case["id"]+"-native_pair_v2")
            previous_evaluation = read(old/"evaluation.json")
            assert sha(old/"evaluation.json") == read(old/"seal.json")["evaluation_sha256"]
            # Persisted ScenarioSpec includes native serde defaults that were
            # absent in the input JSON. Compare its complete canonical form to
            # the sealed original rather than weakening input-byte identity.
            assert evaluation["scenario"] == previous_evaluation["scenario"]
            assert evaluation["physical_sha256"] == previous_evaluation["physical_sha256"]
            request = info["arrival"]["request"] if info["arrival"] else None
            status, tick = observed_arrival(evaluation, request, False)
            if status:
                assert status == info["arrival"]["status"] and tick == info["arrival"]["resolution"]["physics_step"]
                assert info["landing_search"]["selection_observation"] == info["arrival"]["resolution"]
                assert check_prefix(folder, old) == read(folder/"prefix-proof.json")
                totals["prefix_proofs"] += 1
            contact = evaluation["execution"]["episode"]["incoming_contact"]
            if contact:
                incoming = contact["state"]
                contacts.append({"phase": phase, "case": case["id"], "speed_mps": math.hypot(*incoming["velocity_mps"].values()),
                    "attitude_rad": incoming["attitude_rad"], "angular_rate_radps": incoming["angular_rate_radps"],
                    "position_m": incoming["position_m"], "classification": contact["classification"]})
            for key, field in [("native_steps", "physics_steps"), ("model_steps", "model_steps"), ("forecast_steps", "forecast_steps")]:
                counts[key] += 3*info[field]
            counts["neutral_replay_steps"] += 3*info["physics_steps"]
            goal_steps += 2*case.get("recipe", {}).get("physics_ticks", 0)
            totals["captures"] += 1
            totals["verified_landings"] += int(evaluation["verified_target_landing"])
        counts["native_steps"] += goal_steps
        assert all(result["resources"]["counts"][k] == value for k, value in counts.items()), (phase, counts, result["resources"])
        assert result["resources"]["total_steps"] == sum(counts.values()) <= allocation["max_steps"]
        assert result["resources"]["counts"]["goal_generation_steps"] == goal_steps
        assert result["resources"]["counts"]["originals"] == len(result["rows"])+len(result["captures"])
        assert result["resources"]["counts"]["reconstructions"] == 2*len(result["captures"])
        for key, value in counts.items():totals[key] += value
        totals["originals"] += result["resources"]["counts"]["originals"]
        totals["reconstructions"] += result["resources"]["counts"]["reconstructions"]
    allocation = read(root/"proof-completion-allocation.json")
    result = read(root/"proof-completion/result.json")
    runtime = read(root/"proof-completion/runtime.json")
    assert runtime["allocation_sha256"] == sha(root/"proof-completion-allocation.json")
    assert sha(runtime["native_path"]) == runtime["native_sha256"] == allocation["native_sha256"]
    assert sha(runtime["runner_path"]) == runtime["runner_sha256"] == allocation["runner_sha256"]
    for item in runtime["compiled_sources"]["source_files"]:
        assert sha(item["path"]) == item["sha256"]
        totals["source_bindings"] += 1
    assert {r["case_path"] for r in allocation["roles"]} == {c["case_path"] for c in result["captures"]}
    counts = {k: 0 for k in ["native_steps", "model_steps", "forecast_steps", "neutral_replay_steps"]}
    goal_steps = 0
    for role in allocation["roles"]:
        assert sha(role["case_path"]) == role["case_sha256"]
        assert sha(role["summary_path"]) == role["summary_sha256"]
    for capture in result["captures"]:
        folder = Path(capture["capture"])
        proof, seal = read(folder/"proof.json"), read(folder/"seal.json")
        assert sha(folder/"evaluation.json") == proof["capture_sha256"] == seal["evaluation_sha256"] == capture["capture_sha256"]
        assert sha(folder/"decisions.json") == proof["decisions_sha256"] == capture["decisions_sha256"]
        assert all(proof[k] for k in ["physical_replay", "controller_reconstruction", "planner_reconstruction", "summary_equality"])
        evaluation, info = read(folder/"evaluation.json"), read(folder/"decisions.json")
        case = read(capture["case_path"])
        assert sha(case["scenario_path"]) == case["scenario_sha256"]
        assert info == read(capture["summary_path"])["info"]
        assert evaluation["integrity_passed"] and evaluation["replay"]["complete_episode_verified"]
        assert evaluation["verified_target_landing"] == proof["verified_target_landing"] == info["native_success"]
        assert evaluation["execution"]["descriptor"]["configuration"]["timing_selection"] == FUEL
        old = Path(allocation["retained_root"])/"confirmation/captures"/(case["id"]+"-native_pair_v2")
        previous_evaluation = read(old/"evaluation.json")
        assert sha(old/"evaluation.json") == read(old/"seal.json")["evaluation_sha256"]
        assert evaluation["scenario"] == previous_evaluation["scenario"]
        assert evaluation["physical_sha256"] == previous_evaluation["physical_sha256"]
        request = info["arrival"]["request"] if info["arrival"] else None
        status, tick = observed_arrival(evaluation, request, False)
        if status:
            assert status == info["arrival"]["status"] and tick == info["arrival"]["resolution"]["physics_step"]
            assert check_prefix(folder, old) == read(folder/"prefix-proof.json")
            totals["prefix_proofs"] += 1
        contact = evaluation["execution"]["episode"]["incoming_contact"]
        if contact:
            incoming = contact["state"]
            contacts.append({"phase": "proof-completion", "case": case["id"], "speed_mps": math.hypot(*incoming["velocity_mps"].values()),
                "attitude_rad": incoming["attitude_rad"], "angular_rate_radps": incoming["angular_rate_radps"],
                "position_m": incoming["position_m"], "classification": contact["classification"]})
        for key, field in [("native_steps", "physics_steps"), ("model_steps", "model_steps"), ("forecast_steps", "forecast_steps")]:
            counts[key] += 3*info[field]
        counts["neutral_replay_steps"] += 3*info["physics_steps"]
        goal_steps += 2*case.get("recipe", {}).get("physics_ticks", 0)
        totals["captures"] += 1
        totals["verified_landings"] += int(evaluation["verified_target_landing"])
    counts["native_steps"] += goal_steps
    assert all(result["resources"]["counts"][k] == v for k, v in counts.items())
    assert result["resources"]["total_steps"] == sum(counts.values()) <= allocation["max_steps"]
    assert result["resources"]["counts"]["goal_generation_steps"] == goal_steps
    assert result["resources"]["counts"]["originals"] == len(result["captures"])
    assert result["resources"]["counts"]["reconstructions"] == 2*len(result["captures"])
    for k, v in counts.items():totals[k] += v
    totals["originals"] += result["resources"]["counts"]["originals"]
    totals["reconstructions"] += result["resources"]["counts"]["reconstructions"]
    comparison_captures = read(root/"comparison/result.json")["captures"]+result["captures"]
    assert len(comparison_captures) == 180 == len({c["case"] for c in comparison_captures})
    totals["complete_new_timing_comparison_captures"] = len(comparison_captures)
    totals["steps"] = sum(totals[k] for k in ["native_steps", "model_steps", "forecast_steps", "neutral_replay_steps"])
    assert totals["steps"] <= 1_200_000_000 and totals["captures"] <= 256
    totals.update(passed=True, scope="Read-only schedule/source/predicate/prefix/selection/proof/accounting audit; no native transitions", incoming_contacts=contacts)
    return totals


if __name__ == "__main__":
    p = argparse.ArgumentParser();p.add_argument("root", type=Path);p.add_argument("--receipt", type=Path, required=True);args = p.parse_args()
    result = audit(args.root.resolve())
    with args.receipt.open("x") as f:json.dump(result, f, indent=2, allow_nan=False);f.write("\n")
    print(json.dumps({k: v for k, v in result.items() if k != "incoming_contacts"}, indent=2))
