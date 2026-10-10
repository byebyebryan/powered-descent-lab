"""Bounded, reproducible PPO experiment; selection sees development inputs only."""
from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import time
from pathlib import Path

import numpy as np
import pd_native
import torch
from stable_baselines3 import PPO
from stable_baselines3.common.callbacks import BaseCallback
from stable_baselines3.common.vec_env import DummyVecEnv

from pd_env import CONFIGURATION, FAMILIES, ProceduralLandingEnv, native_module_paths


def write_new(path, value):
    with Path(path).open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def load_missions(root, role, per_family=None, usage_id=None):
    missions = []
    for family in FAMILIES:
        directory = Path(root)/f"{family}-{role}"
        manifest_bytes = (directory/"manifest.json").read_bytes()
        manifest = json.loads(manifest_bytes)
        entries = manifest["entries"] if per_family is None else manifest["entries"][:per_family]
        if usage_id:
            missions.extend(json.loads(pd_native.load_collection_missions(
                str(directory), [entry["index"] for entry in entries], usage_id,
                "tuning" if role != "held-out" else "evaluation")))
            continue
        for entry in entries:
            payload = (directory/entry["file"]).read_bytes()
            if hashlib.sha256(payload).hexdigest() != entry["sha256"]:
                raise RuntimeError("collection mission byte identity mismatch")
            mission = json.loads(payload)
            if mission["physical_sha256"] != entry["physical_sha256"]:
                raise RuntimeError("collection physical identity mismatch")
            missions.append(mission)
    return missions


def evaluate_summaries(model, missions, slots=16, env_class=ProceduralLandingEnv):
    """Exact scheduled input accounting; summaries carry no saved-command proof."""
    results, active = [], []
    cursor = 0

    def start(index):
        env = env_class(fixed_missions=[missions[index]])
        obs, _ = env.reset()
        return index, env, obs

    while cursor < len(missions) and len(active) < slots:
        active.append(start(cursor))
        cursor += 1
    while active:
        actions, _ = model.predict(np.stack([item[2] for item in active]), deterministic=True)
        remaining = []
        for (index, env, _), action in zip(active, actions):
            obs, reward, done, truncated, info = env.step(action)
            if done:
                results.append({"schedule_index": index, "physical_sha256": missions[index]["physical_sha256"],
                                "physical_outcome": info["physical_outcome"],
                                "mission_outcome": info["mission_outcome"],
                                "native_stop": info["native_stop"], "native_failure": info["native_failure"],
                                "target_distance_m": info["target_distance_m"],
                                "sim_time_s": info["sim_time_s"], "fuel_kg": float(env.raw[9])})
                if cursor < len(missions):
                    remaining.append(start(cursor))
                    cursor += 1
            else:
                remaining.append((index, env, obs))
        active = remaining
    results.sort(key=lambda item: item["schedule_index"])
    return {"scheduled": len(missions), "attempts": results,
            "native_summary_target_successes": sum(item["mission_outcome"] == "success" for item in results),
            "median_final_target_distance_m": float(np.median([item["target_distance_m"] for item in results])),
            "saved_command_proof": False}


class DevelopmentCallback(BaseCallback):
    def __init__(self, output, missions, every, max_seconds, env_class=ProceduralLandingEnv):
        super().__init__()
        self.output, self.missions = output, missions
        self.every, self.next_evaluation = every, every
        self.max_seconds = max_seconds
        self.started = time.monotonic()
        self.best_key, self.best = None, None
        self.evaluations = []
        self.env_class = env_class

    def _on_step(self):
        return time.monotonic()-self.started < self.max_seconds

    def evaluate(self, label):
        started = time.monotonic()
        checkpoint = self.output/f"checkpoint-{label}.zip"
        self.model.save(checkpoint)
        report = evaluate_summaries(self.model, self.missions, env_class=self.env_class)
        digest = hashlib.sha256(checkpoint.read_bytes()).hexdigest()
        report.update(checkpoint=str(checkpoint), checkpoint_sha256=digest,
                      transitions=self.num_timesteps, optimizer_epochs=self.model._n_updates,
                      elapsed_s=time.monotonic()-started)
        write_new(self.output/f"development-{label}.json", report)
        key = (report["native_summary_target_successes"], -report["median_final_target_distance_m"], -self.num_timesteps)
        if self.best_key is None or key > self.best_key:
            self.best_key, self.best = key, report
        self.evaluations.append({key: value for key, value in report.items() if key != "attempts"})
        print(json.dumps({"kind": "development", **self.evaluations[-1]}), flush=True)

    def _on_rollout_end(self):
        if self.num_timesteps >= self.next_evaluation:
            self.evaluate(f"{self.num_timesteps:010}")
            self.next_evaluation = self.num_timesteps+self.every


