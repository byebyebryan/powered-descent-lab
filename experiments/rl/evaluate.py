"""Frozen policy -> ordinary native captures -> common campaign/report schema."""
from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import subprocess
import time
from pathlib import Path

import pd_native
import torch
from stable_baselines3 import PPO

from pd_env import native_module_paths
from train import write_new
from provenance import backend_paths


def resolve_accounted(collection, entries, usage_id):
    """One native bulk read normally; isolate bad inputs without dropping attempts."""
    def read(group, suffix):
        try:
            missions = json.loads(pd_native.load_collection_missions(
                str(collection), [entry["index"] for entry in group], usage_id+suffix, "evaluation"))
            if len(missions) != len(group):
                raise RuntimeError("native resolver changed the declared schedule length")
            return [(mission, None) for mission in missions]
        except Exception as error:
            if len(group) == 1:
                return [(None, repr(error))]
            middle = len(group)//2
            return read(group[:middle], suffix+"l")+read(group[middle:], suffix+"r")
    return read(entries, "")


def execute_attempt(model, env_class, mission, descriptor, output, capture,
                    source_files, inputs, report):
    """Stage failures explicitly; native captures remain the sole outcome proof."""
    stage = "initialization_failed"
    env, result, wall_us = None, None, None
    try:
        before = pd_native.evidence_snapshot(source_files, inputs)
        env = env_class(fixed_missions=[mission], captured=True, agent_id=descriptor["implementation"])
        observation, _ = env.reset()
        stage = "execution_failed"
        started = time.monotonic()
        done = False
        while not done:
            action, _ = model.predict(observation, deterministic=True)
            observation, _, done, _, _ = env.step(action)
        wall_us = int((time.monotonic()-started)*1e6)
        stage = "capture_failed"
        after = pd_native.evidence_snapshot(source_files, inputs)
        result = json.loads(env.episode.save_capture(json.dumps(descriptor), before, after,
                           str(output/capture), wall_us, report))
        verification = json.loads(pd_native.verify_capture(str(output/capture)))
        for key in ["integrity_passed", "replay", "verified_target_landing"]:
            if verification[key] != result[key]:
                raise RuntimeError("independent saved-command verification differs from capture assessment")
        if result["physical_sha256"] != mission["physical_sha256"]:
            raise RuntimeError("capture physical identity differs from scheduled mission")
        disposition = {"kind": "executed", "capture": capture, "stop": result["stop"],
                       "physical_outcome": result["manifest"]["physical_outcome"],
                       "mission_outcome": result["manifest"]["mission_outcome"],
                       "integrity_passed": result["integrity_passed"],
                       "physical_replay_verified": result["replay"]["physical_prefix_verified"],
                       "complete_replay_verified": result["replay"]["complete_episode_verified"],
                       "verified_target_landing": result["verified_target_landing"]}
        return disposition, result, float(env.raw[9]), wall_us
    except Exception as error:
        return {"kind": stage, "detail": repr(error)}, None, None, wall_us


