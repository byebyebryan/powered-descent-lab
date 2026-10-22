"""One native episode, one stopping decision, fixed landing execution."""
from __future__ import annotations

import hashlib
import json
import math
import time
from pathlib import Path

import gymnasium as gym
import numpy as np
import torch
from stable_baselines3 import PPO
from stable_baselines3.common.policies import ActorCriticPolicy
import pd_coast_native as native

NATIVE_ARTIFACT = (Path(native.__file__) if Path(native.__file__).suffix == ".so" else
                   next(Path(native.__file__).parent.glob("pd_coast_native*.so")))


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def write_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x") as stream:
        stream.write(encoded(value) + "\n")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class HazardPolicy(ActorCriticPolicy):
    """A learned residual over a clock-only uniform stopping-time prior."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        torch.nn.init.zeros_(self.action_net.weight)
        torch.nn.init.zeros_(self.action_net.bias)

    def distribution_at(self, obs, latent):
        residual = self.action_net(latent)
        slots = torch.round(obs[:, -1] * 81).clamp(min=1, max=81)
        ignite = residual[:, 1] - residual[:, 0] - torch.log(slots)
        logits = torch.stack((torch.zeros_like(ignite), ignite), dim=1)
        return self.action_dist.proba_distribution(action_logits=logits)

    def forward(self, obs, deterministic=False):
        pi, vf = self.mlp_extractor(self.extract_features(obs))
        dist = self.distribution_at(obs, pi)
        actions = dist.get_actions(deterministic=deterministic)
        return actions, self.value_net(vf), dist.log_prob(actions)

    def evaluate_actions(self, obs, actions):
        pi, vf = self.mlp_extractor(self.extract_features(obs))
        dist = self.distribution_at(obs, pi)
        return self.value_net(vf), dist.log_prob(actions), dist.entropy()

    def get_distribution(self, obs):
        latent = self.mlp_extractor.forward_actor(self.extract_features(obs))
        return self.distribution_at(obs, latent)


def model_for(env, seed):
    return PPO(HazardPolicy, env, seed=seed, device="cpu", n_steps=512,
               batch_size=128, n_epochs=4, learning_rate=3e-4, gamma=1.0,
               gae_lambda=1.0, ent_coef=0.0, clip_range=0.2, vf_coef=0.5,
               max_grad_norm=0.5, target_kl=0.02,
               policy_kwargs={"net_arch": {"pi": [64, 64], "vf": [64, 64]},
                              "activation_fn": torch.nn.Tanh}, verbose=0)


class ArcSampler:
    """Retain all input-only proposals; policy receives only the minimal task."""

    def __init__(self, root, anchor, seed, cohort, budget=None, seen=None, requested_arcs=1):
        self.root = Path(root)
        self.root.mkdir()
        self.anchor = encoded(anchor)
        self.seed, self.cohort = seed, cohort
        self.index = self.accepted = 0
        self.max_arcs, self.max_proposals = requested_arcs, 16 * requested_arcs
        self.budget = budget
        self.seen = seen if seen is not None else set()
        self.log = (self.root / "proposals.jsonl").open("x")
        write_new(self.root / "declaration.json", {"seed": seed, "cohort": cohort,
                  "max_requested_arcs": requested_arcs, "max_proposals": self.max_proposals})

    def next(self):
        # Bound the complete requested collection, retaining leading exclusions.
        if self.accepted >= self.max_arcs:
            raise RuntimeError("declared arc stream exhausted")
        while self.index < self.max_proposals:
            p = json.loads(native.generate_proposal(self.anchor, self.seed, self.cohort, self.index))
            self.index += 1
            self.log.write(encoded(p) + "\n")
            self.log.flush()
            if self.budget:
                self.budget.count("pure_grid_queries", p["audit"]["pure_grid_queries"])
                self.budget.count("pure_reference_constructions", 1)
                self.budget.check()
            if not p["admitted"]:
                continue
            identity = p["physical_sha256"]
            if identity in self.seen:
                raise RuntimeError("physical arc overlap across streams")
            self.seen.add(identity)
            task = {"scenario": p["scenario"], "reference_end": p["reference_end"],
                    "physical_sha256": identity}
            path = self.root / f"arc-{self.accepted}.json"
            write_new(path, task)
            write_new(self.root / f"arc-{self.accepted}-audit.json", p["audit"])
            self.accepted += 1
            return task, path
        raise RuntimeError(f"input preparation underfilled: {self.cohort}, {self.accepted}, {self.index}")


class CoastEnv(gym.Env):
    observation_space = gym.spaces.Box(-np.inf, np.inf, shape=(10,), dtype=np.float32)
    action_space = gym.spaces.Discrete(2)

    def __init__(self, sampler, budget, records):
        super().__init__()
        self.sampler, self.budget, self.records = sampler, budget, records
        self.episode = None
        self.previous_steps = self.previous_checks = self.previous_constructions = 0

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)
        self.task, self.task_path = self.sampler.next()
        self.episode = native.CoastEpisode(encoded(self.task["scenario"]), self.task["reference_end"])
        self.previous_steps = self.previous_checks = self.previous_constructions = 0
        self.budget.count("original_episodes", 1)
        return np.asarray(self.episode.observation(), dtype=np.float32), {}

    def step(self, action):
        self.episode.set_work_limit(self.budget.remaining_steps())
        try:
            obs, reward, done, raw = self.episode.step(int(action))
        except BaseException:
            partial = json.loads(self.episode.status_json())
            self.budget.count("original_native_steps", partial["physics_steps"] - self.previous_steps)
            self.records.write(encoded({"task": str(self.task_path), "incomplete_prefix": partial}) + "\n")
            self.records.flush()
            raise
        info = json.loads(raw)
        self.budget.count("original_native_steps", info["physics_steps"] - self.previous_steps)
        self.budget.count("original_command_updates", (info["physics_steps"] + 1) // 2 - (self.previous_steps + 1) // 2)
        self.previous_steps = info["physics_steps"]
        checks = (info["feedback"] or {}).get("admission_actuation_checks", 0)
        self.budget.count("admission_checks", checks - self.previous_checks)
        self.previous_checks = checks
        constructions = (info["feedback"] or {}).get("constructions", 0)
        self.budget.count("reference_constructions", constructions - self.previous_constructions)
        self.previous_constructions = constructions
        self.budget.count("policy_decisions", 1)
        self.budget.check()
        if done:
            result = json.loads(self.episode.finish_json())
            self.records.write(encoded({"task": str(self.task_path), **result}) + "\n")
            self.records.flush()
        return np.asarray(obs, dtype=np.float32), reward, done, False, info


def decide(rule, episode, model=None, rng=None):
    obs = episode.observation()
    physical = json.loads(episode.physical_json())
    tick = physical["physics_step"]
    kind = rule["kind"]
    if kind == "model":
        return int(model.predict(np.asarray(obs, dtype=np.float32), deterministic=True)[0])
    if kind == "immediate":
        return 1
    if kind == "never":
        return 0
    if kind == "delay":
        return int(tick >= round(rule["seconds"] * 120))
    if kind == "prior":
        return int(rng.random() < 1 / (82 - tick // 12))
    if kind == "ttc":
        h = max(0, min(physical["touchdown_clearance_m"], physical["min_hull_clearance_m"]))
        vy = physical["velocity_mps"]["y"]
        ttc = (vy + math.sqrt(vy * vy + 2 * 9.81 * h)) / 9.81
        return int(ttc <= rule["seconds"])
    raise ValueError(f"unknown policy rule {kind}")


def run_episode(task, rule, budget, model=None, captured=False, inputs=(), descriptor=None,
                schedule=None, rng=None, category="original"):
    ep = native.CoastEpisode(encoded(task["scenario"]), task["reference_end"], captured,
                             encoded(descriptor) if descriptor else None,
                             [str(Path(p).resolve()) for p in inputs])
    previous_steps = previous_checks = previous_constructions = 0
    budget.count(category + "_episodes", 1)
    while True:
        index = len(json.loads(ep.status_json())["decisions"])
        action = schedule[index][1] if schedule is not None else decide(rule, ep, model, rng)
        ep.set_work_limit(budget.remaining_steps())
        try:
            _, _, done, raw = ep.step(action)
        except BaseException:
            partial = json.loads(ep.status_json())
            budget.count(category + "_native_steps", partial["physics_steps"] - previous_steps)
            write_new(budget.root / f"incomplete-{time.time_ns()}.json", partial)
            raise
        info = json.loads(raw)
        budget.count(category + "_native_steps", info["physics_steps"] - previous_steps)
        budget.count(category + "_command_updates", (info["physics_steps"] + 1) // 2 - (previous_steps + 1) // 2)
        previous_steps = info["physics_steps"]
        checks = (info["feedback"] or {}).get("admission_actuation_checks", 0)
        budget.count("admission_checks", checks - previous_checks)
        previous_checks = checks
        constructions = (info["feedback"] or {}).get("constructions", 0)
        budget.count("reference_constructions", constructions - previous_constructions)
        previous_constructions = constructions
        budget.count(category + "_policy_decisions", 1)
        budget.check()
        if done:
            return ep, info


def capture_verified(task, task_path, rule, output, budget, model=None, model_path=None):
    """Physical replay, recorded-decision control, and live model inference are distinct."""
    inputs = [task_path, native.__file__, NATIVE_ARTIFACT]
    if model_path:
        inputs.append(model_path)
    descriptor = {"schema_version": 1, "implementation": "coast-ignite-v1",
                  "checkpoint_sha256": sha(model_path) if model_path else None,
                  "configuration": {"rule": rule, "reference_end": task["reference_end"],
                                    "native_library_sha256": sha(NATIVE_ARTIFACT),
                                    "decision_hz": 10, "hazard": "uniform-82-v1",
                                    "controller_backend": "polynomial_landing_v2",
                                    "coast_policy": "engine_off_hold_initial_attitude",
                                    "hard_deadline_physics_step": 4800,
                                    "command_coverage_end_physics_step": task["reference_end"] + 4,
                                    "feature_schema": "coast-10-fixed-v1"}}
    ep, original = run_episode(task, rule, budget, model, True, inputs, descriptor)
    if budget.remaining_steps() < 3 * original["physics_steps"]:
        raise RuntimeError("insufficient reserved native steps for all neutral replays")
    original = json.loads(ep.save_capture(str(output)))
    # Capture fingerprint, assess_execution and save_evaluation each replay once.
    budget.count("neutral_replay_passes", original["neutral_replay_passes"])
    budget.count("neutral_replay_native_steps", original["neutral_replay_passes"] * original["native_replay_steps"])
    final_info = original["info"]
    for category, schedule in (("controller_reconstruction", final_info["decisions"]),
                               ("planner_reconstruction", None)):
        reconstruction_model = (PPO.load(model_path, device="cpu")
                                if model_path and schedule is None else None)
        replay, _ = run_episode(task, rule, budget, reconstruction_model, schedule=schedule, category=category)
        result = json.loads(replay.finish_json())["info"]
        if result != final_info:
            raise RuntimeError(category + " differs from live execution")
    proof = {"physical_replay": True, "controller_reconstruction": True,
             "planner_reconstruction": True, "capture_sha256": sha(Path(output) / "evaluation.json"),
             "decisions_sha256": sha(Path(output) / "decisions.json"),
             "verified_target_landing": original["verified_target_landing"],
             "native_steps_each_pass": final_info["physics_steps"]}
    write_new(Path(output) / "proof.json", proof)
    budget.check()
    return {"task": str(task_path), "rule": rule, "capture": str(output), **proof,
            "native_success": final_info["native_success"],
            "physics_steps": final_info["physics_steps"],
            "ignition_tick": next((t for t, a in final_info["decisions"] if a == 1), None)}
