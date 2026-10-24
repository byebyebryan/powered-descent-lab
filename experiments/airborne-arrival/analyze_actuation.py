"""Read-only characterization of a completed, immutable actuation campaign."""
import argparse
import collections
import statistics
from pathlib import Path

from actuation_study import compact, read, sha, write_new


def counts(rows, field):
    arrivals = [r[field] for r in rows if r[field]["arrival_status"] != "direct"]
    direct = [r[field] for r in rows if r[field]["arrival_status"] == "direct"]
    return {"arrival_attempts": len(arrivals),
        "reached": sum(r["arrival_status"] == "reached" for r in arrivals),
        "arrival_native_success": sum(r["success"] for r in arrivals),
        "reached_and_native_success": sum(r["success"] and r["arrival_status"] == "reached" for r in arrivals),
        "direct_attempts": len(direct), "direct_native_success": sum(r["success"] for r in direct)}


def analyze(root):
    main = read(root / "main/result.json")
    witnesses = read(root / "witnesses/result.json")
    rows = main["rows"]
    populations = {p: {m: counts([r for r in rows if r["population"] == p], m)
        for m in ["legacy", "result"]} for p in sorted({r["population"] for r in rows})}
    arrivals = [r for r in rows if r["result"]["arrival_status"] != "direct"]
    lost_landings = [r["case"] for r in arrivals if r["legacy"]["success"] and not r["result"]["success"]]
    gained_landings = [r["case"] for r in arrivals if not r["legacy"]["success"] and r["result"]["success"]]
    lost_arrivals = [r["case"] for r in arrivals if r["legacy"]["arrival_status"] == "reached" and r["result"]["arrival_status"] != "reached"]
    gained_arrivals = [r["case"] for r in arrivals if r["legacy"]["arrival_status"] != "reached" and r["result"]["arrival_status"] == "reached"]
    witness_families = {p: {m: counts([r for r in witnesses["rows"] if r["population"] == p and r["mode"] == m], "result")
        for m in ["witness", "legacy_v1", "native_pair_v2"]} for p in sorted({r["population"] for r in witnesses["rows"]})}
    diagnostics, errors, switches = [], [], []
    regimes, flags = collections.Counter(), collections.Counter()
    for phase, result in [("main", main), ("witnesses", witnesses)]:
        for row in result["rows"]:
            if row["mode"] != "native_pair_v2" or row["result"]["arrival_status"] == "direct":
                continue
            info = read(root / phase / "summaries" / (row["case"] + "-native_pair_v2.json"))["info"]
            assert compact(info) == row["result"]
            a, plant = info["arrival"], info["plant_allocation"]
            v = a["actuation"]
            errors.append(plant["maximum_aligned_tracking_error_mps2"])
            switches.append(v["engine_switches"])
            regimes.update(v["regime_updates"])
            if phase != "main":
                continue
            flags[(a["status"], v["ideal_contract_pass"], v["actuator_forecast_contract_pass"], v["ideal_pass_ever"]) ] += 1
            diagnostics.append({"case": row["case"], "summary_status": a["status"],
                "native_success": info["native_success"], "no_plan": info["landing_search"]["no_plan"],
                "regimes": v["regime_updates"], "switches": v["engine_switches"],
                "violations": a["violations"], "local_time_remaining_s": a["deadline_remaining_s"],
                "operation_time_remaining_s": v["operation_remaining_s"],
                "tilt_saturated_updates": a["tilt_saturated_updates"], "updates": a["command_updates"],
                "last_allocation": v["last"], "fuel_used_kg": row["result"]["fuel_used_kg"]})
    angles = {}
    for index, angle in enumerate([0, 30, 60, 80]):
        group = [r for r in arrivals if r["population"] == "core" and (int(r["case"].split("-")[2])-100)//9 == index]
        assert len(group) == 24
        angles[str(angle)] = {m: counts(group, m) for m in ["legacy", "result"]}
    fuel_delta = [r["result"]["fuel_used_kg"] - r["legacy"]["fuel_used_kg"] for r in arrivals
        if r["result"]["success"] and r["legacy"]["success"]]
    return {"schema_version": 1, "scope": "Read-only characterization; no new native transitions or policy selection",
        "inputs": [{"path": str(root / p / "result.json"), "sha256": sha(root / p / "result.json")} for p in ["pilot", "main", "witnesses"]],
        "overall": {m: counts(rows, m) for m in ["legacy", "result"]}, "populations": populations,
        "core_angles_degrees": angles, "witness_families": witness_families,
        "gained_arrivals": gained_arrivals, "lost_arrivals": lost_arrivals,
        "gained_landings": gained_landings, "lost_landings": lost_landings,
        "reached_without_native_success": [r["case"] for r in arrivals if r["result"]["arrival_status"] == "reached" and not r["result"]["success"]],
        "corrected_diagnostics": diagnostics,
        "corrected_all_phases": {"maximum_aligned_tracking_error_mps2": max(errors),
            "maximum_engine_switches": max(switches), "allocation_regimes": dict(regimes)},
        "final_prediction_flags": [{"status": k[0], "ideal": k[1], "actuated": k[2], "ideal_pass_ever": k[3], "count": v}
            for k, v in sorted(flags.items())],
        "fuel_corrected_minus_legacy_matched_summary_success": {"count": len(fuel_delta),
            "minimum_kg": min(fuel_delta), "median_kg": statistics.median(fuel_delta), "maximum_kg": max(fuel_delta),
            "more_fuel_cases": sum(v > 1e-8 for v in fuel_delta), "less_fuel_cases": sum(v < -1e-8 for v in fuel_delta),
            "interpretation": "Conditional diagnostic on matched native Summary successes; no efficiency optimization or all-case ranking"}}


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--receipt", type=Path, required=True)
    args = p.parse_args()
    result = analyze(args.root.resolve())
    write_new(args.receipt, result)
    print({"overall": result["overall"], "gained_arrivals": len(result["gained_arrivals"]),
        "lost_arrivals": len(result["lost_arrivals"]), "lost_landings": len(result["lost_landings"])})
