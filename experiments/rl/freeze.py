"""Freeze declared development selections before any held-out policy evaluation."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import statistics
from datetime import datetime, timezone

from train import write_new


def development_selection(root, design):
    seeds, attempts = [], []
    for seed in [801, 802, 803]:
        directory = root/f"training-{design}-seed-{seed}"
        request = json.loads((directory/"request.json").read_text())
        selection = json.loads((directory/"selection.json").read_text())
        if request["seed"] != seed or not selection["source_identity_unchanged"]:
            raise ValueError("training seed or source identity is invalid")
        if selection["actual_transitions"] < request["requested_transitions"]:
            raise ValueError("declared transition allocation has not completed")
        best = selection["best"]
        checkpoint = Path(best["checkpoint"])
        if hashlib.sha256(checkpoint.read_bytes()).hexdigest() != best["checkpoint_sha256"]:
            raise ValueError("selected checkpoint bytes changed")
        label = checkpoint.stem.removeprefix("checkpoint-")
        development = json.loads((directory/f"development-{label}.json").read_text())
        if development["scheduled"] != 128 or len(development["attempts"]) != 128:
            raise ValueError("selected development panel changed denominator")
        attempts.extend(development["attempts"])
        seeds.append({"seed": seed, "actual_transitions": selection["actual_transitions"],
                      "training_directory": str(directory), "checkpoint": str(checkpoint),
                      "checkpoint_sha256": best["checkpoint_sha256"], "selected_development": best,
                      "training_request_sha256": hashlib.sha256((directory/"request.json").read_bytes()).hexdigest(),
                      "selection_sha256": hashlib.sha256((directory/"selection.json").read_bytes()).hexdigest(),
                      "training_source_identities": request["source_identities"]})
    return {"design": design, "seeds": seeds, "development_attempts": len(attempts),
            "development_physical_inputs": [item["physical_sha256"] for item in attempts],
            "development_native_summary_successes": sum(item["mission_outcome"] == "success" for item in attempts),
            "median_development_final_target_distance_m": statistics.median(item["target_distance_m"] for item in attempts)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--designs", nargs="+", choices=["v1", "v2", "v3"], default=["v1", "v2"])
    args = parser.parse_args()
    designs = [development_selection(args.run, design) for design in args.designs]
    if any(design["development_physical_inputs"] != designs[0]["development_physical_inputs"] for design in designs):
        raise ValueError("design comparison used different development panels")
    winner = max(enumerate(designs), key=lambda item: (
        item[1]["development_native_summary_successes"],
        -item[1]["median_development_final_target_distance_m"], -item[0]))[1]
    args.output.mkdir()
    selected = []
    for item in winner["seeds"]:
        destination = args.output/f'checkpoint-seed-{item["seed"]}.zip'
        with destination.open("xb") as stream:
            stream.write(Path(item["checkpoint"]).read_bytes())
        if hashlib.sha256(destination.read_bytes()).hexdigest() != item["checkpoint_sha256"]:
            raise ValueError("frozen checkpoint copy identity mismatch")
        directory = Path(item["training_directory"])
        retained = args.output/f'seed-{item["seed"]}-receipts'
        retained.mkdir()
        for name in ["request.json", "selection.json"]:
            shutil.copy2(directory/name, retained/name)
        selected.append({**item, "original_checkpoint": item["checkpoint"], "checkpoint": str(destination.resolve())})
    write_new(args.output/"freeze.json", {
        "schema_version": 1, "frozen_at_utc": datetime.now(timezone.utc).isoformat(),
        "selection_rule": "sum of three selected development success counts; median of 384 endpoint distances; earlier design on exact tie",
        "design": winner["design"], "design_comparison": designs, "selected": selected,
        "held_out_used_for_selection": False, "validation_used_for_selection": False})


if __name__ == "__main__":
    main()
