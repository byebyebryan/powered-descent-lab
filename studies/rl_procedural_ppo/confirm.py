"""Broad summary confirmations for already frozen policies; never re-ranks them."""
from __future__ import annotations

import argparse
import hashlib
import importlib
import json
from pathlib import Path

import pd_native
import torch
from stable_baselines3 import PPO

from pd_env import FAMILIES, native_module_paths
from train import evaluate_summaries, write_new


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--freeze", required=True, type=Path)
    parser.add_argument("--cohorts", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    frozen = json.loads(args.freeze.read_text())
    module = importlib.import_module("pd_env" if frozen["design"] == "v1" else "pd_env_"+frozen["design"])
    torch.set_num_threads(1)
    args.output.mkdir()
    sources = [str(Path(__file__).resolve()), str(Path(module.__file__).resolve()),
               str(Path(__file__).with_name("train.py").resolve()),
               str(Path(__file__).with_name("pd_env.py").resolve())]
    if frozen["design"] == "v3":
        sources.append(str(Path(__file__).with_name("pd_env_v2.py").resolve()))
    inputs = [str(args.freeze.resolve()), *map(str, native_module_paths())]
    missions_by_role = {}
    for role in ["development", "validation"]:
        missions = []
        for family in FAMILIES:
            root = args.cohorts/f"{family}-{role}"
            manifest = json.loads((root/"manifest.json").read_text())
            usage = "confirmation-"+hashlib.sha256(str(args.output).encode()).hexdigest()[:20]+"-"+role
            missions.extend(json.loads(pd_native.load_collection_missions(
                str(root), [entry["index"] for entry in manifest["entries"]], usage, "evaluation")))
            inputs.extend(str((root/name).resolve()) for name in ["manifest.json", "request.json"])
            inputs.extend(str((root/entry["file"]).resolve()) for entry in manifest["entries"])
        missions_by_role[role] = missions
    inputs.extend(item["checkpoint"] for item in frozen["selected"])
    before = pd_native.evidence_snapshot(sources, inputs)
    reports = []
    for selected in frozen["selected"]:
        checkpoint = Path(selected["checkpoint"])
        if hashlib.sha256(checkpoint.read_bytes()).hexdigest() != selected["checkpoint_sha256"]:
            raise RuntimeError("frozen checkpoint changed before confirmation")
        model = PPO.load(checkpoint, device="cpu")
        model.policy.set_training_mode(False)
        for role, missions in missions_by_role.items():
            report = evaluate_summaries(model, missions, env_class=module.ProceduralLandingEnv)
            report.update(seed=selected["seed"], design=frozen["design"], role=role,
                          checkpoint_sha256=selected["checkpoint_sha256"], used_for_selection=False)
            write_new(args.output/f'seed-{selected["seed"]}-{role}.json', report)
            reports.append({key: value for key, value in report.items() if key != "attempts"})
            print(json.dumps(reports[-1]), flush=True)
    after = pd_native.evidence_snapshot(sources, inputs)
    write_new(args.output/"summary.json", {"schema_version": 1, "reports": reports,
              "evidence_before": json.loads(before), "evidence_after": json.loads(after),
              "integrity_passed": json.loads(before) == json.loads(after), "saved_command_proof": False})
    if json.loads(before) != json.loads(after):
        raise RuntimeError("confirmation source, input or checkpoint changed")


if __name__ == "__main__":
    main()
