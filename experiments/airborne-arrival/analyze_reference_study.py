"""Paired original-level analysis; failed missions never earn fuel credit."""
import argparse
import math
import statistics
from collections import Counter
from pathlib import Path

from arrival import read, write_new

FIRST, FUEL, EXTENDED = "first_feasible_v1", "minimum_fuel_grid_v1", "minimum_fuel_extended_grid_v1"
MODES = [FIRST,FUEL,EXTENDED]


def distribution(values):
    import numpy as np
    if not values: return None
    return {"n":len(values),"mean":statistics.mean(values),"median":statistics.median(values),
        "p05":float(np.percentile(values,5)),"p95":float(np.percentile(values,95)),"max":max(values),"min":min(values)}


def lower_bound(successes, n):
    if successes == 0: return 0.
    if successes == n: return .05**(1/n)
    def tail(p):
        return sum(math.exp(math.lgamma(n+1)-math.lgamma(k+1)-math.lgamma(n-k+1)+
            k*math.log(p)+(n-k)*math.log1p(-p)) for k in range(successes,n+1))
    lo,hi = 1e-15,1-1e-15
    for _ in range(70):
        mid = (lo+hi)/2
        if tail(mid) > .05: hi=mid
        else: lo=mid
    return (lo+hi)/2


def paired(rows, left, right):
    import numpy as np
    states = {}
    for r in rows: states.setdefault(r["state_id"],{})[r["mode"]]=r["result"]
    good = [v for v in states.values() if all(v[m]["success"] for m in [left,right])]
    if not good: return {"successful_pairs":0,"originals":len(states),"conditional":True}
    a = np.array([v[left]["fuel_used_kg"] for v in good]); b = np.array([v[right]["fuel_used_kg"] for v in good])
    # Resample originals, preserving both outcomes in every paired draw.
    index = np.random.default_rng(2310241907).integers(0,len(good),size=(10000,len(good)))
    delta = a-b
    relative = 1-b[index].mean(axis=1)/a[index].mean(axis=1)
    return {"successful_pairs":len(good),"originals":len(states),"conditional":len(good)!=len(states),
        "left":left,"right":right,"mean_saving_kg":float(delta.mean()),"saving_kg":distribution(delta.tolist()),
        "saving_fraction":float(1-b.mean()/a.mean()),
        "mean_saving_kg_ci95":np.percentile(delta[index].mean(axis=1),[2.5,97.5]).tolist(),
        "saving_fraction_ci95":np.percentile(relative,[2.5,97.5]).tolist(),
        "bootstrap_replicates":10000,"bootstrap_seed":2310241907,
        "right_uses_more_fuel":int((delta < -1e-9).sum())}


def analyze(root):
    chosen = read(root/"reference-freeze.json")["chosen_reference"]
    audit = read(root/"evidence-audit.json"); assert audit["passed"]
    populations = {}
    for phase in ["primary","boundary","regressions"]:
        rows = read(root/phase/"result.json")["rows"]
        profiles = {}
        for mode in sorted({r["mode"] for r in rows},key=MODES.index):
            selected = [r for r in rows if r["mode"]==mode]
            successes = sum(r["result"]["success"] for r in selected)
            contacts = [c for c in audit["contacts"] if c["phase"]==phase and c["mode"]==mode]
            profiles[mode] = {"originals":len(selected),"verified_landings":successes,
                "lower_bound_95":lower_bound(successes,len(selected)) if phase=="primary" else None,
                "fuel_successes_kg":distribution([r["result"]["fuel_used_kg"] for r in selected if r["result"]["success"]]),
                "arrival_reached":sum(r["result"]["arrival_status"]=="reached" for r in selected),
                "selected_coast_seconds":dict(sorted(Counter(r["result"]["coast_ticks"]/120 for r in selected if r["result"]["coast_ticks"] is not None).items())),
                "selected_duration_seconds":dict(sorted(Counter(r["result"]["duration_ticks"]/120 for r in selected if r["result"]["duration_ticks"] is not None).items())),
                "contact":{k:distribution([abs(c[k]) for c in contacts]) for k in
                    ["speed_mps","lateral_mps","vertical_mps","attitude_rad","angular_rate_radps","pad_offset_m"]}}
        populations[phase] = {"originals":len({r["state_id"] for r in rows}),"profiles":profiles,
            "paired":{left+"__"+right:paired(rows,left,right) for left,right in [(FIRST,FUEL),(FIRST,chosen),(FUEL,chosen)] if left in profiles and right in profiles}}
    benchmark = read(root/"benchmark/result.json")
    costs = {}
    for mode in MODES:
        rows = [r for r in benchmark["rows"] if r["mode"]==mode and r["role"]=="measured"]
        costs[mode] = {"selection_ms":distribution([r["measurements"]["selection_wall_ns"][0]/1e6 for r in rows]),
            "planning_frame_ms":distribution([r["measurements"]["planning_frame_wall_ns"][0]/1e6 for r in rows]),
            "ordinary_frame_us":distribution([v/1e3 for r in rows for v in r["measurements"]["ordinary_frame_wall_ns"]]),
            "episode_process_cpu_ms":distribution([r["measured_cost"]["episode_process_cpu_seconds"]*1e3 for r in rows]),
            "forecast_steps":distribution([r["result"]["forecast_steps"] for r in rows])}
    accepted = populations["primary"]["profiles"][chosen]["verified_landings"]==300
    boundary = populations["boundary"]["profiles"][chosen]["verified_landings"]==60
    regressions = populations["regressions"]["profiles"][chosen]
    return {"chosen_reference":chosen,"primary_99_percent_gate_passed":accepted,
        "boundary_engineering_gate_passed":boundary,"regression_gate_passed":regressions["verified_landings"]==21 and regressions["arrival_reached"]==21,
        "populations":populations,"costs":costs,"development":read(root/"reference-freeze.json"),
        "charged_steps":audit["total_steps"],"new_captures":audit["captures"],"fitting_steps":0,"rl_updates":0,
        "scope":"Frozen 12-cell independent mixture, fixed vehicle/fuel/Earth/flat plant; no family-wise reliability, dynamics robustness, global optimum or default promotion"}


if __name__ == "__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=Path);p.add_argument("output",type=Path);args=p.parse_args()
    write_new(args.output,analyze(args.root.resolve()))
