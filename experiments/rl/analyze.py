"""Create-only experiment audit; no controller execution or checkpoint selection."""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import statistics

from train import write_new


def read(path):
    return json.loads(Path(path).read_text())


def training_audit(directory, forbidden_inputs):
    directory = Path(directory)
    selection = read(directory/"selection.json")
    if not selection["source_identity_unchanged"]:
        raise ValueError("training source changed during allocation")
    best = selection["best"]
    checkpoint = Path(best["checkpoint"])
    if hashlib.sha256(checkpoint.read_bytes()).hexdigest() != best["checkpoint_sha256"]:
        raise ValueError("selected checkpoint identity changed")
    label = checkpoint.stem.removeprefix("checkpoint-")
    development = read(directory/f"development-{label}.json")
    proposals, unique, rejected, exhausted, overlap = Counter(), set(), 0, 0, set()
    outcomes, stops, reward_parts = Counter(), Counter(), {}
    simulation_times = []
    for worker in sorted(directory.glob("worker-*.jsonl")):
        with worker.open() as stream:
            for line in stream:
                item = json.loads(line)
                if item["kind"] == "proposal":
                    proposals[item["family"]] += 1
                    rejected += len(item["rejections"])
                    identity = item["physical_sha256"]
                    if identity is None:
                        exhausted += 1
                    else:
                        unique.add(identity)
                        if identity in forbidden_inputs:
                            overlap.add(identity)
                elif item["kind"] == "completed":
                    outcomes[str(item["physical_outcome"])] += 1
                    stops[json.dumps(item["native_stop"], sort_keys=True)] += 1
                    simulation_times.append(item["sim_time_s"])
                    for key, value in item["reward_totals"].items():
                        reward_parts.setdefault(key, []).append(value)
    if overlap:
        raise ValueError("consumed training inputs overlap reserved cohorts")
    return {
        "directory": str(directory), "request": read(directory/"request.json"),
        "actual_transitions": selection["actual_transitions"], "elapsed_s": selection["elapsed_s"],
        "source_identity_unchanged": True, "selected": best,
        "selected_development_outcomes": dict(Counter(str(item["physical_outcome"]) for item in development["attempts"])),
        "selected_development_attempts": development["attempts"],
        "proposal_counts": dict(proposals), "unique_training_physical_inputs": len(unique),
        "structural_rejections": rejected, "exhausted_proposals": exhausted,
        "reserved_input_overlap": len(overlap), "completed_training_outcomes": dict(outcomes),
        "completed_native_stops": dict(stops),
        "median_completed_sim_time_s": statistics.median(simulation_times) if simulation_times else None,
        "mean_reward_components": {key: statistics.mean(values) for key, values in reward_parts.items()},
    }


def verified_row(row):
    return (row["disposition"] == "executed" and row["verified_target_landing"]
            and row["execution_valid"] and row["integrity_passed"]
            and row["physical_replay_verified"] and row["complete_replay_verified"]
            and row["stop"] == "mission_terminal"
            and row["physical_outcome"] == "landed_on_target" and row["mission_outcome"] == "success")


def campaign_audit(directory):
    directory = Path(directory)
    summary, page = read(directory/"summary.json"), read(directory/"report-input.json")
    if summary["request"]["agent"] != page["agent"] or summary["integrity_passed"] != page["integrity_passed"]:
        raise ValueError("report projection differs from campaign evidence")
    attempts = summary["attempts"]
    rows = page["attempts"]
    if len(attempts) != len(rows) or len(attempts) != len(summary["request"]["attempt_indices"]):
        raise ValueError("campaign changed scheduled attempt denominator")
    if summary["integrity_passed"] and summary["evidence_before"] != summary["evidence_after"]:
        raise ValueError("campaign integrity claim contradicts its evidence snapshots")
    for attempt, row, index in zip(attempts, rows, summary["request"]["attempt_indices"]):
        if attempt["mission_index"] != index or row["mission_index"] != index:
            raise ValueError("campaign projection changed mission schedule")
        if attempt["disposition"]["kind"] != row["disposition"]:
            raise ValueError("campaign projection changed disposition")
        if row["disposition"] == "executed":
            for key in ["stop", "physical_outcome", "mission_outcome", "integrity_passed",
                        "physical_replay_verified", "complete_replay_verified", "verified_target_landing"]:
                if attempt["disposition"][key] != row[key]:
                    raise ValueError("campaign projection changed native outcome/proof")
        if row["verified_target_landing"] and not verified_row(row):
            raise ValueError("landing claim lacks complete native tuple")
    groups = {}
    for row in rows:
        key = f'{row["terrain_family"]}/direction-{row["direction"]}/stratum-{row["terrain_stratum"]}'
        group = groups.setdefault(key, {"scheduled": 0, "verified_landings": 0, "unsupported": 0})
        group["scheduled"] += 1
        group["verified_landings"] += int(summary["integrity_passed"] and verified_row(row))
        group["unsupported"] += int(row["disposition"] == "unsupported")
    return {"directory": str(directory), "agent": page["agent"], "collection_role": page["collection_role"],
            "manifest_sha256": page["manifest_sha256"], "scheduled": len(rows),
            "unique_missions": len(set(row["physical_sha256"] for row in rows)),
            "physical_inputs": [row["physical_sha256"] for row in rows],
            "verified_landings": sum(verified_row(row) for row in rows) if summary["integrity_passed"] else 0,
            "integrity_passed": summary["integrity_passed"],
            "complete_replays": sum(row["complete_replay_verified"] for row in rows),
            "dispositions": dict(Counter(row["disposition"] for row in rows)),
            "physical_outcomes": dict(Counter(str(row["physical_outcome"]) for row in rows)),
            "stops": dict(Counter(json.dumps(row["stop"], sort_keys=True) for row in rows)),
            "coverage": groups,
            "summary_sha256": hashlib.sha256((directory/"summary.json").read_bytes()).hexdigest(),
            "projection_sha256": hashlib.sha256((directory/"report-input.json").read_bytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cohorts", type=Path, required=True)
    parser.add_argument("--training", type=Path, action="append", default=[])
    parser.add_argument("--campaign", type=Path, action="append", default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    reserved = set()
    for family in ["value-noise", "layered"]:
        for role in ["development", "validation", "held-out"]:
            reserved.update(entry["physical_sha256"] for entry in read(args.cohorts/f"{family}-{role}"/"manifest.json")["entries"])
    result = {"schema_version": 1, "training": [training_audit(path, reserved) for path in args.training],
              "campaigns": [campaign_audit(path) for path in args.campaign]}
    write_new(args.output, result)


if __name__ == "__main__":
    main()
