"""Read-only independent schedule, source, predicate and accounting audit."""
import argparse
import math
from pathlib import Path

from arrival import read, sha, write_new, encoded
from audit_actuation import observed_arrival
from composition_study import check_prefix

FIRST, FUEL, EXTENDED = "first_feasible_v1", "minimum_fuel_grid_v1", "minimum_fuel_extended_grid_v1"
FIELDS = {"native_steps":"physics_steps", "model_steps":"model_steps", "forecast_steps":"forecast_steps"}


def subset(raw, canonical):
    """Serde may add defaults; every supplied input value must survive exactly."""
    if isinstance(raw, dict):
        assert isinstance(canonical, dict)
        for key, value in raw.items():
            assert key in canonical, key
            subset(value, canonical[key])
    elif isinstance(raw, list):
        assert len(raw) == len(canonical)
        for left, right in zip(raw, canonical): subset(left, right)
    else:
        assert raw == canonical, (raw, canonical)


def selection(info, mode):
    if mode == FIRST: return
    search = info["landing_search"]
    coasts = [0,60,120,240,360,480,720] if mode == FUEL else [0,60,120,240,360,480,600,720,840,960,1080,1200]
    burns = [720,960,1200,1440,1920,2400,2880,3600,4320] if mode == FUEL else [720,960,1200,1440,1680,1920,2160,2400,2880,3600,4320]
    pairs = [(c["coast_ticks"], c["duration_ticks"]) for c in search["candidates"]]
    assert pairs == [(c,b) for c in coasts for b in burns]
    passing = [c for c in search["candidates"] if c.get("result") == "predicted_target_landing"]
    if not passing:
        assert search["no_plan"] and search["chosen_coast_ticks"] is None
        return
    best = max(passing, key=lambda c:c["remaining_fuel_kg"])
    assert (best["coast_ticks"],best["duration_ticks"]) == (search["chosen_coast_ticks"],search["chosen_duration_ticks"])
    assert best["remaining_fuel_kg"] == info["observation"]["fuel_kg"]
    assert best["contact_tick"] == info["physics_steps"] and info["native_success"]


