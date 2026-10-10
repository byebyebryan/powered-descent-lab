"""Approach-owned Gymnasium features/actions/rewards over neutral native episodes."""
from __future__ import annotations

import json
import math
from pathlib import Path

import gymnasium as gym
import numpy as np
import pd_native

FAMILIES = ("value-noise", "layered")
TRAINING_SEEDS = (501101, 501201)
OBSERVATION_VERSION = "terrain-relative-v1"
ACTION_VERSION = "mirrored-thrust-angle-launch-v1"
REWARD_VERSION = "distance-progress-v1"
CONFIGURATION = {
    "observation": OBSERVATION_VERSION,
    "action": ACTION_VERSION,
    "reward": REWARD_VERSION,
    "action_repeat": 6,
    "protected_upright_launch_s": 0.75,
    "attitude_limit_rad": 0.8,
    "potential_discount": 1.0,
    "native_physics_hz": 120,
    "native_controller_hz": 60,
}


class ProceduralLandingEnv(gym.Env):
    """Complete source-rest missions. No snapshot resets or incumbent calls.

    Six native command boundaries share each policy action (10 Hz decisions,
    60 Hz native updates). Initial upright thrust is an explicit approach-owned
    launch primitive, including its time in the original fuel/horizon budget.
    """

    metadata = {"render_modes": []}

    def __init__(self, worker_id=0, workers=1, log_path=None, fixed_missions=None,
                 captured=False, agent_id="ppo-terrain-v1"):
        super().__init__()
        self.action_space = gym.spaces.Box(-1.0, 1.0, (2,), np.float32)
        self.observation_space = gym.spaces.Box(-20.0, 20.0, (64,), np.float32)
        self.worker_id, self.workers = worker_id, workers
        self.next_indices = [256 + worker_id, 256 + worker_id]
        self.family_cursor = worker_id % 2
        self.log_path = Path(log_path) if log_path else None
        self.fixed_missions = fixed_missions
        self.fixed_cursor = 0
        self.captured, self.agent_id = captured, agent_id
        self.episode = None
        self.steps = 0
        self.completed = 0

    def _mission(self):
        if self.fixed_missions is not None:
            mission = self.fixed_missions[self.fixed_cursor % len(self.fixed_missions)]
            self.fixed_cursor += 1
            return mission
        family = self.family_cursor
        self.family_cursor = 1 - self.family_cursor
        index = self.next_indices[family]
        self.next_indices[family] += self.workers
        result = json.loads(pd_native.generate_mission(FAMILIES[family], TRAINING_SEEDS[family], index))
        self._log({"kind": "proposal", "family": FAMILIES[family], "seed": TRAINING_SEEDS[family],
                   "index": index, "rejections": result["rejections"],
                   "physical_sha256": result["mission"]["physical_sha256"] if result["mission"] else None})
        if result["mission"] is None:
            raise RuntimeError("training generator exhausted a structurally declared request")
        return result["mission"]

    def _log(self, data):
        if self.log_path:
            with self.log_path.open("a") as stream:
                stream.write(json.dumps(data, separators=(",", ":")) + "\n")

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)
        self.mission = (options or {}).get("mission") or self._mission()
        self.episode = pd_native.NativeEpisode(json.dumps(self.mission["scenario"]),
                                               self.captured, self.agent_id)
        self.physical = json.loads(self.episode.physical_json())
        self.raw = np.asarray(self.episode.observation(), dtype=np.float64)
        self.direction = 1.0 if self.raw[11] >= 0 else -1.0
        self.initial_distance = abs(self.raw[11])
        self.source_y = self.raw[3]
        self.base_offset = self.physical["vehicle"]["geometry"]["touchdown_base_offset_m"]
        self.horizon = self.physical["sim"]["max_time_s"]
        self.initial_fuel = self.physical["vehicle"]["initial_fuel_kg"]
        self.potential = self._potential(self.raw)
        self.terminal = False
        self.reward_totals = {"potential": 0.0, "fuel": 0.0, "time": 0.0, "terminal": 0.0}
        return self._features(), {"physical_sha256": self.mission["physical_sha256"]}

    def _features(self):
        o = self.raw
        sign = self.direction
        thrust = self.physical["vehicle"]["max_thrust_n"]
        scalars = [o[11]*sign/1500, (o[12]-self.base_offset)/300,
                   o[4]*sign/80, o[5]/60, math.sin(o[6])*sign, math.cos(o[6]),
                   o[7]*sign/1.6, o[9]/self.initial_fuel, o[8]*o[10]/thrust,
                   (self.horizon-o[0])/self.horizon, o[15]/300, o[16]/100,
                   o[14]/40, (o[13]-(self.source_y-self.base_offset))/300,
                   (self.initial_distance-o[11]*sign)/1500, o[0]/self.horizon]
        local_offsets = np.linspace(-240.0, 480.0, 24)
        local_xs = o[2] + sign*local_offsets
        local_heights = np.asarray(self.episode.terrain_heights(local_xs.tolist()))
        route_xs = np.linspace(o[2], o[2]+o[11], 24)
        route_heights = np.asarray(self.episode.terrain_heights(route_xs.tolist()))
        values = np.concatenate((scalars, (local_heights-(o[3]-self.base_offset))/300,
                                 (route_heights-o[13])/300))
        return np.clip(values, -20.0, 20.0).astype(np.float32)

    def _potential(self, o):
        return -math.hypot(o[11]/1500, (o[12]-self.base_offset)/500)

    def step(self, action):
        if self.terminal:
            raise RuntimeError("reset explicitly before stepping a completed mission")
        action = np.asarray(action, dtype=np.float64)
        if action.shape != (2,) or not np.isfinite(action).all():
            raise ValueError("policy action must contain two finite values")
        throttle = float((np.clip(action[0], -1, 1)+1)/2)
        angle = float(np.clip(action[1], -1, 1)*0.8*self.direction)
        old = self.raw.copy()
        total_advanced = 0
        for _ in range(6):
            command = (1.0, 0.0) if self.raw[1] < 90 else (throttle, angle)
            raw, done, advanced, physical, mission = self.episode.step(*command)
            self.raw = np.asarray(raw, dtype=np.float64)
            total_advanced += advanced
            if done:
                break
        elapsed = self.raw[0]-old[0]
        discount = CONFIGURATION["potential_discount"] ** (elapsed/0.1)
        potential = self._potential(self.raw)
        shaping = 100.0*(discount*potential-self.potential)
        self.potential = potential
        fuel = -2.0*(old[9]-self.raw[9])/self.initial_fuel
        time = -0.01*elapsed
        physical, mission = json.loads(physical), json.loads(mission)
        terminal_reward = 0.0
        if done:
            terminal_reward = 100.0 if mission == "success" else (-20.0 if physical == "crashed" else -15.0)
        parts = {"potential": shaping, "fuel": fuel, "time": time, "terminal": terminal_reward}
        for name, value in parts.items():
            self.reward_totals[name] += value
        self.steps += 1
        self.terminal = done
        info = {"physical_outcome": physical, "mission_outcome": mission,
                "physics_steps_advanced": total_advanced, "sim_time_s": self.raw[0],
                "target_distance_m": abs(self.raw[11]), "reward_components": parts,
                "physical_sha256": self.mission["physical_sha256"]}
        if done:
            self.completed += 1
            info["reward_totals"] = self.reward_totals.copy()
        # The original scenario's finite horizon is an intrinsic task terminal.
        # There is no external collector timeout and no invented autoreset.
        features = self._features()
        if done and not self.captured:
            summary = json.loads(self.episode.finish_json())["summary"]
            info["native_stop"] = summary["stop"]
            info["native_failure"] = summary["failure"]
        if done:
            self._log({"kind": "completed", **info, "fuel_kg": self.raw[9],
                       "physics_step": int(self.raw[1])})
        return features, sum(parts.values()), done, False, info
