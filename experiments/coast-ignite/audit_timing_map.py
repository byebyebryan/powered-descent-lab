"""Independent read-only audit of saved timing maps; executes zero native steps."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def read(path):
    return json.loads(Path(path).read_text())


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def audit(root):
    root = Path(root).resolve()
    allocation = read(root / "execution-allocation.json")
    campaign = root / "campaign"
    result = read(campaign / "result.json")
    assert result["complete"] and result["optimizer_steps"] == 0
    for path, expected in allocation["retained_hashes"].items():
        assert digest(path) == expected, path
    for model in allocation["models"].values():
        assert digest(model["path"]) == model["sha256"]
    assert digest(allocation["native_artifact"]) == allocation["native_artifact_sha256"]
    epoch = Path(allocation["source_epoch"])
    source_record = read(root / "source-epoch-a.json")
    for item in source_record["source_files"]:
        assert digest(epoch / item["path"]) == item["sha256"], item["path"]
    rows = [json.loads(s) for s in (campaign / "results.jsonl").read_text().splitlines()]
    captures = read(campaign / "report-captures.json")
    assert len(rows) == 158 * 82
    assert len(captures) == result["captured_occurrences"] <= 256
    assert len({r["capture"] for r in captures}) == len(captures)
    assert len({c["physical_sha256"] for c in allocation["cases"]}) == 158
    bests = {}
    projected_maps = read(campaign / "map-summary.json")["maps"]
    assert [m["id"] for m in projected_maps] == [c["id"] for c in allocation["cases"]]
    projected_by_id = {m["id"]: m for m in projected_maps}
    for case in allocation["cases"]:
        group = [r for r in rows if r["id"] == case["id"]]
        assert len(group) == 82
        assert {r["slot"] for r in group} == set(range(81)) | {None}
        task = read(case["task_path"])
        initial = task["scenario"]["vehicle"]["initial_fuel_kg"]
        for row in group:
            assert row["physical_sha256"] == task["physical_sha256"] == case["physical_sha256"]
            assert row["native_success"] == row["info"]["native_success"]
            assert row["metrics"]["fuel_used_kg"] == initial - row["info"]["observation"]["fuel_kg"]
            assert row["metrics"]["stop_tick"] == row["info"]["physics_steps"]
            decisions = row["info"]["decisions"]
            assert all(t == 12 * i for i, (t, _) in enumerate(decisions))
            assert sum(a for _, a in decisions) <= 1
            for tick, action in decisions:
                assert action == int(row["slot"] is not None and tick == 12 * row["slot"])
        for rule in ["M", "G"]:
            old = case["policies"][rule]["info"]
            tick = next((t for t, a in old["decisions"] if a), None)
            slot = tick // 12 if tick is not None else None
            assert next(r for r in group if r["slot"] == slot)["info"] == old
        successful = [r for r in group if r["native_success"]]
        best = min(successful, key=lambda r: (initial - r["info"]["observation"]["fuel_kg"], 81 if r["slot"] is None else r["slot"])) if successful else None
        summary = read(campaign / "maps" / (case["id"] + ".json"))
        assert projected_by_id[case["id"]] == summary
        assert summary["has_successful_point"] == bool(best)
        if best:
            assert summary["best_slot"] == best["slot"]
            assert summary["best_fuel_kg"] == best["metrics"]["fuel_used_kg"]
            bests[case["id"]] = best
    roles = read(campaign / "capture-roles.json")
    by_capture = {Path(r["capture"]).name: r for r in captures}
    for case_id, best in bests.items():
        role = next(r for r in roles if r["id"] == case_id and r["role"] == "best")
        verified = by_capture[role["capture_key"]]
        assert verified["info"] == best["info"] and verified["verified_target_landing"] is True
    for row in captures:
        folder = Path(row["capture"])
        proof, seal = read(folder / "timing-proof.json"), read(folder / "seal.json")
        for name, key in [("evaluation.json", "capture_sha256"), ("decisions.json", "decisions_sha256"),
                          ("diagnostics.json", "diagnostics_sha256"), ("timing.json", "timing_sha256")]:
            assert digest(folder / name) == proof[key]
        assert seal["evaluation_sha256"] == proof["capture_sha256"]
        assert proof["physical_replay"] and proof["controller_reconstruction"] and proof["planner_reconstruction"] and proof["summary_equality"]
        assert proof["planner_kind"] == ("frozen_model_" + row["rule"] if row["rule"] else "fixed_slot_scheduler")
        assert read(folder / "decisions.json") == row["info"]
        assert read(folder / "diagnostics.json") == row["diagnostics"]
        evaluation = read(folder / "evaluation.json")
        assert evaluation["integrity_passed"] and evaluation["replay"]["complete_episode_verified"]
        assert evaluation["verified_target_landing"] == row["verified_target_landing"] == row["native_success"]
        assert evaluation["physical_sha256"] == row["physical_sha256"]
        assert evaluation["execution"]["episode"]["final_state"]["fuel_kg"] == row["info"]["observation"]["fuel_kg"]
    count = result["resource_counts"]
    summary_steps = sum(r["info"]["physics_steps"] for r in rows)
    captured_steps = sum(r["info"]["physics_steps"] for r in captures)
    expected = {"original_episodes": len(rows) + len(captures), "original_native_steps": summary_steps + captured_steps,
                "neutral_replay_native_steps": 3 * captured_steps, "neutral_replay_passes": 3 * len(captures),
                "controller_reconstruction_episodes": len(captures), "controller_reconstruction_native_steps": captured_steps,
                "planner_reconstruction_episodes": len(captures), "planner_reconstruction_native_steps": captured_steps,
                "original_policy_decisions": sum(len(r["info"]["decisions"]) for r in rows + captures),
                "controller_reconstruction_policy_decisions": sum(len(r["info"]["decisions"]) for r in captures),
                "planner_reconstruction_policy_decisions": sum(len(r["info"]["decisions"]) for r in captures)}
    for key, value in expected.items():
        assert count[key] == value, (key, count[key], value)
    def reference_queries(r):
        ref = (r["diagnostics"].get("reference_projection") or {}).get("reference")
        return ref["physics_ticks"] + 5 if ref else 0
    assert count["diagnostic_reference_grid_queries"] == sum(reference_queries(r) for r in rows) + 3 * sum(reference_queries(r) for r in captures)
    for field, key in [("admission_actuation_checks", "admission_checks"), ("constructions", "reference_constructions")]:
        assert count[key] == sum((r["info"]["feedback"] or {}).get(field, 0) for r in rows) + 3 * sum((r["info"]["feedback"] or {}).get(field, 0) for r in captures)
    total_steps = sum(v for k, v in expected.items() if k.endswith("_native_steps"))
    assert total_steps <= allocation["max_total_native_steps"]
    return {"passed": True, "native_steps_executed_by_audit": 0,
            "summary_occurrences": len(rows), "captured_occurrences": len(captures),
            "best_successful_points_verified": len(bests), "input_identities": 158,
            "original_occurrences": count["original_episodes"], "measured_campaign_native_steps": total_steps,
            "counts": expected, "source_files_verified": len(source_record["source_files"]),
            "retained_inputs_models_and_ledgers_unchanged": True,
            "auditor_sha256": digest(__file__)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("--root", required=True)
    print(json.dumps(audit(parser.parse_args().root), sort_keys=True, indent=2))
