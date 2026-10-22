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
                    source_files, inputs, report, verify_saved=True):
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
        if verify_saved:
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


def campaign(collection, checkpoint, output, native_receipt, count=None, report=True, design="v1", freeze_receipt=None,
             test_policy=False, request_file=None, verify_saved=True):
    output, collection = Path(output), Path(collection)
    checkpoint, native_receipt = Path(checkpoint) if checkpoint is not None else None, Path(native_receipt)
    manifest_payload = (collection/"manifest.json").read_bytes()
    manifest = json.loads(manifest_payload)
    if count is not None and not 1 <= count <= len(manifest["entries"]):
        raise ValueError("count must be within the declared collection size")
    if design not in ["v1", "v2", "v3"]:
        raise ValueError("unknown reward design")
    if test_policy == (checkpoint is not None) or (test_policy and freeze_receipt is not None):
        raise ValueError("choose a checkpoint or the untrained integration policy")
    if count is not None and request_file is not None:
        raise ValueError("choose count or an explicit request")
    entries = manifest["entries"][:count] if count is not None else manifest["entries"]
    declared_request = json.loads(Path(request_file).read_text()) if request_file is not None else None
    if declared_request is not None:
        by_index = {entry["index"]: entry for entry in manifest["entries"]}
        indices = declared_request["attempt_indices"]
        if not indices or len(indices) > 100000 or any(type(i) is not int or i not in by_index for i in indices):
            raise ValueError("request schedule requires 1..100000 admitted indices")
        entries = [by_index[i] for i in indices]
    if not entries:
        raise ValueError("evaluation requires scheduled resolved inputs")
    output.mkdir()
    module = importlib.import_module("pd_env" if design == "v1" else "pd_env_"+design)
    env_class = module.ProceduralLandingEnv
    configuration = {**module.CONFIGURATION, "policy_inference": {
        "deterministic": True, "device": "cpu", "torch_num_threads": 1, "batch_size": 1}}
    digest = hashlib.sha256(checkpoint.read_bytes()).hexdigest() if checkpoint else None
    retained_training_inputs = []
    descriptor = {"schema_version": 1, "implementation": "deterministic-integration-policy-v1" if test_policy else "ppo-terrain-"+design,
                  "configuration": configuration, "checkpoint_sha256": digest}
    if test_policy:
        descriptor["configuration"] = {**configuration, "integration_policy": "constant normalized action [0, 0]; untrained"}
    if freeze_receipt is not None:
        frozen = json.loads(Path(freeze_receipt).read_text())
        selected = [item for item in frozen["selected"] if item["checkpoint_sha256"] == digest]
        if frozen["design"] != design or len(selected) != 1:
            raise ValueError("checkpoint/design is absent from the predeclared selection freeze")
        for binding in selected[0]["retained_training_sources"]:
            path = Path(binding["retained_path"])
            if hashlib.sha256(path.read_bytes()).hexdigest() != binding["sha256"]:
                raise ValueError("retained training source/native-module bytes changed")
            retained_training_inputs.append(str(path.resolve()))
        descriptor["configuration"] = {**configuration, "training_seed": selected[0]["seed"],
            "training_transitions": selected[0]["actual_transitions"],
            "selection_receipt_sha256": hashlib.sha256(Path(freeze_receipt).read_bytes()).hexdigest()}
    request = {"schema_version": 1, "manifest_sha256": hashlib.sha256(manifest_payload).hexdigest(),
               "agent": descriptor, "attempt_indices": [entry["index"] for entry in entries]}
    if declared_request is not None and declared_request != request:
        raise ValueError("declared request differs from manifest or actual approach")
    write_new(output/"request.json", request)
    usage_id = "frozen-eval-"+hashlib.sha256(str(output).encode()).hexdigest()[:24]
    resolved = resolve_accounted(collection, entries, usage_id)
    description = json.loads(pd_native.describe_collection(str(collection), request["attempt_indices"]))
    receipt = json.loads(native_receipt.read_text())
    if {str(Path(p).resolve()) for p in receipt["native_modules"]} != set(map(str, native_module_paths())):
        raise RuntimeError("receipt does not bind the native modules actually loaded")
    for path, expected in receipt["native_modules"].items():
        if hashlib.sha256(Path(path).read_bytes()).hexdigest() != expected:
            raise RuntimeError("loaded native extension differs from retained build receipt")
    source_files = [str(Path(__file__).resolve()), str(Path(__file__).with_name("provenance.py").resolve()), str(Path(module.__file__).resolve()),
                    str(Path(__file__).with_name("pd_env.py").resolve()),
                    str(Path(__file__).with_name("train.py").resolve()),
                    str(Path(__file__).with_name("pyproject.toml").resolve()),
                    str(Path(__file__).with_name("uv.lock").resolve()),
                    *[str(Path(item["retained_path"]).resolve()) for item in receipt["sources"]]]
    if design == "v3":
        source_files.append(str(Path(__file__).with_name("pd_env_v2.py").resolve()))
    if test_policy:
        source_files.append(str(Path(__file__).with_name("integration_policy.py").resolve()))
    for item in receipt["sources"]:
        if hashlib.sha256(Path(item["retained_path"]).read_bytes()).hexdigest() != item["sha256"]:
            raise RuntimeError("retained compiled-source bytes changed")
    fixed_inputs = [str(native_receipt.resolve()),
                    *map(str, native_module_paths())]
    if checkpoint is not None:
        fixed_inputs.append(str(checkpoint.resolve()))
    if request_file is not None:
        fixed_inputs.append(str(Path(request_file).resolve()))
    if freeze_receipt is not None:
        fixed_inputs.append(str(Path(freeze_receipt).resolve()))
        fixed_inputs.extend(retained_training_inputs)
    immutable_inputs = [str((collection/name).absolute()) for name in description["immutable_files"]]
    # Initial exposure records are rechecked; later appends remain a journal.
    immutable_inputs.extend(str(path.resolve()) for path in sorted(collection.glob("usage-*.json")))
    sdk_sources, sdk_binaries = backend_paths()
    campaign_sources = source_files+sdk_sources
    missing_inputs = [path for path in immutable_inputs if not Path(path).is_file() or Path(path).is_symlink()]
    collection_evidence = output/"collection-evidence.json"
    write_new(collection_evidence, {"schema_version": 1,
        "manifest_sha256": request["manifest_sha256"], "description": description,
        "unavailable_immutable_inputs": missing_inputs})
    # A failed campaign binds only extant regular evidence and explicitly records
    # the unavailable inventory. It cannot execute or claim complete integrity.
    campaign_inputs = fixed_inputs+[p for p in immutable_inputs if p not in missing_inputs]+sdk_binaries+[str(collection_evidence.resolve())]
    before = pd_native.evidence_snapshot(campaign_sources, campaign_inputs)
    torch.set_num_threads(1)
    try:
        if test_policy:
            from integration_policy import DeterministicIntegrationPolicy
            model = DeterministicIntegrationPolicy()
        else:
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
        metadata = description["metadata"].get(str(entry["index"]))
        metadata_error = description["metadata_errors"].get(str(entry["index"]))
        if metadata_error:
            resolution_error = metadata_error
        if missing_inputs:
            resolution_error = "declared immutable evidence unavailable: "+repr(missing_inputs)
        if not resolution_error and hashlib.sha256(Path(selected).read_bytes()).hexdigest() != entry["sha256"]:
            resolution_error = "selected mission changed after collection resolution"
        if resolution_error:
            disposition = {"kind": "input_invalid", "detail": resolution_error}
        elif model_error:
            disposition = {"kind": "initialization_failed", "detail": model_error}
        else:
            disposition, result, fuel, wall_us = execute_attempt(
                model, env_class, mission, descriptor, output, capture, source_files, fixed_inputs+[selected], report,
                verify_saved=verify_saved)
        detail = disposition.get("detail")
        attempt = {"attempt_index": position, "mission_index": entry["index"], "disposition": disposition}
        write_new(output/f"attempt-{position:06}.json", attempt)
        attempts.append(attempt)
        row = {"attempt_index": position, "mission_index": entry["index"],
               "physical_sha256": entry["physical_sha256"],
               "terrain_family": metadata["family"] if metadata else description["family"],
               "direction": metadata["direction"] if metadata else 0,
               "disposition": disposition["kind"], "detail": detail, "stop": None,
               "physical_outcome": None, "mission_outcome": None, "execution_valid": False,
               "integrity_passed": False, "physical_replay_verified": False,
               "complete_replay_verified": False, "verified_target_landing": False,
               "sim_time_s": None, "fuel_kg": None, "wall_time_us": wall_us, "report_href": None}
        if metadata:
            if metadata["stratum"] is not None:
                row["terrain_stratum"] = metadata["stratum"]
            if metadata["geometry"] is not None:
                row["geometry"] = metadata["geometry"]
        else:
            row["input_metadata_unavailable"] = metadata_error or resolution_error or "native metadata unavailable"
        if result:
            for key in ["stop", "physical_outcome", "mission_outcome", "integrity_passed",
                        "physical_replay_verified", "complete_replay_verified", "verified_target_landing"]:
                row[key] = disposition[key]
            row.update(execution_valid=result["failure"] is None,
                       sim_time_s=result["manifest"]["sim_time_s"], fuel_kg=fuel,
                       report_href=f"{capture}/report.html" if report else None)
            if result.get("presentation"):
                row["presentation"] = {**result["presentation"],
                    "preview_href": capture+"/"+result["presentation"]["preview_href"]}
        rows.append(row)
        print(json.dumps({"event": "native_attempt", "position": position, **disposition}), flush=True)
    after = pd_native.evidence_snapshot(campaign_sources, campaign_inputs)
    end_description = json.loads(pd_native.describe_collection(str(collection), request["attempt_indices"]))
    integrity = (json.loads(before) == json.loads(after) and not missing_inputs
                 and description["complete_verification_error"] is None
                 and end_description["complete_verification_error"] is None)
    unique = len({entry["physical_sha256"] for entry in entries})
    summary = {"schema_version": 1, "request": request, "unique_scheduled_missions": unique,
               "attempts": attempts, "evidence_before": json.loads(before), "evidence_after": json.loads(after),
               "integrity_passed": integrity}
    write_new(output/"summary.json", summary)
    exposures = [json.loads(path.read_text())["exposure"] for path in collection.glob("usage-*.json")]
    page = {"schema_version": 1, "title": f"Procedural {'untrained integration' if test_policy else 'frozen PPO'} campaign · {design}",
            "agent": descriptor, "integrity_passed": integrity, "unique_scheduled_missions": unique,
            "manifest_sha256": request["manifest_sha256"], "collection_role": manifest["request"]["role"],
            "cohort": description["cohort"],
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
    policies = parser.add_mutually_exclusive_group(required=True)
    policies.add_argument("--checkpoint", type=Path)
    policies.add_argument("--test-policy", action="store_true")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--native-receipt", required=True, type=Path)
    parser.add_argument("--count", type=int)
    parser.add_argument("--design", choices=["v1", "v2", "v3"], default="v1")
    parser.add_argument("--freeze-receipt", type=Path)
    parser.add_argument("--request", type=Path)
    parser.add_argument("--defer-independent-verification", action="store_true",
                        help="Run verify.py once after capture; do not also replay inline")
    args = parser.parse_args()
    campaign(args.collection, args.checkpoint, args.output, args.native_receipt, args.count,
             design=args.design, freeze_receipt=args.freeze_receipt, test_policy=args.test_policy,
             request_file=args.request, verify_saved=not args.defer_independent_verification)


if __name__ == "__main__":
    main()