def audit_phase(root, phase, result_name="result.json"):
    a = read(root/(phase+"-allocation.json")); result = read(root/phase/result_name)
    runtime = read(root/phase/"runtime.json")
    assert runtime["allocation_sha256"] == sha(root/(phase+"-allocation.json"))
    assert sha(runtime["native_path"]) == runtime["native_sha256"] == a["native_sha256"]
    for identity in runtime["compiled_sources"]["source_files"]:
        assert sha(identity["path"]) == identity["sha256"]
    scheduled = {(read(i["path"])["id"],m) for i in a["cases"] for m in i["modes"]}
    actual = {(r["case"],r["mode"]) for r in result["rows"]}
    assert len(actual) == len(result["rows"])
    assert actual == scheduled if result_name == "result.json" else actual <= scheduled
    # An item can repeat a case with a different reference in development.
    lookup = {(read(i["path"])["id"],m):i for i in a["cases"] for m in i["modes"]}
    counts = {k:0 for k in [*FIELDS,"neutral_replay_steps"]}; goal_steps = 0
    captures = {(c["case"],c["mode"]):c for c in result["captures"]}
    contacts, preserved, prefixes, landed = [], 0, 0, 0
    canonical = {}
    for row in result["rows"]:
        item = lookup[(row["case"],row["mode"])]
        assert sha(item["path"]) == item["sha256"]
        case = read(item["path"])
        assert sha(case["scenario_path"]) == case["scenario_sha256"]
        info = read(root/phase/"summaries"/(row["case"]+"-"+row["mode"]+".json"))["info"]
        selection(info,row["mode"])
        if item.get("expected_root"):
            assert info == read(Path(item["expected_root"])/(row["case"]+"-"+row["mode"]+".json"))["info"]
            preserved += 1
        for dest,field in FIELDS.items():counts[dest] += info[field]
        goal_steps += case.get("recipe",{}).get("physics_ticks",0)
        key = (row["case"],row["mode"])
        expected_capture = row["mode"] in item.get("capture_modes",[])
        if result_name == "result.json": assert (key in captures) == expected_capture
        if key not in captures: continue
        folder = Path(captures[key]["capture"])
        proof,seal = read(folder/"proof.json"),read(folder/"seal.json")
        assert sha(folder/"evaluation.json") == proof["capture_sha256"] == seal["evaluation_sha256"]
        assert sha(folder/"decisions.json") == proof["decisions_sha256"]
        assert read(folder/"decisions.json") == info
        assert all(proof[k] for k in ["physical_replay","controller_reconstruction","planner_reconstruction","summary_equality"])
        e = read(folder/"evaluation.json")
        assert e["integrity_passed"] and e["agent_descriptor_unchanged"]
        assert e["replay"]["physical_prefix_verified"] and e["replay"]["complete_episode_verified"]
        assert e["verified_target_landing"] == proof["verified_target_landing"] == info["native_success"]
        assert e["execution"]["descriptor"]["configuration"]["timing_selection"] == row["mode"]
        subset(read(case["scenario_path"]),e["scenario"])
        identity = (e["scenario"],e["physical_sha256"])
        if row["case"] in canonical: assert canonical[row["case"]] == identity
        canonical[row["case"]] = identity
        for identity in e["evidence_after"]["source_files"]+e["evidence_after"]["input_files"]:
            assert sha(identity["path"]) == identity["sha256"]
        status,tick = observed_arrival(e,info["arrival"]["request"] if info["arrival"] else None,False)
        if status:
            assert status == info["arrival"]["status"] and tick == info["arrival"]["resolution"]["physics_step"]
            assert info["landing_search"]["selection_observation"] == info["arrival"]["resolution"]
        if item.get("retained_capture"):
            assert check_prefix(folder,item["retained_capture"]) == read(folder/"prefix-proof.json")
            prefixes += 1
        contact = e["execution"]["episode"]["incoming_contact"]
        if contact:
            state = contact["state"]
            contacts.append({"phase":phase,"case":row["case"],"mode":row["mode"],
                "speed_mps":math.hypot(*state["velocity_mps"].values()),"lateral_mps":state["velocity_mps"]["x"],
                "vertical_mps":state["velocity_mps"]["y"],"attitude_rad":state["attitude_rad"],
                "angular_rate_radps":state["angular_rate_radps"],"pad_offset_m":state["position_m"]["x"]-next(
                    p["center_x_m"] for p in e["scenario"]["world"]["landing_pads"]
                    if p["id"] == e["scenario"]["mission"]["goal"]["target_pad_id"]),
                "classification":contact["classification"]})
        for dest,field in FIELDS.items():counts[dest] += 3*info[field]
        counts["neutral_replay_steps"] += 3*info["physics_steps"]
        goal_steps += 2*case.get("recipe",{}).get("physics_ticks",0)
        landed += int(e["verified_target_landing"])
    counts["native_steps"] += goal_steps
    extra = int(result_name == "fault.json")
    if extra:
        aborted = read(root/"development-aborted-attempt.json")
        assert aborted["fault_sha256"] == sha(root/phase/result_name)
        assert aborted["counts"] == {"native_steps":2,"model_steps":0,"forecast_steps":90894,"neutral_replay_steps":0}
        completed = read(root/"development-completion/summaries/composition-confirmation-058-direct-minimum_fuel_grid_v1.json")["info"]
        assert completed["forecast_steps"] == aborted["counts"]["forecast_steps"]
        for k,v in aborted["counts"].items(): counts[k] += v
    recorded = result.get("resources",{}).get("counts",result.get("counts"))
    assert all(recorded[k] == v for k,v in counts.items()), (phase,counts,recorded)
    assert recorded.get("goal_generation_steps",0) == goal_steps
    assert recorded["originals"] == len(result["rows"])+len(captures)+extra
    if extra:
        assert result["error"] == "ValueError('insufficient per-update work reservation')"
        assert 0 <= a["max_steps"]-sum(counts.values()) < 2_000_002
    assert recorded["reconstructions"] == 2*len(captures)
    assert sum(counts.values()) <= a["max_steps"]
    return {"phase":phase,"counts":recorded,"total_steps":sum(counts.values()),"rows":len(result["rows"]),
        "captures":len(captures),"verified_landings":landed,"preserved":preserved,"prefixes":prefixes,"contacts":contacts}


if __name__ == "__main__":
    p = argparse.ArgumentParser();p.add_argument("root",type=Path);p.add_argument("phase");p.add_argument("receipt",type=Path)
    args = p.parse_args();write_new(args.receipt,audit_phase(args.root.resolve(),args.phase))
