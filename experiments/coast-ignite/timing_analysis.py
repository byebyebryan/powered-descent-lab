"""Read-only timing/cost reductions. Native outcomes and failure costs stay separate."""
from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from pathlib import Path

SLOTS = tuple(range(81)) + (None,)


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def slot_order(slot):
    if slot is not None and (type(slot) is not int or not 0 <= slot <= 80):
        raise ValueError("slot must be an integer in 0..80 or never")
    return 81 if slot is None else slot


def fixed_action(slot, tick):
    slot_order(slot)
    if type(tick) is not int or tick < 0 or tick > 960 or tick % 12:
        raise ValueError("off-grid decision")
    return int(slot is not None and tick == 12 * slot)


def slot_from_info(info):
    ignitions = [t for t, a in info["decisions"] if a]
    if len(ignitions) > 1:
        raise ValueError("more than one ignition")
    return ignitions[0] // 12 if ignitions else None


def metric(task, info, diagnostics):
    initial = task["scenario"]["vehicle"]["initial_fuel_kg"]
    final = info["observation"]
    ignition = next((t for t, a in info["decisions"] if a), None)
    admission = diagnostics["admission"]
    admitted = admission is not None and admission["kind"] == "accepted"
    contact = diagnostics["incoming_contact"]
    contact_tick = contact["recorded"]["state"]["physics_step"] if contact else None
    if diagnostics["activation"] is not None:
        if abs(initial - diagnostics["activation"]["fuel_kg"]) > 1e-9:
            raise ValueError("engine-off coast changed fuel")
    if final["physics_step"] != info["physics_steps"]:
        raise ValueError("summary clock mismatch")
    if initial - final["fuel_kg"] < -1e-9:
        raise ValueError("negative fuel consumption")
    return {"fuel_used_kg": initial - final["fuel_kg"],
            "initial_fuel_kg": initial, "final_fuel_kg": final["fuel_kg"],
            "ignition_tick": ignition, "admitted": admitted,
            "contact_tick": contact_tick, "stop_tick": info["physics_steps"],
            "powered_duration_s": (contact_tick - ignition) / 120 if admitted and contact_tick is not None else None,
            "planned_burn_s": (task["reference_end"] - ignition) / 120 if ignition is not None else None,
            "contact_residual_s": (contact_tick - task["reference_end"]) / 120 if contact_tick is not None else None,
            "contact": contact["geometry"] if contact else None}


def components(slots):
    result = []
    for s in sorted(slots):
        if not result or s != result[-1][-1] + 1:
            result.append([s])
        else:
            result[-1].append(s)
    return [{"first_slot": c[0], "last_slot": c[-1], "points": len(c),
             "span_s": (c[-1] - c[0]) / 10} for c in result]


def neighbors(slot, by_slot):
    if slot is None:
        return {"left": None, "right": None, "two_sided_success": False}
    sides = [by_slot.get(s) if 0 <= s <= 80 else None for s in [slot - 1, slot + 1]]
    good = [None if r is None else bool(r["native_success"] and r["metrics"]["admitted"]) for r in sides]
    return {"left": good[0], "right": good[1], "two_sided_success": all(v is True for v in good)}


def map_summary(case, rows):
    if len(rows) != 82 or {r["slot"] for r in rows} != set(SLOTS):
        raise ValueError("require one complete 82-schedule map")
    by_slot = {r["slot"]: r for r in rows}
    if len(by_slot) != len(rows) or any(r["physical_sha256"] != case["physical_sha256"] for r in rows):
        raise ValueError("duplicate slot or physical identity mismatch")
    success = [r for r in rows if r["native_success"]]
    powered = [r for r in success if r["metrics"]["admitted"]]
    best = min(success, key=lambda r: (r["metrics"]["fuel_used_kg"], slot_order(r["slot"]))) if success else None
    best_fuel = best["metrics"]["fuel_used_kg"] if best else None
    near = [r["slot"] for r in powered if best_fuel is not None and r["metrics"]["fuel_used_kg"] <= best_fuel * 1.01]
    policies = {}
    for rule in ["M", "G"]:
        marker = case["policies"][rule]
        slot = slot_from_info(marker["info"])
        row = by_slot[slot]
        # A censored marker maps to never; original physical execution must still agree.
        if row["info"] != marker["info"]:
            raise ValueError("timing map does not preserve original policy outcome")
        regret = row["metrics"]["fuel_used_kg"] - best_fuel if row["native_success"] and best else None
        policies[rule] = {"slot": slot, "native_success": row["native_success"],
                          "fuel_used_kg": row["metrics"]["fuel_used_kg"],
                          "fuel_regret_kg": regret,
                          "fuel_regret_fraction": regret / best_fuel if regret is not None and best_fuel > 0 else None,
                          "neighbors": neighbors(slot, by_slot)}
    return {"id": case["id"], "cohort": case["cohort"], "arc": case["arc"],
            "physical_sha256": case["physical_sha256"], "schedules": 82,
            "native_successful_schedules": len(success), "successful_powered_schedules": len(powered),
            "successful_powered_components": components(r["slot"] for r in powered),
            "within_one_percent_components": components(near),
            "best_slot": best["slot"] if best else None, "has_successful_point": bool(best),
            "best_fuel_kg": best_fuel,
            "best_neighbors": neighbors(best["slot"], by_slot) if best else None,
            "policies": policies}


