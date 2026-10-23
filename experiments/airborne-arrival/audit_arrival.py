"""Read-only evidence/accounting audit. Executes no native transition."""
import argparse
import json
from pathlib import Path

from arrival import compact, findings, program, read, sha, write_new


def audit(root):
    counts = {"captures": 0, "sources": 0, "input_hashes": 0, "campaign_steps": 1_619_059}
    for phase in ["pilot-b", "full", "fresh", "proof-probes"]:
        result = read(root / phase / "result.json")
        allocation = read(root / (phase + "-allocation.json"))
        if phase != "proof-probes":
            expected = {(Path(c["path"]).stem, choice) for c in allocation["cases"] for choice in c["choices"]}
            assert expected == {(r["case"], r["choice"]) for r in result["rows"]}
            assert result["findings"] == findings(result["rows"])
            runtime = read(root / phase / "runtime.json")
            assert sha(runtime["native"]) == runtime["native_sha256"] == allocation["native_sha256"]
            assert sha(root / (phase + "-allocation.json")) == runtime["allocation_sha256"]
            for source in runtime["compiled_sources"]["source_files"]:
                assert sha(source["path"]) == source["sha256"]
                counts["sources"] += 1
            for case in allocation["cases"]:
                assert sha(case["path"]) == case["sha256"]
                counts["input_hashes"] += 1
        calculated = {"native_steps": 0, "model_steps": 0, "forecast_steps": 0, "neutral_replay_steps": 0}
        if phase != "proof-probes":
            for row in result["rows"]:
                for key, field in [("native_steps", "steps"), ("model_steps", "model_steps"), ("forecast_steps", "forecast_steps")]:
                    calculated[key] += row["result"][field]
        for capture in result["captures"]:
            folder = Path(capture["capture"])
            proof, seal = read(folder / "proof.json"), read(folder / "seal.json")
            assert sha(folder / "evaluation.json") == seal["evaluation_sha256"] == proof["capture_sha256"]
            assert sha(folder / "decisions.json") == proof["decisions_sha256"]
            assert all(proof[k] for k in ["physical_replay", "controller_reconstruction", "planner_reconstruction", "summary_equality"])
            e, info = read(folder / "evaluation.json"), read(folder / "decisions.json")
            assert e["integrity_passed"] and e["replay"]["complete_episode_verified"]
            assert e["verified_target_landing"] == proof["verified_target_landing"] == info["native_success"]
            expected_program = program(e["scenario"], capture["choice"])
            assert e["execution"]["descriptor"]["configuration"]["program"] == expected_program
            assert (info["arrival"]["request"] if info["arrival"] else None) == expected_program["arrival"]
            source_phase = "fresh" if capture["case"].startswith("arrival-3") else "full"
            if phase == "pilot-b":
                source_phase = phase
            expected_info = read(root / source_phase / "summaries" / f'{capture["case"]}-{capture["choice"]}.json')["info"]
            assert compact(info) == compact(expected_info)
            for key, field in [("native_steps", "physics_steps"), ("model_steps", "model_steps"), ("forecast_steps", "forecast_steps")]:
                calculated[key] += 3 * info[field]  # capture plus two reconstructions
            calculated["neutral_replay_steps"] += 3 * info["physics_steps"]
            counts["captures"] += 1
        resource = result["resources"]
        assert all(resource["counts"][k] == v for k, v in calculated.items())
        assert resource["total_steps"] == sum(calculated.values()) <= allocation["max_steps"]
        assert resource["counts"]["reconstructions"] == 2 * len(result["captures"])
        assert resource["optimizer_steps"] == 0
        counts["campaign_steps"] += resource["total_steps"]
    # Two actual invocations of the command-pair diagnostic are counted.
    actuation = read(root / "actuation-audit.json")
    counts["diagnostic_steps"] = 2 * (actuation["native_steps"] + actuation["model_steps"]) + read(root / "timing-grid-audit.json")["native_forecast_steps"]
    counts["campaign_steps"] += counts["diagnostic_steps"]
    assert counts["captures"] == 92
    assert counts["campaign_steps"] < 150_000_000
    assert read(root / "pilot/summary.json")["rows"] == read(root / "pilot-b/result.json")["rows"]
    fresh = {c["sha256"] for c in read(root / "fresh-allocation.json")["cases"]}
    assert not fresh.intersection(c["sha256"] for c in read(root / "full-allocation.json")["cases"])
    counts["passed"] = True
    counts["scope"] = "read-only source/input/capture/proof/accounting audit; software tests are separate"
    return counts


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    result = audit(args.root.resolve())
    if args.receipt:
        write_new(args.receipt, result)
    print(json.dumps(result, sort_keys=True))
