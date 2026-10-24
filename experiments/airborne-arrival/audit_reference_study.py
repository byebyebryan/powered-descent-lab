"""Read-only independent schedule, source, predicate and accounting audit."""
import argparse
import math
import random
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


def audit_inputs(root):
    frozen = read(root/"reference-freeze.json"); receipt = read(root/"input-audit.json")
    assert receipt["reference_freeze_sha256"] == sha(root/"reference-freeze.json")
    assert receipt["generated_unix"] > frozen["frozen_unix"]
    exposed=set()
    development=read(root/"development-allocation.json")
    oldroot=Path(development["cases"][0]["path"]).parents[2]
    for name in ["confirmation","development","witnesses"]:
        path=oldroot/(name+"-allocation.json")
        if path.exists():
            for item in read(path)["cases"]:exposed.add(encoded(read(read(item["path"])["scenario_path"])["initial_state"]))
    anchor=read(root/"source-epoch-a/fixtures/scenarios/flat_terminal_descent.json")
    expected_world=read(root/"source-epoch-a/fixtures/scenarios/flat_terminal_descent.json")["world"]
    expected_world["terrain"]["points_m"]=[{"x":-4000,"y":0},{"x":4000,"y":0}]
    expected_mission=anchor["mission"].copy();expected_mission.pop("transfer_route",None)
    seen = set(); family_names = ["vertical","lateral","braking","attitude_preparation"]
    rng = random.Random(2310241853)
    for cohort,n in [("primary",300),("boundary",60)]:
        result = read(root/(cohort+"-inputs/result.json"))
        assert len(result["cases"]) == n and result["admissions"] == result["replacements"] == result["native_steps"] == 0
        assert result["generator_sha256"] == frozen["input_generator_sha256"]
        for index,item in enumerate(result["cases"]):
            assert sha(item["path"]) == item["sha256"]
            case = read(item["path"]); s = read(case["scenario_path"]); state=s["initial_state"]
            assert sha(case["scenario_path"]) == case["scenario_sha256"]
            assert s["id"] == f"reference-{cohort}-{index:03}"
            key = encoded(state); assert key not in seen and key not in exposed; seen.add(key)
            assert s["vehicle"] == anchor["vehicle"] and s["world"] == expected_world and s["mission"] == expected_mission
            family,sink = (rng.randrange(4),rng.randrange(3)) if cohort=="primary" else (index//15,(index%15)//5)
            assert s["metadata"]["family"] == family_names[family] and int(s["metadata"]["sink_band"]) == sink
            assert 725 <= state["position_m"]["y"] <= 875
            vx = abs(state["velocity_mps"]["x"]); x = abs(state["position_m"]["x"])
            assert (vx == x == 0) if family==0 else 310 <= x <= 390
            if family: assert ([37,43] if family==2 else [22,28])[0] <= vx <= ([37,43] if family==2 else [22,28])[1]
            assert [13,28,43][sink] <= -state["velocity_mps"]["y"] <= [17,32,47][sink]
            direction=int(s["metadata"]["direction"])
            assert direction in [-1,1]
            center=direction*.25 if family==3 else 0
            assert center-.040000000000001 <= state["attitude_rad"] <= center+.040000000000001
            if family:
                assert state["position_m"]["x"]*direction < 0 and state["velocity_mps"]["x"]*direction > 0
            assert state["angular_rate_radps"] == 0 and s["sim"]["max_time_s"] == 90 and s["sim"]["sample_hz"] == 120
            if cohort=="boundary":
                pattern=[(0,0,0,0,0),(1,1,1,1,1),(0,1,0,1,0),(1,0,1,0,1),(0,1,1,1,1)][index%5]
                h,offset,lateral,down,angle=pattern;direction=1 if index%2==0 else -1
                assert state["position_m"]["y"] == [725.,875.][h]
                assert state["position_m"]["x"] == (0 if family==0 else -direction*[310.,390.][offset])
                assert state["velocity_mps"]["y"] == -([13.,28.,43.][sink]+4*down)
                assert state["velocity_mps"]["x"] == (0 if family==0 else direction*([37.,43.] if family==2 else [22.,28.])[lateral])
                assert state["attitude_rad"] == (direction*.25 if family==3 else 0)+[-.04,.04][angle]
    assert len(seen)==360
    return {"primary":300,"boundary":60,"unique_physical_initial_states":len(seen),"no_admission_or_replacements":True}


def audit_benchmark(root):
    a,result = read(root/"benchmark-allocation.json"),read(root/"benchmark/result.json")
    host = read(root/"benchmark/host.json")
    assert host["allocation_sha256"] == sha(root/"benchmark-allocation.json")
    assert host["native_sha256"] == a["native_sha256"]
    expected = {(i["id"],m,j) for i in a["cases"] for m in [FIRST,FUEL,EXTENDED] for j in range(10)}
    assert len(result["rows"]) == len(expected) == 360
    assert expected == {(r["case"],r["mode"],r["repeat"]) for r in result["rows"]}
    counts = {k:0 for k in FIELDS}; fingerprints={}
    for row in result["rows"]:
        assert row["role"] == ("first_invocation" if row["repeat"]==0 else "warmup" if row["repeat"]<3 else "measured")
        m=row["measurements"]
        assert len(m["selection_wall_ns"]) == len(m["planning_frame_wall_ns"]) == 1
        assert m["selection_wall_ns"][0] > 0 and m["planning_frame_wall_ns"][0] >= m["selection_wall_ns"][0]
        assert len(m["ordinary_frame_wall_ns"]) == math.ceil(row["result"]["steps"]/2)-1
        key=(row["case"],row["mode"])
        if key in fingerprints: assert fingerprints[key] == row["result"]["fingerprint"]
        fingerprints[key] = row["result"]["fingerprint"]
        counts["native_steps"] += row["result"]["steps"]
        for k in ["model_steps","forecast_steps"]:counts[k] += row["result"][k]
    assert all(result["resources"]["counts"][k] == v for k,v in counts.items())
    assert result["resources"]["total_steps"] == sum(counts.values()) <= a["max_steps"]
    return {"rows":360,"measured_per_profile":84,"total_steps":sum(counts.values()),"counts":result["resources"]["counts"]}


def audit(root):
    phases=[audit_phase(root,p,"fault.json" if p=="development" else "result.json") for p in
        ["pilot","development","development-completion","primary","boundary","regressions"]]
    inputs,benchmark = audit_inputs(root),audit_benchmark(root)
    total=sum(p["total_steps"] for p in phases)+benchmark["total_steps"]
    captures=sum(p["captures"] for p in phases)
    assert total<=1_000_000_000 and captures==1165
    assert sum(p["preserved"] for p in phases)==360 and sum(p["prefixes"] for p in phases)==21
    return {"passed":True,"total_steps":total,"captures":captures,"verified_landings":sum(p["verified_landings"] for p in phases),
        "reconstructions":sum(p["counts"]["reconstructions"] for p in phases),"preserved_executions":360,"prefixes":21,
        "phases":[{k:v for k,v in p.items() if k!="contacts"} for p in phases],"benchmark":benchmark,"inputs":inputs,
        "contacts":[c for p in phases for c in p["contacts"]],"native_transitions_in_audit":0}


if __name__ == "__main__":
    p = argparse.ArgumentParser();p.add_argument("root",type=Path);p.add_argument("receipt",type=Path)
    args = p.parse_args();write_new(args.receipt,audit(args.root.resolve()))
