"""Read-only input, prefix, native proof and work-accounting audit."""
import argparse
import json
from pathlib import Path
from composition_study import check_prefix, compact, read, sha, write_new
from audit_actuation import observed_arrival

PHASES = ["pilot", "witnesses", "diagnostics", "development", "confirmation"]


def audit(root):
    totals = {"native_steps":0,"model_steps":0,"forecast_steps":0,"neutral_replay_steps":0,
        "originals":0,"reconstructions":0,"captures":0,"prefix_proofs":0,"verified_landings":0,"source_bindings":0}
    phase_results={}
    native_hash=None
    for phase in PHASES:
        a,result=read(root/(phase+"-allocation.json")),read(root/phase/"result.json")
        assert {(read(i["path"])["id"],m) for i in a["cases"] for m in i["modes"]}=={(r["case"],r["mode"]) for r in result["rows"]}
        runtime=read(root/phase/"runtime.json")
        assert runtime["native_sha256"]==sha(runtime["native_path"])==a["native_sha256"]
        assert native_hash is None or native_hash==runtime["native_sha256"]
        native_hash=runtime["native_sha256"]
        assert runtime["allocation_sha256"]==sha(root/(phase+"-allocation.json"))
        for source in runtime["compiled_sources"]["source_files"]:
            assert sha(source["path"])==source["sha256"]
            totals["source_bindings"]+=1
        for item in a["cases"]:
            assert sha(item["path"])==item["sha256"]
            case=read(item["path"])
            assert sha(case["scenario_path"])==case["scenario_sha256"]
            if "retained_summary" in case:
                assert sha(case["retained_summary"])==case["retained_summary_sha256"]
        computed={k:a.get("carried_counts",{}).get(k,0) for k in ["native_steps","model_steps","forecast_steps","neutral_replay_steps"]}
        goal_steps=0
        for row in result["rows"]:
            case=read(row["case_path"])
            for key,field in [("native_steps","steps"),("model_steps","model_steps"),("forecast_steps","forecast_steps")]:computed[key]+=row["result"][field]
            info=read(root/phase/"summaries"/(row["case"]+"-"+row["mode"]+".json"))["info"]
            assert compact(info)==row["result"]
            if "retained_summary" in case:assert info==read(case["retained_summary"])["info"]
            if "recipe" in case:goal_steps+=case["recipe"]["physics_ticks"]
        lookup={(r["case"],r["mode"]):r for r in result["rows"]}
        assert {(c["case"],c["mode"]) for c in result["captures"]}=={tuple(c) for c in a["captures"]}
        assert len(result["captures"])<=a["max_captures"]
        for capture in result["captures"]:
            folder=Path(capture["capture"]);proof,seal=read(folder/"proof.json"),read(folder/"seal.json")
            assert sha(folder/"evaluation.json")==seal["evaluation_sha256"]==proof["capture_sha256"]==capture["capture_sha256"]
            assert sha(folder/"decisions.json")==proof["decisions_sha256"]
            assert all(proof[k] for k in ["physical_replay","controller_reconstruction","planner_reconstruction","summary_equality"])
            e,info=read(folder/"evaluation.json"),read(folder/"decisions.json")
            assert e["integrity_passed"] and e["replay"]["complete_episode_verified"]
            assert e["verified_target_landing"]==info["native_success"]==proof["verified_target_landing"]
            row=lookup[(capture["case"],capture["mode"])];case=read(row["case_path"])
            assert info==read(root/phase/"summaries"/(row["case"]+"-"+row["mode"]+".json"))["info"]
            assert e["execution"]["descriptor"]["configuration"]["mode"]==capture["mode"]
            request=info["arrival"]["request"] if info["arrival"] else None
            status,tick=observed_arrival(e,request,capture["mode"]=="witness")
            if status is not None:
                assert status==info["arrival"]["status"]
                assert tick==info["arrival"]["resolution"]["physics_step"]
                assert info["landing_search"]["selection_observation"]==info["arrival"]["resolution"]
            if capture["mode"] in case.get("prefix_captures",{}):
                actual=check_prefix(folder,case["prefix_captures"][capture["mode"]]["path"])
                assert actual==read(folder/"prefix-proof.json") and proof["prefix_preserved"]
                totals["prefix_proofs"]+=1
            for key,field in [("native_steps","physics_steps"),("model_steps","model_steps"),("forecast_steps","forecast_steps")]:computed[key]+=3*info[field]
            computed["neutral_replay_steps"]+=3*info["physics_steps"]
            if "recipe" in case:goal_steps+=2*case["recipe"]["physics_ticks"]
            totals["captures"]+=1;totals["verified_landings"]+=int(e["verified_target_landing"])
        computed["native_steps"]+=goal_steps
        assert all(result["resources"]["counts"][k]==v for k,v in computed.items()),(phase,computed,result["resources"])
        assert result["resources"]["total_steps"]==sum(computed.values())<=a["max_steps"]
        assert result["resources"]["counts"]["originals"]==len(result["rows"])+len(result["captures"])
        assert result["resources"]["counts"]["reconstructions"]==2*len(result["captures"])
        assert result["resources"]["optimizer_steps"]==0
        for k,v in computed.items():totals[k]+=v
        totals["originals"]+=result["resources"]["counts"]["originals"]
        totals["reconstructions"]+=result["resources"]["counts"]["reconstructions"]
        phase_results[phase]={"captures":len(result["captures"]),"steps":sum(computed.values())}
    physical_inputs={}
    for phase in ["development","confirmation"]:
        a,g=read(root/(phase+"-input-allocation.json")),read(root/(phase+"-inputs/result.json"))
        assert sha(g["generator_path"])==g["generator_sha256"]==a["generator_sha256"]
        assert sha(a["anchor_path"])==a["anchor_sha256"]
        assert g["allocation_sha256"]==sha(root/(phase+"-input-allocation.json"))
        assert not g["exclusions"]
        states={read(c["path"])["state_id"]:read(read(c["path"])["scenario_path"]) for c in g["cases"]}
        assert len(states)==(12 if phase=="development" else 60)
        physical_inputs[phase]={json.dumps(s["initial_state"],sort_keys=True) for s in states.values()}
        assert len(physical_inputs[phase])==len(states)
        assert sum(read(c["path"])["recipe"]["physics_ticks"] for c in g["cases"] if "recipe" in read(c["path"]))==g["resources"]["counts"]["native_steps"]
    assert physical_inputs["development"].isdisjoint(physical_inputs["confirmation"])
    totals["steps"]=sum(totals[k] for k in ["native_steps","model_steps","forecast_steps","neutral_replay_steps"])
    assert totals["steps"]<=150_000_000 and totals["captures"]==244<=256 and totals["prefix_proofs"]==44
    totals.update(passed=True,scope="Read-only source/input/prefix/predicate/proof/accounting audit; no native transitions",phases=phase_results,native_sha256=native_hash)
    return totals


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=Path);p.add_argument("--receipt",type=Path,required=True);args=p.parse_args()
    result=audit(args.root.resolve());write_new(args.receipt,result);print(json.dumps(result,sort_keys=True))
