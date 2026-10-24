"""Read-only crossed timing/goal analysis; never starts a native transition."""
import argparse
import collections
import json
import random
import statistics
from pathlib import Path

FIRST, FUEL = "first_feasible_v1", "minimum_fuel_grid_v1"
CHOICES = ["direct", "support_short", "brake_long"]


def read(path):
    return json.loads(Path(path).read_text())


def interval(values):
    rng = random.Random(2310241714)
    draws = sorted(statistics.mean(rng.choices(values, k=len(values))) for _ in range(5000))
    return [draws[125], draws[4874]]


def analyze(root):
    result = read(root/"comparison/result.json")
    states = {}
    for row in result["rows"]:
        states.setdefault(row["state_id"], {}).setdefault(row["mode"], {})[row["choice"]] = row
    assert len(states) == 60
    table, state_rows = {}, []
    for mode in [FIRST, FUEL]:
        assert all(set(v[mode]) == set(CHOICES) for v in states.values())
        means = {c: statistics.mean(v[mode][c]["result"]["fuel_used_kg"] for v in states.values()) for c in CHOICES}
        wins = collections.Counter()
        savings = []
        failures = []
        for state_id, values in sorted(states.items()):
            available = values[mode]
            good = {c: row for c, row in available.items() if row["result"]["success"]
                and row["result"]["arrival_status"] in ["direct", "reached"]}
            failures += [row["case"] for c, row in available.items() if c not in good]
            best = min(good, key=lambda c: (good[c]["result"]["fuel_used_kg"], CHOICES.index(c))) if good else None
            wins[best] += 1
            saving = available["direct"]["result"]["fuel_used_kg"]-good[best]["result"]["fuel_used_kg"] if best else None
            if saving is not None:
                savings.append(saving)
            if mode == FUEL:
                state_rows.append({"state_id": state_id, "population": available["direct"]["population"],
                    "best_hindsight_choice": best, "hindsight_saving_kg": saving,
                    "choices": {c: row["result"] for c, row in available.items()}})
        direct = means["direct"]
        table[mode] = {"mean_fuel_kg": means, "eligible_attempts": 180-len(failures), "failures": failures,
            "best_hindsight_counts": dict(wins), "hindsight_mean_saving_kg": statistics.mean(savings),
            "hindsight_saving_fraction": statistics.mean(savings)/direct,
            "hindsight_mean_fuel_kg": direct-statistics.mean(savings),
            "states_saving_more_than_1kg": sum(v > 1 for v in savings),
            "hindsight_saving_median_kg": statistics.median(savings),
            "hindsight_saving_range_kg": [min(savings), max(savings)],
            "exploratory_paired_hindsight_mean_interval_kg": interval(savings)}
    savings = [v[FIRST]["direct"]["result"]["fuel_used_kg"]-v[FUEL]["direct"]["result"]["fuel_used_kg"] for v in states.values()]
    current = table[FUEL]
    release = current["eligible_attempts"] == 180 and current["hindsight_saving_fraction"] >= .01 and current["states_saving_more_than_1kg"] >= 6
    costs = {mode: {"forecast_steps": sum(r["result"]["forecast_steps"] for r in result["rows"] if r["mode"] == mode),
        "model_steps": sum(r["result"]["model_steps"] for r in result["rows"] if r["mode"] == mode),
        "episode_wall_seconds": sum(r["measured_cost"]["episode_wall_seconds"] for r in result["rows"] if r["mode"] == mode),
        "episode_process_cpu_seconds": sum(r["measured_cost"]["episode_process_cpu_seconds"] for r in result["rows"] if r["mode"] == mode)} for mode in [FIRST, FUEL]}
    resources = {"steps": 0, "captures": 0, "reconstructions": 0, "originals": 0, "phase_seconds": 0., "counts": {}}
    for phase in ["pilot", "comparison", "proof-completion"]:
        if not (root/phase/"result.json").exists():
            continue
        data = read(root/phase/"result.json")
        resources["steps"] += data["resources"]["total_steps"]
        resources["captures"] += len(data["captures"])
        resources["phase_seconds"] += data["resources"]["seconds"]
        for k, v in data["resources"]["counts"].items():
            resources["counts"][k] = resources["counts"].get(k, 0)+v
    resources["reconstructions"] = resources["counts"]["reconstructions"]
    resources["originals"] = resources["counts"]["originals"]
    return {"scope": "Exposed 60-state crossed comparison; no new population or learned-policy claim",
        "states": 60, "timing": table, "direct_timing_mean_saving_kg": statistics.mean(savings),
        "direct_timing_saving_fraction": statistics.mean(savings)/table[FIRST]["mean_fuel_kg"]["direct"],
        "direct_timing_paired_mean_interval_kg": interval(savings), "costs": costs,
        "learning_release_passed": release, "release_criteria": "180 eligible attempts, >=1% mean oracle opportunity, >=6 states saving >1kg",
        "state_outcomes": state_rows, "resources": resources,
        "decision": "Release a separately frozen small selector experiment" if release else "Stop before fitting: the declared useful-opportunity gate did not pass",
        "contact_measurement": "Summary final velocities are post-contact clamped states; incoming-contact margins must be read from native captured incoming_contact, never inferred from those zeros"}


if __name__ == "__main__":
    p = argparse.ArgumentParser();p.add_argument("root", type=Path);p.add_argument("--receipt", type=Path, required=True);args = p.parse_args()
    result = analyze(args.root.resolve())
    with args.receipt.open("x") as f:json.dump(result, f, indent=2, allow_nan=False);f.write("\n")
    print(json.dumps({k: v for k, v in result.items() if k not in ["state_outcomes", "resources"]}, indent=2))
