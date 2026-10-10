"""Read-only mathematical screen of existing waypoint fits, never flight evidence.

Reconstruct the terrain-blind finite constructor and validate its least-effort
choice against a saved correction. No terrain audit, plant rollout or file writes.
"""
import argparse
import json
import math
from pathlib import Path

import ballistic_mechanics_review as review
import study

SUBJECTS = [44, 50, 62, 81, 715]


def project(p, v, gravity, dt, ticks):
    return ((p[0] + v[0] * ticks * dt,
             p[1] + v[1] * ticks * dt - gravity * dt * dt * ticks * (ticks + 1) * .5),
            (v[0], v[1] - gravity * ticks * dt))


def room(distance, vx, available, gravity, rotation, dt):
    if distance < 0 or vx < 0 or available <= gravity:
        return None
    braking = math.sqrt((available - gravity) * (available + gravity))
    turn = math.ceil(math.acos(gravity / available) / rotation / (2 * dt)) * 2 * dt
    return distance - vx * turn - vx * vx / (2 * braking)


def fits(state, goal, scenario):
    vehicle = scenario["vehicle"]
    p = (state["position_m"]["x"], state["position_m"]["y"])
    v = (state["velocity_mps"]["x"], state["velocity_mps"]["y"])
    gravity = scenario["world"]["gravity_mps2"]
    dt = 1 / scenario["sim"]["physics_hz"]
    mass = vehicle["dry_mass_kg"] + state["fuel_kg"]
    available = .925 * vehicle["max_thrust_n"] / mass
    rotation = vehicle["max_rotation_rate_radps"]
    dx = goal["x"] - p[0]
    if dx <= 0 or v[0] <= 0 or state["fuel_kg"] <= 0:
        raise ValueError("screen requires a fueled forward waypoint state")
    steps = [max(2, math.ceil(dx / (v[0] * 2 * dt)) * 2)]
    base = max(2, math.ceil(math.sqrt(2 * dx / gravity) / (2 * dt)) * 2)
    for trial in range(32):
        ticks = base + trial * 60
        if ticks not in steps:
            steps.append(ticks)
        if len(steps) == 32:
            break
    target = next(pad["center_x_m"] for pad in scenario["world"]["landing_pads"] if pad["id"] == "pad_main")
    result = []
    for ticks in steps:
        if state["physics_step"] + ticks > math.ceil(scenario["sim"]["max_time_s"] / dt):
            continue
        natural, _ = project(p, v, gravity, dt, ticks)
        miss = (goal["x"] - natural[0], goal["y"] - natural[1])
        turn = 0
        for _ in range(3):
            remaining = (ticks - turn) * dt + dt * .5
            disc = remaining * remaining - 2 * math.hypot(*miss) / available
            if disc < 0:
                break
            burn = max(2, math.ceil((remaining - math.sqrt(disc)) / (2 * dt)) * 2)
            if turn + burn + 24 >= ticks:
                break
            gain = dt * dt * burn * (ticks - turn - (burn - 1) * .5)
            acceleration = (miss[0] / gain, miss[1] / gain)
            length = math.hypot(*acceleration)
            if acceleration[1] < -1e-8 or length > available + 1e-8:
                break
            angle = math.atan2(acceleration[0], acceleration[1])
            delta = abs((angle - state["attitude_rad"] + math.pi) % (2 * math.pi) - math.pi)
            needed = math.ceil(math.ceil(delta / (rotation * dt)) / 2) * 2
            if needed != turn:
                turn = needed
                continue
            cutoff_p, cutoff_v = project(p, v, gravity, dt, turn + burn)
            cutoff_p = tuple(cutoff_p[i] + acceleration[i] * dt * dt * burn * (burn + 1) * .5 for i in (0, 1))
            cutoff_v = tuple(cutoff_v[i] + acceleration[i] * burn * dt for i in (0, 1))
            if cutoff_v[0] <= 0:
                break
            fuel_tick = vehicle["max_fuel_burn_kgps"] * dt
            def mean(throttle):
                return (vehicle["max_thrust_n"] * throttle / (mass - fuel_tick * throttle)
                        + vehicle["max_thrust_n"] * throttle / (mass - 2 * fuel_tick * throttle)) * .5
            if length < mean(vehicle["min_throttle_frac"]) or length > mean(1):
                break
            arrival_p, arrival_v = project(cutoff_p, cutoff_v, gravity, dt, ticks - turn - burn)
            result.append({"ticks": ticks, "turn": turn, "burn": burn, "acceleration": acceleration,
                           "effort": length * burn * dt, "vx": arrival_v[0],
                           "speed": math.hypot(*arrival_v), "arrival_position": arrival_p,
                           "room": room(max(0, target - goal["x"]), arrival_v[0], available, gravity, rotation, dt)})
            break
    return sorted(result, key=lambda fit: (fit["effort"], fit["speed"], fit["ticks"]))


def screen(root, case):
    run = root / f"runs/random-{case:03}"
    feedback = json.loads((run / "feedback.json").read_bytes())
    scenario_bytes = (run / "scenario.json").read_bytes()
    if (not all(feedback[k] for k in ("integrity_passed", "source_replay_passed", "decisions_reproduced"))
            or study.digest(scenario_bytes) != feedback["scenario_sha256"]):
        raise ValueError("unproved or mismatched saved input")
    first = next(i for i, r in enumerate(feedback["refreshes"]) if r["decision"] == "early_destination_obstruction")
    record = next(r for r in feedback["refreshes"][first + 1:] if r["decision"] in ("waypoint_replaced", "waypoint_reacquired"))
    candidates = fits(record["origin"], record["goal"]["position_m"], json.loads(scenario_bytes))
    chosen, correction = candidates[0], record["correction"]
    tick = record["origin"]["physics_step"]
    if ([chosen[k] for k in ("ticks", "turn", "burn")]
            != [correction["arrival_physics_step"] - tick, correction["turn_end_physics_step"] - tick,
                correction["burn_end_physics_step"] - correction["turn_end_physics_step"]]
            or math.hypot(chosen["acceleration"][0] - correction["thrust_acceleration_mps2"]["x"],
                          chosen["acceleration"][1] - correction["thrust_acceleration_mps2"]["y"]) >= 1e-9):
        raise ValueError("screen does not reproduce the recorded constructor choice")
    nonnegative = [fit for fit in candidates if fit["room"] is not None and fit["room"] >= 0]
    concise = lambda fit: None if fit is None else {k: fit[k] for k in ("ticks", "effort", "vx", "room")}
    return {"case": case, "evidence": "math_only_not_terrain_audited_or_flown", "fits": len(candidates),
            "chosen": concise(chosen), "nonnegative_fits": len(nonnegative),
            "first_nonnegative": concise(nonnegative[0] if nonnegative else None)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, default=review.root_for("piecewise-early-target"))
    args = parser.parse_args()
    for case in SUBJECTS:
        print(json.dumps(screen(args.capture, case)))