def source_identities(module=None):
    root = Path(__file__).resolve().parent
    paths = [root/"pd_env.py", root/"train.py", root/"pyproject.toml", root/"uv.lock", *native_module_paths()]
    if module is not None:
        paths.append(Path(module.__file__).resolve())
    return {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cohorts", type=Path, required=True)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--steps", type=int, default=8_000_000)
    parser.add_argument("--workers", type=int, default=16)
    parser.add_argument("--eval-every", type=int, default=524_288)
    parser.add_argument("--max-seconds", type=int, default=7200)
    parser.add_argument("--design", choices=["v1", "v2"], default="v1")
    args = parser.parse_args()
    module = importlib.import_module("pd_env" if args.design == "v1" else "pd_env_"+args.design)
    env_class, configuration = module.ProceduralLandingEnv, module.CONFIGURATION
    args.output.mkdir()
    torch.set_num_threads(1)
    before = source_identities(module)
    request = {"schema_version": 1, "seed": args.seed, "requested_transitions": args.steps,
               "workers": args.workers, "max_seconds": args.max_seconds,
               "design": args.design, "configuration": configuration, "source_identities": before,
               "optimizer": {"algorithm": "stable-baselines3.PPO", "learning_rate": 3e-4,
                             "n_steps": 512, "batch_size": 512, "n_epochs": 5,
                             "gamma": 0.999, "gae_lambda": 0.95, "clip_range": 0.2,
                             "ent_coef": 0.003, "net_arch": [128, 128], "log_std_init": -0.7},
               "selection": "native development summary success count, median endpoint distance, earlier checkpoint",
               "normalization": "fixed explicit physical scales; no running observation or reward normalization",
               "evaluation_every_transitions": args.eval_every}
    write_new(args.output/"request.json", request)
    env = DummyVecEnv([lambda i=i: env_class(worker_id=i, workers=args.workers,
                       log_path=args.output/f"worker-{i:02}.jsonl") for i in range(args.workers)])
    model = PPO("MlpPolicy", env, learning_rate=3e-4, n_steps=512, batch_size=512,
                n_epochs=5, gamma=0.999, gae_lambda=0.95, clip_range=0.2, ent_coef=0.003,
                policy_kwargs={"net_arch": [128, 128], "log_std_init": -0.7},
                seed=args.seed, device="cpu", verbose=0)
    missions = load_missions(args.cohorts, "development", 64,
                            usage_id="training-"+hashlib.sha256(str(args.output).encode()).hexdigest()[:24])
    callback = DevelopmentCallback(args.output, missions, args.eval_every, args.max_seconds, env_class)
    started = time.monotonic()
    model.learn(args.steps, callback=callback, progress_bar=False)
    callback.evaluate("final")
    after = source_identities(module)
    write_new(args.output/"selection.json", {
        "schema_version": 1, "requested_transitions": args.steps,
        "actual_transitions": model.num_timesteps, "elapsed_s": time.monotonic()-started,
        "source_identity_unchanged": before == after, "source_identities_after": after,
        "best": {key: value for key, value in callback.best.items() if key != "attempts"},
        "evaluations": callback.evaluations,
    })
    env.close()
    if before != after:
        raise RuntimeError("training source or loaded native binary changed during optimization")


if __name__ == "__main__":
    main()