def distribution(values):
    values = sorted(v for v in values if v is not None)
    if not values:
        return {"n": 0}
    return {"n": len(values), "mean": statistics.mean(values), "median": statistics.median(values),
            "min": values[0], "max": values[-1],
            "p90": values[round(.90 * (len(values) - 1))]}


def cohort_summary(maps):
    result = {}
    for cohort in sorted({m["cohort"] for m in maps}):
        items = [m for m in maps if m["cohort"] == cohort]
        result[cohort] = {"arcs": len(items), "with_successful_grid_point": sum(m["has_successful_point"] for m in items),
                          "with_disconnected_powered_windows": sum(len(m["successful_powered_components"]) > 1 for m in items),
                          "best_with_successful_neighbors": sum(bool(m["best_neighbors"] and m["best_neighbors"]["two_sided_success"]) for m in items),
                          "successful_powered_points": distribution([m["successful_powered_schedules"] for m in items])}
        for rule in ["M", "G"]:
            ps = [m["policies"][rule] for m in items]
            result[cohort][rule] = {"native_successes": sum(p["native_success"] for p in ps),
                "avoidable_misses": sum(not m["policies"][rule]["native_success"] and m["has_successful_point"] for m in items),
                "fuel_regret_kg_on_success": distribution([p["fuel_regret_kg"] for p in ps]),
                "fuel_regret_fraction_on_success": distribution([p["fuel_regret_fraction"] for p in ps]),
                "at_least_one_percent_headroom": sum(p["fuel_regret_fraction"] is not None and p["fuel_regret_fraction"] >= .01 for p in ps)}
    return result


def paired_reduction(old_root):
    old_root = Path(old_root)
    ledger = old_root / "campaign/fresh/results.jsonl"
    rows = [json.loads(line) for line in ledger.read_text().splitlines()]
    packages = {rule: {r["arc"]: r for r in rows if r["seed"] == 281103 and r["rule"] == rule} for rule in ["M", "G"]}
    if any(len(p) != 1200 for p in packages.values()):
        raise ValueError("incomplete primary comparison")
    paired = []
    for arc in sorted(packages["M"]):
        a, b = [packages[r][arc] for r in ["M", "G"]]
        if not (a["verified_target_landing"] and b["verified_target_landing"]):
            continue
        path = old_root / "campaign/fresh/inputs" / f"arc-{arc}.json"
        task = read(path)
        if not a["physical_sha256"] == b["physical_sha256"] == task["physical_sha256"]:
            raise ValueError("paired physical identity mismatch")
        ma, mb = [metric(task, r["info"], r["diagnostics"]) for r in [a, b]]
        paired.append({"arc": arc, "M": ma, "G": mb,
                       "input_sha256": sha(path), "reference_end": task["reference_end"],
                       "fuel_difference_kg": ma["fuel_used_kg"] - mb["fuel_used_kg"]})
    def summarize(subset):
        return {"arcs": len(subset), "M_fuel_mean_kg": statistics.mean(r["M"]["fuel_used_kg"] for r in subset),
                "G_fuel_mean_kg": statistics.mean(r["G"]["fuel_used_kg"] for r in subset),
                "fuel_M_minus_G_kg": distribution([r["fuel_difference_kg"] for r in subset]),
                "M_lower_fuel": sum(r["fuel_difference_kg"] < 0 for r in subset)}
    return {"schema_version": 1, "post_hoc": True, "native_steps": 0,
            "fresh_ledger_sha256": sha(ledger), "both_success": summarize(paired),
            "endpoint_within_two_ticks": summarize([r for r in paired if all(abs(r[k]["contact_tick"] - r["reference_end"]) <= 2 for k in ["M", "G"])]),
            "largest_M_fuel_increases": sorted(paired, key=lambda r: r["fuel_difference_kg"], reverse=True)[:2]}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--old-root", required=True)
    print(json.dumps(paired_reduction(parser.parse_args().old_root), sort_keys=True, indent=2))
