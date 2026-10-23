"""Input-defined arrival choices and complete native proof; no optimizer."""
from __future__ import annotations

import copy
import hashlib
import json
import math
import shutil
import time
from pathlib import Path

import pd_coast_native as native


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x") as stream:
        stream.write(encoded(value) + "\n")


CHOICES = ["direct"] + [f"p{p}-v{v}" for p in [35, 60] for v in [40, 70, 100]]


def scenario(anchor, index, angle, energy, miss, reflection, radius=800.0):
    """Same half-arc geometry convention; a new, prospectively fixed 90s task.

    Miss is the discrete ballistic landing x, not an outcome-derived label.
    Upright entry is fixed independently of any selected burn or goal.
    """
    s = copy.deepcopy(anchor)
    t = {0: 9, 30: 8.75, 60: 8.25, 80: 8}[angle] * {"low": 1.25, "mid": 1, "high": .75}[energy]
    n = 2 * round(t * 60)
    t = n / 120
    a = math.radians(angle)
    x, y = -radius * math.sin(a), 5 + radius * math.cos(a)
    vx = (-x + miss) / t
    vy = (5 - y + 9.81 * n * (n + 1) / (2 * 120**2)) / t
    s.update(id=f"arrival-{index:03}", name=f"Airborne {angle}° {energy} miss {miss:+g} reflection {reflection:+}",
             description="Input-only flat airborne arrival characterization", seed=231023 + index,
             tags=["airborne_arrival_v1", "flat", energy], metadata={"angle_deg": str(angle),
             "energy": energy, "ballistic_miss_m": str(miss), "reflection": str(reflection),
             "radius_m": str(radius), "nominal_ballistic_ticks": str(n)})
    s["sim"].update(max_time_s=90.0, sample_hz=120)
    s["mission"].pop("transfer_route", None)
    s["world"]["terrain"]["points_m"] = [{"x": -4000, "y": 0}, {"x": 4000, "y": 0}]
    s["initial_state"] = {"position_m": {"x": x * reflection, "y": y},
                          "velocity_mps": {"x": vx * reflection, "y": vy},
                          "attitude_rad": 0.0, "angular_rate_radps": 0.0}
    return s


def program(s, choice, land=True):
    if choice == "direct":
        return {"arrival": None, "land": land}
    p, v = choice.split("-")
    progress, scale = int(p[1:]) / 100, int(v[1:]) / 100
    pos, vel = s["initial_state"]["position_m"], s["initial_state"]["velocity_mps"]
    delta = (-pos["x"], 5 - pos["y"])
    length = math.hypot(*delta)
    tangent = {"x": delta[0] / length, "y": delta[1] / length}
    speed = max(12.0, math.hypot(vel["x"], vel["y"]) * scale)
    radius = 30.0
    goal = {"id": "optional-arrival-once", "position_m": {"x": pos["x"] + progress * delta[0],
            "y": pos["y"] + progress * delta[1]}, "handoff_tangent_unit": tangent,
            "capture_radius_m": radius, "max_cross_track_m": radius,
            "max_outbound_heading_error_rad": .25, "min_outbound_progress_mps": .85 * speed,
            "max_outbound_cross_speed_mps": .25 * speed,
            "min_speed_mps": .85 * speed, "max_speed_mps": 1.15 * speed,
            "min_vertical_speed_mps": tangent["y"] * speed - .15 * speed,
            "max_vertical_speed_mps": tangent["y"] * speed + .15 * speed}
    return {"arrival": {"waypoint": goal, "deadline_physics_step": 3600}, "land": land}


def compact(info):
    a, search, o = info["arrival"], info["landing_search"], info["observation"]
    return {"success": info["native_success"], "arrival_status": a["status"] if a else "direct",
            "arrival_tick": a["resolution"]["physics_step"] if a and a["resolution"] else None,
            "fuel_used_kg": 6300 - o["fuel_kg"], "stop_s": o["sim_time_s"],
            "steps": info["physics_steps"], "model_steps": info["model_steps"],
            "forecast_steps": info["forecast_steps"], "fingerprint": info["fingerprint"],
            "no_plan": search["no_plan"], "candidates": len(search["candidates"]),
            "tilt_saturated_updates": a["tilt_saturated_updates"] if a else 0,
            "arrival_updates": a["command_updates"] if a else 0,
            "replans": a["replans"] if a else 0}


