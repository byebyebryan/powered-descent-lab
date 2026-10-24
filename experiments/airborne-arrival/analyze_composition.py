"""Read-only qualification and conditional decision-headroom characterization."""
import argparse
import collections
import json
import statistics
from pathlib import Path
from composition_study import read, sha, write_new

PHASES=["pilot","witnesses","diagnostics","development","confirmation"]


def analyze(root):
    results={p:read(root/p/"result.json") for p in PHASES}
    rows=[dict(row,phase=p) for p,d in results.items() for row in d["rows"]]
    captures=[dict(c,phase=p) for p,d in results.items() for c in d["captures"]]
    cases={r["case_path"]:read(r["case_path"]) for r in rows}
    groups=collections.defaultdict(dict)
    for row in results["confirmation"]["rows"]:
        c=cases[row["case_path"]]
        groups[c["state_id"]][c.get("goal_choice","direct")]=row["result"]
    assert len(groups)==60 and all(set(g)=={"direct","support_short","brake_long"} for g in groups.values())
    means={ch:statistics.mean(g[ch]["fuel_used_kg"] for g in groups.values()) for ch in ["direct","support_short","brake_long"]}
    winners=collections.Counter();gains=[];oracle=[];case_outcomes=[]
    for state,g in sorted(groups.items()):
        best=min(g,key=lambda ch:g[ch]["fuel_used_kg"])
        winners[best]+=1
        gain=g["direct"]["fuel_used_kg"]-g[best]["fuel_used_kg"]
        gains.append(gain);oracle.append(g[best]["fuel_used_kg"])
        case_outcomes.append({"state_id":state,"best_observed_choice":best,"direct_minus_best_kg":gain,
            "choices":g})
    family=collections.defaultdict(lambda:collections.defaultdict(list))
    for row in results["confirmation"]["rows"]:
        c=cases[row["case_path"]];family[c["population"]][c.get("goal_choice","direct")].append(row["result"])
    family_summary={f:{ch:{"programs":len(v),"mean_fuel_kg":statistics.mean(x["fuel_used_kg"] for x in v),
        "reached":sum(x["arrival_status"]=="reached" for x in v),"native_success":sum(x["success"] for x in v)} for ch,v in chs.items()} for f,chs in family.items()}
    proofs={(c["case"],c["mode"]):c for c in results["confirmation"]["captures"]}
    arrival_rows=[r for r in results["confirmation"]["rows"] if r["result"]["arrival_status"]!="direct"]
    qualified=[r for r in arrival_rows if r["result"]["arrival_status"]=="reached" and proofs[(r["case"],r["mode"])]["verified_target_landing"]]
    direct_rows=[r for r in results["confirmation"]["rows"] if r["result"]["arrival_status"]=="direct"]
    diagnostics=[]
    for row in results["diagnostics"]["rows"]:
        info=read(root/"diagnostics/summaries"/(row["case"]+"-"+row["mode"]+".json"))["info"]
        a=info["arrival"]
        diagnostics.append({"case":row["case"],"arrival_status":a["status"],"native_success":info["native_success"],
            "violations":a["violations"],"regimes":a["actuation"]["regime_updates"],"no_plan":info["landing_search"]["no_plan"],
            "operation_remaining_s":a["actuation"]["operation_remaining_s"],"local_remaining_s":a["deadline_remaining_s"],
            "replans":a["replans"],"ideal_pass_ever":a["actuation"]["ideal_pass_ever"],
            "interpretation":"Exposed diagnostic, unchanged complete decision info; no physical-infeasibility claim"})
    physical=[read(cases[r["case_path"]]["scenario_path"])["initial_state"] for r in direct_rows]
    ranges={"height_m":[min(s["position_m"]["y"] for s in physical),max(s["position_m"]["y"] for s in physical)],
        "abs_horizontal_position_m":[min(abs(s["position_m"]["x"]) for s in physical),max(abs(s["position_m"]["x"]) for s in physical)],
        "abs_lateral_speed_mps":[min(abs(s["velocity_mps"]["x"]) for s in physical),max(abs(s["velocity_mps"]["x"]) for s in physical)],
        "sink_mps":[min(-s["velocity_mps"]["y"] for s in physical),max(-s["velocity_mps"]["y"] for s in physical)],
        "attitude_rad":[min(s["attitude_rad"] for s in physical),max(s["attitude_rad"] for s in physical)]}
    counts=collections.Counter()
    for d in results.values():counts.update(d["resources"]["counts"])
    return {"scope":"Bounded flat airborne arrival-to-landing qualification; read-only analysis; no optimizer or new native transitions",
        "inputs":[{"path":str(root/p/"result.json"),"sha256":sha(root/p/"result.json")} for p in PHASES],
        "confirmation":{"states":60,"programs":180,"arrival_programs":120,"arrival_and_verified_landing":len(qualified),
            "direct_verified_landings":sum(proofs[(r["case"],r["mode"])]["verified_target_landing"] for r in direct_rows),
            "engineering_gate_passed":len(qualified)==120,"population_reliability_claim":False,"family_summary":family_summary,"observed_input_ranges":ranges},
        "headroom":{"interpretation":"Conditional on the unchanged first-feasible landing selector; hindsight choices are not a learned or validated policy and not an optimized direct baseline",
            "development_selected_constant":read(root/"confirmation-release.json")["development_selected_constant"],
            "mean_fuel_kg":means,"hindsight_mean_fuel_kg":statistics.mean(oracle),"hindsight_mean_saving_vs_direct_kg":statistics.mean(gains),
            "hindsight_saving_fraction":statistics.mean(gains)/means["direct"],"best_observed_choice_counts":dict(winners),
            "states_with_more_than_1kg_headroom":sum(g>1 for g in gains),"maximum_headroom_kg":max(gains),"case_outcomes":case_outcomes},
        "retained_diagnostics":diagnostics,"phase_findings":{p:d["findings"] for p,d in results.items()},
        "resources":{"counts":dict(counts),"steps":sum(d["resources"]["total_steps"] for d in results.values()),
            "phase_wall_seconds":sum(d["resources"]["seconds"] for d in results.values()),"optimizer_steps":0},
        "captures":len(captures),"verified_landing_captures":sum(c["verified_target_landing"] for c in captures),
        "preserved_prefix_captures":sum(c["prefix_preserved"] is True for c in captures),"guidance_changed":False,
        "decision":"The existing corrected controller qualifies the declared small menu and envelope; no guidance repair or default promotion. A bounded RL menu-selection experiment is supportable, with new training/validation/test inputs and an explicit conditional efficiency baseline."}


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=Path);p.add_argument("--receipt",type=Path,required=True);args=p.parse_args()
    result=analyze(args.root.resolve());write_new(args.receipt,result)
    print(json.dumps({"confirmation":result["confirmation"],"headroom":{k:v for k,v in result["headroom"].items() if k!="case_outcomes"},"resources":result["resources"]},sort_keys=True))
