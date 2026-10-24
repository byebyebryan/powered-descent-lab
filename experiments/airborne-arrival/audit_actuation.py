"""Read-only source, goal-predicate, proof and work-accounting audit."""
import argparse
import json
import math
from pathlib import Path
from actuation_study import compact, native, read, sha, write_new


def observed_arrival(e, request, witness):
    if request is None:
        return None, None
    w = request["waypoint"]
    anchor = e["scenario"]["initial_state"]["position_m"]
    dx, dy = w["position_m"]["x"]-anchor["x"], w["position_m"]["y"]-anchor["y"]
    length = math.sqrt(dx*dx+dy*dy)
    ux, uy = dx/length, dy/length
    tangent = w["handoff_tangent_unit"]
    window = False
    final = e["execution"]["episode"]["final_state"]["physics_step"]
    observations = {s["observation"]["physics_step"]: s["observation"] for s in e["execution"]["episode"]["run"]["samples"]}
    for tick, o in sorted(observations.items()):
        if (tick >= final and not witness) or (tick % 2 and tick != final):
            continue
        x, y = o["position_m"]["x"]-w["position_m"]["x"], o["position_m"]["y"]-w["position_m"]["y"]
        vx, vy = o["velocity_mps"]["x"], o["velocity_mps"]["y"]
        distance, speed = math.sqrt(x*x+y*y), math.sqrt(vx*vx+vy*vy)
        plane = x*ux+y*uy
        window |= distance <= w["capture_radius_m"]
        dot = (vx*tangent["x"]+vy*tangent["y"])/speed if speed > 1e-9 else -1.0
        heading = math.acos(max(-1.0,min(1.0,dot)))
        spatial = distance <= w["capture_radius_m"] or (abs(x*uy-y*ux) <= w["max_cross_track_m"] and plane >= -w["capture_radius_m"])
        valid = (heading <= w["max_outbound_heading_error_rad"]
            and vx*tangent["x"]+vy*tangent["y"] >= w["min_outbound_progress_mps"]
            and (w.get("max_outbound_cross_speed_mps") is None or abs(vx*tangent["y"]-vy*tangent["x"]) <= w["max_outbound_cross_speed_mps"])
            and w["min_speed_mps"] <= speed <= w["max_speed_mps"]
            and (w.get("min_vertical_speed_mps") is None or vy >= w["min_vertical_speed_mps"])
            and (w.get("max_vertical_speed_mps") is None or vy <= w["max_vertical_speed_mps"]))
        if (window or plane >= 0) and spatial and valid:
            return "reached", tick
        if plane >= 0:
            return "plane_miss", tick
        if tick >= request["deadline_physics_step"]:
            return "expired", tick
    return "native_end", final


def audit(root):
    totals = {"steps": 0, "captures": 0, "source_bindings": 0, "legacy_controls": 0, "direct_controls": 0}
    for phase in ["pilot", "witnesses", "main"]:
        a, result = read(root/(phase+"-allocation.json")), read(root/phase/"result.json")
        assert {(read(i["path"])["id"],m) for i in a["cases"] for m in i["modes"]} == {(r["case"],r["mode"]) for r in result["rows"]}
        runtime=read(root/phase/"runtime.json")
        assert runtime["native_sha256"] == sha(runtime["native_path"]) == a["native_sha256"]
        assert runtime["allocation_sha256"] == sha(root/(phase+"-allocation.json"))
        for source in runtime["compiled_sources"]["source_files"]:
            assert sha(source["path"]) == source["sha256"]
            totals["source_bindings"] += 1
        for item in a["cases"]:
            assert sha(item["path"]) == item["sha256"]
            case=read(item["path"])
            assert sha(case["scenario_path"]) == case["scenario_sha256"]
            if "legacy_summary" in case:
                assert sha(case["legacy_summary"]) == case["legacy_summary_sha256"]
        computed={k:a.get("carried_counts",{}).get(k,0) for k in ["native_steps","model_steps","forecast_steps","neutral_replay_steps"]}
        goal_steps=0
        for row in result["rows"]:
            case=read(row["case_path"])
            for key,field in [("native_steps","steps"),("model_steps","model_steps"),("forecast_steps","forecast_steps")]:computed[key]+=row["result"][field]
            if "recipe" in case:goal_steps+=case["recipe"]["physics_ticks"]
            if row["mode"]=="legacy_v1" and "legacy" in row:
                assert row["legacy"]==row["result"];totals["legacy_controls"]+=1
            if row["result"]["arrival_status"]=="direct":
                assert row["legacy"]==row["result"];totals["direct_controls"]+=1
        lookup={(r["case"],r["mode"]):r for r in result["rows"]}
        for capture in result["captures"]:
            folder=Path(capture["capture"]);proof,seal=read(folder/"proof.json"),read(folder/"seal.json")
            assert sha(folder/"evaluation.json")==seal["evaluation_sha256"]==proof["capture_sha256"]
            assert sha(folder/"decisions.json")==proof["decisions_sha256"]
            assert all(proof[k] for k in ["physical_replay","controller_reconstruction","planner_reconstruction","summary_equality"])
            e,info=read(folder/"evaluation.json"),read(folder/"decisions.json")
            assert e["integrity_passed"] and e["replay"]["complete_episode_verified"]
            assert e["verified_target_landing"]==info["native_success"]==proof["verified_target_landing"]
            row=lookup[(capture["case"],capture["mode"])];assert compact(info)==row["result"]
            case=read(row["case_path"])
            mode=capture["mode"]
            assert e["execution"]["descriptor"]["configuration"]["mode"]==mode
            request=info["arrival"]["request"] if info["arrival"] else None
            status,tick=observed_arrival(e,request,mode=="witness")
            if status is not None:
                assert status==info["arrival"]["status"],(capture["case"],status,info["arrival"]["status"])
                assert tick==info["arrival"]["resolution"]["physics_step"],(capture["case"],tick,info["arrival"]["resolution"]["physics_step"])
            for key,field in [("native_steps","physics_steps"),("model_steps","model_steps"),("forecast_steps","forecast_steps")]:computed[key]+=3*info[field]
            computed["neutral_replay_steps"]+=3*info["physics_steps"]
            if "recipe" in case:goal_steps+=2*case["recipe"]["physics_ticks"]
            totals["captures"]+=1
        computed["native_steps"]+=goal_steps
        assert all(result["resources"]["counts"][k]==v for k,v in computed.items()),(phase,computed,result["resources"])
        assert result["resources"]["total_steps"]==sum(computed.values())<=a["max_steps"]
        assert result["resources"]["optimizer_steps"]==0
        totals["steps"]+=sum(computed.values())
    assert totals["steps"]<=100_000_000 and totals["captures"]==60
    totals["passed"]=True
    totals["scope"]="read-only audit including independent fly-through assessment; no native transitions"
    return totals


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=Path);p.add_argument("--receipt",type=Path);args=p.parse_args()
    result=audit(args.root.resolve())
    if args.receipt:write_new(args.receipt,result)
    print(json.dumps(result,sort_keys=True))