class Budget:
    def __init__(self, root, allocation):
        self.root, self.a = Path(root), allocation
        self.counts = {"originals": 0, "reconstructions": 0, "native_steps": 0,
                       "model_steps": 0, "forecast_steps": 0, "neutral_replay_steps": 0}
        self.started = time.time()

    def total(self):
        return sum(self.counts[k] for k in ["native_steps", "model_steps", "forecast_steps", "neutral_replay_steps"])

    def remaining(self):
        return self.a["max_steps"] - self.total()

    def check(self):
        if time.time() >= self.a["deadline"] or self.total() > self.a["max_steps"]:
            raise RuntimeError("declared work/time limit")
        if shutil.disk_usage(self.root).free < 40 * 1024**3:
            raise RuntimeError("free-space reserve")
        size = sum(p.stat().st_size for p in self.root.rglob("*") if p.is_file())
        if size > self.a["max_bytes"]:
            raise RuntimeError("artifact allocation")
        return size

    def run(self, case, choice, captured=False, inputs=None, forced=None, category="originals", land=True):
        self.check()
        p = forced if forced is not None else program(case, choice, land)
        descriptor = {"schema_version": 1, "implementation": "airborne-arrival-composition-v1",
                      "checkpoint_sha256": None, "configuration": {"choice": choice, "program": p,
                      "native_library_sha256": sha(Path(native.__file__))}}
        ep = native.ArrivalEpisode(encoded(case), encoded(p), captured, encoded(descriptor), inputs or [])
        self.counts[category] += 1
        try:
            ep.execute(self.remaining())
        except BaseException:
            info = json.loads(ep.status_json())
            for key, dest in [("physics_steps", "native_steps"), ("model_steps", "model_steps"), ("forecast_steps", "forecast_steps")]:
                self.counts[dest] += info[key]
            raise
        return ep, p

    def consume(self, info):
        self.counts["native_steps"] += info["physics_steps"]
        self.counts["model_steps"] += info["model_steps"]
        self.counts["forecast_steps"] += info["forecast_steps"]
        self.check()

    def receipt(self):
        return {"counts": self.counts, "total_steps": self.total(), "seconds": time.time() - self.started,
                "logical_bytes": self.check(), "optimizer_steps": 0}


def execute_summary(budget, s, choice, land=True):
    ep, _ = budget.run(s, choice, land=land)
    result = json.loads(ep.finish_json())
    budget.consume(result["info"])
    return result


def capture(budget, s, path, choice, output, expected, allocation_path, land=True):
    Path(output).parent.mkdir(parents=True, exist_ok=True)
    inputs = [str(path), str(allocation_path), str(Path(native.__file__))]
    ep, p = budget.run(s, choice, True, inputs, land=land)
    status = json.loads(ep.status_json())
    if budget.remaining() < 3 * status["physics_steps"]:
        raise RuntimeError("insufficient proof reservation")
    budget.consume(status)
    # Reserve all three neutral passes before entering the save operation, so
    # a filesystem/proof fault cannot drop already executed work from counters.
    budget.counts["neutral_replay_steps"] += 3 * status["physics_steps"]
    saved = json.loads(ep.save_capture(str(output)))
    info = saved["info"]
    if compact(info) != compact(expected):
        raise RuntimeError("Summary/Captured differs")
    for forced in [p, None]:
        other, _ = budget.run(s, choice, forced=forced, category="reconstructions", land=land)
        rebuilt = json.loads(other.finish_json())["info"]
        budget.consume(rebuilt)
        if rebuilt != info:
            raise RuntimeError("controller/planner reconstruction differs")
    proof = {"physical_replay": True, "controller_reconstruction": True, "planner_reconstruction": True,
             "summary_equality": True, "planner_kind": "input_defined_goal_rule_v1",
             "capture_sha256": sha(Path(output) / "evaluation.json"),
             "decisions_sha256": sha(Path(output) / "decisions.json"),
             "verified_target_landing": saved["verified_target_landing"]}
    write_new(Path(output) / "proof.json", proof)
    return {"capture": str(output), "choice": choice, "case": s["id"], **proof}


def findings(rows):
    by = {}
    for r in rows:
        by.setdefault(r["case"], {})[r["choice"]] = r["result"]
    direct = sum(c["direct"]["success"] for c in by.values() if "direct" in c)
    reached = sum(r["result"]["arrival_status"] == "reached" for r in rows)
    arrivals = [r for r in rows if r["choice"] != "direct"]
    reached_landed = sum(r["result"]["arrival_status"] == "reached" and r["result"]["success"] for r in arrivals)
    best = {}
    recoveries, positive = [], []
    for key, choices in by.items():
        eligible = [(v["fuel_used_kg"], k) for k, v in choices.items() if v["success"] and (k == "direct" or v["arrival_status"] == "reached")]
        best[key] = min(eligible)[1] if eligible else None
        if best[key] and "direct" in choices:
            d = choices["direct"]
            if not d["success"]:
                recoveries.append(key)
            elif choices[best[key]]["fuel_used_kg"] < d["fuel_used_kg"] - 1e-6:
                positive.append({"case": key, "choice": best[key], "saving_kg": d["fuel_used_kg"] - choices[best[key]]["fuel_used_kg"]})
    return {"original_cases": len(by), "summary_occurrences": len(rows), "direct_native_successes": direct,
            "arrival_attempts": len(arrivals), "arrival_reached": reached,
            "arrival_reached_and_landed": reached_landed,
            "arrival_landed_without_fulfillment": sum(r["result"]["success"] and r["result"]["arrival_status"] != "reached" for r in arrivals),
            "best_tested": best, "recovered_cases": recoveries, "fuel_improvements": positive,
            "scope": "hindsight finite bank, not an online learned policy or reliability qualification"}
