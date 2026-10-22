"""Actual application fault/parity checks on temporary native source fixtures.

Historical integration check for the retired procedural PPO package.
Requires its retained Python environment and compatible native CLI/report examples.
These are software checks; no reserved cohort, checkpoint or optimizer is used.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]/"studies/rl_procedural_ppo"))
import pd_native
from evaluate import campaign
from pd_env import native_module_paths
from verify import verify_campaign


def read(path):
    return json.loads(path.read_text())


def check():
    receipts = []
    with tempfile.TemporaryDirectory(prefix="pd-preparation-composition-") as temp:
        root = Path(temp)
        native = root/"native.json"
        native.write_text(json.dumps({"native_modules": {
            str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in native_module_paths()},
            "sources": []}))
        # The actual installed native binary is bound; SDK scanning is replaced
        # only in this software fixture to keep fault checks small and bounded.
        with patch("evaluate.backend_paths", return_value=([], [])):
            for kind in ("v1", "v2", "missing", "excluded-drift"):
                collection = root/kind
                if kind == "v1":
                    command = ["target/release/examples/missions", "generate", "--output", str(collection),
                               "--seed", "45", "--count", "2", "--family", "value-noise"]
                else:
                    request = read(Path("studies/controller_rl_preparation/requests/stress-support-a.json"))
                    request.update(seed=981, indices=list(range(8)), population={"kind": "broad"})
                    path = root/(kind+".json"); path.write_text(json.dumps(request))
                    command = ["target/release/examples/missions", "collect", "--request", str(path), "--output", str(collection)]
                subprocess.run(command, check=True, capture_output=True)
                manifest = read(collection/"manifest.json")
                if kind == "missing":
                    (collection/manifest["entries"][0]["file"]).unlink()
                elif kind == "excluded-drift":
                    entry = next(r["proposal"] for r in manifest["receipts"]
                                 if r["proposal"] and r["disposition"]["kind"] != "accepted")
                    (collection/entry["file"]).write_text("{}")
                    native_output = root/"invalid-native-campaign"
                    preflight = subprocess.run(["target/release/pd-benchmark", "--collection", str(collection),
                        "--output", str(native_output)], capture_output=True, text=True)
                    assert preflight.returncode != 0 and not native_output.exists()
                    assert not list(collection.glob("usage-*.json"))
                output = root/(kind+"-campaign")
                failed = kind in ("missing", "excluded-drift")
                try:
                    campaign(collection, None, output, native, count=2, test_policy=True, verify_saved=False)
                except RuntimeError:
                    if not failed: raise
                else:
                    if failed: raise AssertionError("incomplete inputs promoted campaign integrity")
                page, summary = read(output/"report-input.json"), read(output/"summary.json")
                assert len(page["attempts"]) == len(summary["request"]["attempt_indices"]) == 2
                assert (output/"report.html").is_file()
                if kind == "missing":
                    row = page["attempts"][0]
                    assert row["disposition"] == "input_invalid"
                    assert row["input_metadata_unavailable"] and row["direction"] == 0
                    assert "geometry" not in row and "terrain_stratum" not in row
                    assert not any(r["verified_target_landing"] for r in page["attempts"])
                    assert not any(r.get("presentation") for r in page["attempts"])
                if failed:
                    assert not summary["integrity_passed"]
                    try: verify_campaign(output)
                    except ValueError: pass
                    else: raise AssertionError("independent verifier accepted failed campaign")
                else:
                    verification = verify_campaign(output)
                    assert verification["replayed"] == 2
                    assert all(r.get("presentation") for r in page["attempts"])
                    if kind == "v2":
                        assert all("geometry" in r and "terrain_stratum" not in r for r in page["attempts"])
                        original = read(output/"report-input.json")
                        tampered = json.loads(json.dumps(original))
                        tampered["attempts"][0]["geometry"]["chord_barrier_m"] += 1
                        (output/"report-input.json").write_text(json.dumps(tampered))
                        try: verify_campaign(output)
                        except ValueError as error: assert "metadata" in str(error)
                        else: raise AssertionError("independent verifier accepted altered geometry")
                receipts.append({"fixture": kind, "scheduled": 2, "passed": True,
                                 "native_binary_bound": True, "sdk_scope": "fixture only; scanning mocked"})
    return {"schema_version": 1, "software_fixtures": receipts,
            "reserved_cohorts_used": False, "training_transitions": 0,
            "measured_study_attempts": 0, "all_passed": True}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = check()
    with args.output.open("x") as stream:
        json.dump(result, stream, indent=2); stream.write("\n")
