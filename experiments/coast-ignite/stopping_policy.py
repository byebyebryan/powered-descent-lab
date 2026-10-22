"""Frozen-policy stopping rules; no plant, training or witness access."""
from __future__ import annotations

import hashlib
import json
import math

import numpy as np
import torch

DOMAIN = "pd-coast-stopping-uniform-v1"


def uniform(key, physical_hash, replicate, tick):
    if replicate < 0 or tick < 0 or tick > 960 or tick % 12:
        raise ValueError("invalid stopping randomization coordinates")
    if len(physical_hash) != 64 or any(c not in "0123456789abcdef" for c in physical_hash):
        raise ValueError("require a canonical physical SHA-256")
    payload = json.dumps([DOMAIN, key, physical_hash, replicate, tick],
                         ensure_ascii=True, separators=(",", ":")).encode("ascii")
    # The upper 52 bits and midpoint give 2^52 equiprobable values strictly in (0,1).
    # Unlike a 53-bit midpoint, the largest value cannot round to 1 in binary64.
    bits = int.from_bytes(hashlib.sha256(payload).digest()[:8], "big") >> 12
    return (bits + 0.5) / 2**52


class Stopper:
    def __init__(self, rule, physical_hash, key, replicate=0):
        if rule not in ("G", "S", "M", "delay", "immediate"):
            raise ValueError("undeclared stopping rule")
        self.rule, self.physical_hash = rule, physical_hash
        self.key, self.replicate = key, replicate
        self.log_survival = 0.0
        self._log_terms = []
        self.trace = []
        self.closed = False

    def choose(self, model, features, tick):
        if self.closed or tick != 12 * len(self.trace) or tick > 960:
            raise ValueError("closed, skipped or duplicate stopping decision")
        obs = np.asarray(features, dtype=np.float32)
        hazard = None
        if model is not None:
            with torch.no_grad():
                hazard = float(model.policy.get_distribution(torch.as_tensor(obs[None])).distribution.probs[0, 1])
        greedy = None
        if self.rule == "G":
            greedy = int(model.predict(obs, deterministic=True)[0])
        return self.advance(hazard, tick, greedy)

    def advance(self, hazard, tick, greedy=None):
        if self.closed or tick != 12 * len(self.trace) or tick > 960:
            raise ValueError("closed, skipped or duplicate stopping decision")
        before = self.log_survival
        draw = None
        if self.rule in ("G", "M", "S"):
            if hazard is None or not math.isfinite(hazard) or not 0 <= hazard <= 1:
                raise ValueError("non-finite or invalid policy hazard")
            self._log_terms.append(-math.inf if hazard == 1 else math.log1p(-hazard))
            self.log_survival = math.fsum(self._log_terms)
            if self.rule == "G":
                if greedy not in (0, 1):
                    raise ValueError("G requires the original model prediction")
                action = greedy
            elif self.rule == "S":
                draw = uniform(self.key, self.physical_hash, self.replicate, tick)
                action = int(draw < hazard)
            else:
                action = int(self.log_survival <= math.log(0.5))
        else:
            action = int(self.rule == "immediate" or tick >= 360)
        survival = math.exp(self.log_survival)
        self.trace.append({"physics_tick": tick, "action": action, "hazard": hazard,
                           "survival_before": math.exp(before), "survival_after": survival,
                           "cumulative_stopping_probability": -math.expm1(self.log_survival),
                           "draw": draw})
        self.closed = bool(action or tick == 960)
        return action