def campaign(collection, checkpoint, output, native_receipt, count=None, report=True, design="v1", freeze_receipt=None):
    output, collection = Path(output), Path(collection)
    checkpoint, native_receipt = Path(checkpoint), Path(native_receipt)
    manifest_payload = (collection/"manifest.json").read_bytes()
    manifest = json.loads(manifest_payload)
    if count is not None and not 1 <= count <= len(manifest["entries"]):
        raise ValueError("count must be within the declared collection size")
    if design not in ["v1", "v2"]:
        raise ValueError("unknown reward design")
    entries = manifest["entries"][:count] if count is not None else manifest["entries"]
    if not entries:
        raise ValueError("evaluation requires scheduled resolved inputs")
    output.mkdir()
    module = importlib.import_module("pd_env" if design == "v1" else "pd_env_"+design)
    env_class = module.ProceduralLandingEnv
    configuration = module.CONFIGURATION
    digest = hashlib.sha256(checkpoint.read_bytes()).hexdigest()
    descriptor = {"schema_version": 1, "implementation": "ppo-terrain-"+design,
                  "configuration": configuration, "checkpoint_sha256": digest}
    if freeze_receipt is not None:
        frozen = json.loads(Path(freeze_receipt).read_text())
        selected = [item for item in frozen["selected"] if item["checkpoint_sha256"] == digest]
        if frozen["design"] != design or len(selected) != 1:
            raise ValueError("checkpoint/design is absent from the predeclared selection freeze")
        descriptor["configuration"] = {**configuration, "training_seed": selected[0]["seed"],
            "training_transitions": selected[0]["actual_transitions"],
            "selection_receipt_sha256": hashlib.sha256(Path(freeze_receipt).read_bytes()).hexdigest()}
    request = {"schema_version": 1, "manifest_sha256": hashlib.sha256(manifest_payload).hexdigest(),
               "agent": descriptor, "attempt_indices": [entry["index"] for entry in entries]}
    write_new(output/"request.json", request)
    usage_id = "frozen-eval-"+hashlib.sha256(str(output).encode()).hexdigest()[:24]
    resolved = resolve_accounted(collection, entries, usage_id)
    receipt = json.loads(native_receipt.read_text())
    for path, expected in receipt["native_modules"].items():
        if hashlib.sha256(Path(path).read_bytes()).hexdigest() != expected:
            raise RuntimeError("loaded native extension differs from retained build receipt")
    source_files = [str(Path(__file__).resolve()), str(Path(__file__).with_name("provenance.py").resolve()), str(Path(module.__file__).resolve()),
                    str(Path(__file__).with_name("pd_env.py").resolve()),
                    str(Path(__file__).with_name("train.py").resolve()),
                    str(Path(__file__).with_name("pyproject.toml").resolve()),
                    str(Path(__file__).with_name("uv.lock").resolve()),
                    *[str(Path(item["retained_path"]).resolve()) for item in receipt["sources"]]]
    for item in receipt["sources"]:
        if hashlib.sha256(Path(item["retained_path"]).read_bytes()).hexdigest() != item["sha256"]:
            raise RuntimeError("retained compiled-source bytes changed")
    fixed_inputs = [str(checkpoint.resolve()), str(native_receipt.resolve()),
                    *map(str, native_module_paths())]
    if freeze_receipt is not None:
        fixed_inputs.append(str(Path(freeze_receipt).resolve()))
    immutable_inputs = [str((collection/name).resolve()) for name in ["manifest.json", "request.json"]]
    immutable_inputs.extend(str((collection/entry["file"]).resolve()) for entry in manifest["entries"])
    # Initial exposure records are rechecked; later appends remain a journal.
    immutable_inputs.extend(str(path.resolve()) for path in sorted(collection.glob("usage-*.json")))
    sdk_sources, sdk_binaries = backend_paths()
    campaign_sources = source_files+sdk_sources
    campaign_inputs = fixed_inputs+immutable_inputs+sdk_binaries
    before = pd_native.evidence_snapshot(campaign_sources, campaign_inputs)
    torch.set_num_threads(1)
    try:
        model = PPO.load(checkpoint, device="cpu")
        model.policy.set_training_mode(False)
        if hashlib.sha256(checkpoint.read_bytes()).hexdigest() != digest:
            raise RuntimeError("checkpoint changed during model loading")
        model_error = None
    except Exception as error:
        model, model_error = None, repr(error)
    attempts, rows = [], []
    for position, (entry, (mission, resolution_error)) in enumerate(zip(entries, resolved)):
        selected = str((collection/entry["file"]).resolve())
        capture = f"attempt-{position:06}"
        result, fuel, wall_us = None, None, None
        if not resolution_error and hashlib.sha256(Path(selected).read_bytes()).hexdigest() != entry["sha256"]:
            resolution_error = "selected mission changed after collection resolution"
        if resolution_error:
            disposition = {"kind": "input_invalid", "detail": resolution_error}
        elif model_error:
            disposition = {"kind": "initialization_failed", "detail": model_error}
        else:
            disposition, result, fuel, wall_us = execute_attempt(
                model, env_class, mission, descriptor, output, capture, source_files, fixed_inputs+[selected], report)
        detail = disposition.get("detail")
        attempt = {"attempt_index": position, "mission_index": entry["index"], "disposition": disposition}
        write_new(output/f"attempt-{position:06}.json", attempt)
        attempts.append(attempt)
        row = {"attempt_index": position, "mission_index": entry["index"],
               "physical_sha256": entry["physical_sha256"],
               "terrain_family": manifest["request"]["recipe"]["generator_version"],
               "direction": (mission or {}).get("direction", 1 if (entry["index"]//4)%2 == 0 else -1),
               "terrain_stratum": (mission or {}).get("terrain_stratum", entry["index"]%4),
               "disposition": disposition["kind"], "detail": detail, "stop": None,
               "physical_outcome": None, "mission_outcome": None, "execution_valid": False,
               "integrity_passed": False, "physical_replay_verified": False,
               "complete_replay_verified": False, "verified_target_landing": False,
               "sim_time_s": None, "fuel_kg": None, "wall_time_us": wall_us, "report_href": None}
        if result:
            for key in ["stop", "physical_outcome", "mission_outcome", "integrity_passed",
                        "physical_replay_verified", "complete_replay_verified", "verified_target_landing"]:
                row[key] = disposition[key]
            row.update(execution_valid=result["failure"] is None,
                       sim_time_s=result["manifest"]["sim_time_s"], fuel_kg=fuel,
                       report_href=f"{capture}/report.html" if report else None)
        rows.append(row)
        print(json.dumps({"event": "native_attempt", "position": position, **disposition}), flush=True)
    after = pd_native.evidence_snapshot(campaign_sources, campaign_inputs)
    integrity = json.loads(before) == json.loads(after)
    summary = {"schema_version": 1, "request": request, "unique_scheduled_missions": len(entries),
               "attempts": attempts, "evidence_before": json.loads(before), "evidence_after": json.loads(after),
               "integrity_passed": integrity}
    write_new(output/"summary.json", summary)
    exposures = [json.loads(path.read_text())["exposure"] for path in collection.glob("usage-*.json")]
    page = {"schema_version": 1, "title": f"Frozen procedural PPO campaign · {design}",
            "agent": descriptor, "integrity_passed": integrity, "unique_scheduled_missions": len(entries),
            "manifest_sha256": request["manifest_sha256"], "collection_role": manifest["request"]["role"],
            "prior_training_or_tuning": any(value in ["training", "tuning"] for value in exposures),
            "recorded_input_inspection": "input_inspection" in exposures,
            "attempts": rows, "summary_href": "summary.json"}
    write_new(output/"report-input.json", page)
    if report:
        subprocess.run(["rtk", "proxy", "target/release/examples/campaign", str(output/"report-input.json"),
                        str(output/"report.html")], check=True)
    if not integrity or any(item["disposition"]["kind"] != "executed" for item in attempts):
        raise RuntimeError("campaign retained integrity/execution failures; consult dispositions")
    return summary


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--collection", required=True, type=Path)
    parser.add_argument("--checkpoint", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--native-receipt", required=True, type=Path)
    parser.add_argument("--count", type=int)
    parser.add_argument("--design", choices=["v1", "v2"], default="v1")
    parser.add_argument("--freeze-receipt", type=Path)
    args = parser.parse_args()
    campaign(args.collection, args.checkpoint, args.output, args.native_receipt, args.count,
             design=args.design, freeze_receipt=args.freeze_receipt)


if __name__ == "__main__":
    main()
