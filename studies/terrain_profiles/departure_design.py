"""Read-only departure geometry/kinematic study; never runs a native process.

The polynomial screen is a design heuristic, not plant propagation, an
admission certificate, or a reconstructed entry-state/landing claim.
"""

import json
import math

import fresh_validation as fresh
import survey


def sample(points, x):
    if not points[0][0] <= x <= points[-1][0]:
        raise ValueError("outside saved heightfield domain")
    for (a, ya), (b, yb) in zip(points, points[1:]):
        if a <= x <= b:
            return ya + (yb - ya) * ((x - a) / (b - a))
    raise ValueError("invalid heightfield")


def height_max(points, left, right):
    if right < left:
        raise ValueError("reversed corridor")
    return max([sample(points, left), sample(points, right)]
               + [y for x, y in points if left < x < right])


def travel_time(distance, vx, ax):
    if not all(math.isfinite(v) for v in (distance, vx, ax)) or vx <= 0 or ax < 0:
        raise ValueError("unsupported forward estimate")
    if distance <= 0:
        return 0.0
    if ax == 0:
        return distance / vx
    return 2 * distance / (math.sqrt(vx * vx + 2 * ax * distance) + vx)


def segment_reserve(points, state, ax, ay, duration, radius):
    """Minimum circle-envelope reserve on a closed-form constant-accel segment.

    Evaluate terrain under both horizontal circle extremes. The intervening
    terrain vertices are included by splitting their crossing times. For each
    swept terrain line, the residual is quadratic; endpoints and its interior
    minimum suffice. This deliberately overbounds the body, not the plant.
    """
    x, y, vx, vy = state
    if (not all(math.isfinite(v) for v in (*state, ax, ay, duration, radius))
            or duration < 0 or vx <= 0 or ax < 0 or radius < 0):
        raise ValueError("unsupported segment")
    finish = x + vx * duration + 0.5 * ax * duration**2
    if x - radius < points[0][0] or finish + radius > points[-1][0]:
        raise ValueError("screen corridor outside saved terrain")
    reserve = math.inf
    # A disk is inside this square. Require its whole swept square clear.
    for (a, ya), (b, yb) in zip(points, points[1:]):
        enter, leave = max(x, a - radius), min(finish, b + radius)
        if enter > leave:
            continue
        lo, hi = travel_time(enter - x, vx, ax), travel_time(leave - x, vx, ax)
        slope = (yb - ya) / (b - a)
        # Max of this terrain line on the square footprint is at the
        # slope-facing extreme, clipped to the terrain segment endpoints.
        offset = radius if slope >= 0 else -radius
        split = [lo, hi]
        for knot in (a - offset, b - offset):
            if enter < knot < leave:
                split.append(travel_time(knot - x, vx, ax))
        split.sort()
        for t0, t1 in zip(split, split[1:]):
            midpoint = (t0 + t1) / 2
            probe = x + vx * midpoint + 0.5 * ax * midpoint**2 + offset
            if probe <= a:
                q, linear, constant = 0.5 * ay, vy, y - radius - ya
            elif probe >= b:
                q, linear, constant = 0.5 * ay, vy, y - radius - yb
            else:
                q = 0.5 * (ay - slope * ax)
                linear = vy - slope * vx
                constant = y - radius - (ya + slope * (x + offset - a))
            times = [t0, t1]
            if q > 0 and t0 < -linear / (2 * q) < t1:
                times.append(-linear / (2 * q))
            reserve = min(reserve, *(q * t * t + linear * t + constant for t in times))
    if duration == 0:
        reserve = y - radius - height_max(points, x - radius, x + radius)
    end = (finish, y + vy * duration + 0.5 * ay * duration**2,
           vx + ax * duration, vy + ay * duration)
    return reserve, end


def forward_screen(points, state, progress, radius, acceleration, gravity, rotation_rate):
    """Cheap turn/advance/idle screen from an assumed settled upright state.

    Include angular-rate-limited turn drift with constant-thrust integrals.
    This continuous model is NOT a rigorous bound on the discrete plant.
    The query/ordinary replay, never this estimate, decides acceptance.
    """
    hold = 1 / 60
    angle, (x, y, vx, vy) = math.pi / 6, state
    ax, ay = acceleration * 0.5, acceleration * math.cos(angle) - gravity
    if (not all(math.isfinite(v) for v in (*state, progress, radius, acceleration, gravity, rotation_rate))
            or vy < 0 or ay <= 0 or rotation_rate <= 0 or gravity <= 0):
        raise ValueError("screen requires rising, supported upright input")
    turn = angle / rotation_rate
    at = (x + vx * turn + acceleration * (angle - math.sin(angle)) / rotation_rate**2,
          y + vy * turn + acceleration * (1 - math.cos(angle)) / rotation_rate**2 - 0.5 * gravity * turn**2,
          vx + acceleration * (1 - math.cos(angle)) / rotation_rate,
          vy + acceleration * math.sin(angle) / rotation_rate - gravity * turn)
    # Powered turn rises throughout; bound its terrain by a swept square.
    low = y - radius - height_max(points, x - radius, at[0] + radius)
    padding = math.ceil(turn / hold) * hold - turn
    padded, at = segment_reserve(points, at, ax, ay, padding, radius)
    advance = max(hold, math.ceil(travel_time(progress - at[0], at[2], ax) / hold) * hold)
    forward, at = segment_reserve(points, at, ax, ay, advance, radius)
    coast, end = segment_reserve(points, at, 0, -gravity, turn + padding + hold + 2, radius)
    return {"minimum_estimated_reserve_m": min(low, padded, forward, coast),
            "estimated_advance_s": turn + padding + advance, "estimated_certificate_x_m": end[0]}


def upright_boundary(row, gravity):
    """Undo the saved eight-second idle tail, not a new simulation or replay."""
    if row["stop_reason"] != "finite_trace_bound" or not row["row_id"].endswith("row_27"):
        return None
    stop = row["stop_state"]
    tail = (stop["physics_step"] - row["entry_physics_step"] - 960) / 120
    if tail != 8 or stop["held_command"]["throttle_frac"] != 0:
        raise ValueError("unexpected old upright trace tail")
    vx, vy = stop["velocity_mps"]["x"], stop["velocity_mps"]["y"] + gravity * tail
    return (stop["position_m"]["x"] - vx * tail,
            stop["position_m"]["y"] - vy * tail + 0.5 * gravity * tail * (tail + 1 / 120),
            vx, vy)


def delayed_screen(points, state, progress, radius, acceleration, gravity, rotation_rate):
    """Analytical extension of a saved stem, bounded by 16 s TOTAL power.

    This is not a newly queried or recorded state. Constant upright acceleration
    and the simplified turn/forward model are assumptions. Check every held
    command boundary; do not assume monotonicity or pick a per-world ratio.
    """
    for ticks in range(0, 961, 2):
        extra = ticks / 120
        reserve, at = segment_reserve(points, state, 0, acceleration - gravity, extra, radius)
        if reserve < 5:
            return None
        screen = forward_screen(points, at, progress, radius, acceleration, gravity, rotation_rate)
        total = 8 + extra + screen["estimated_advance_s"]
        if screen["minimum_estimated_reserve_m"] >= 5 and total <= 16:
            return dict(screen, extra_estimated_lift_s=extra, total_estimated_power_s=total)
    return None


def describe(case_id, flight, scenario):
    cycle, vehicle = flight["cycles"][0], scenario["vehicle"]
    points = [(p["x"], p["y"]) for p in scenario["world"]["terrain"]["points_m"]]
    geometry, gravity = vehicle["geometry"], scenario["world"]["gravity_mps2"]
    radius = max(math.hypot(geometry["hull_width_m"] / 2, geometry["hull_height_m"] / 2),
                 math.hypot(geometry["touchdown_half_span_m"], geometry["touchdown_base_offset_m"]))
    local, conflict = cycle["local_search"], cycle["conflict_state"]
    progress = local["row_diagnostics"][0]["minimum_progress_x_m"]
    source = next(p for p in scenario["world"]["landing_pads"] if p["id"] == "pad_source")
    maximum = height_max(points, source["center_x_m"] - radius, progress + radius)
    acceleration = 0.81 * (1 - 0.075) * vehicle["max_thrust_n"] / (vehicle["dry_mass_kg"] + vehicle["max_fuel_kg"])
    upright = []
    for row in local["row_diagnostics"]:
        if not row["physically_propagated"] or not row["row_id"].endswith("row_27"):
            continue
        boundary = upright_boundary(row, gravity)
        entry = {"row_id": row["row_id"], "stop_reason": row["stop_reason"],
                 "boundary_status_counts": row["boundary_status_counts"],
                 "derived_powered_boundary": boundary, "screen": None, "delayed_screen": None}
        if boundary is not None:
            entry["screen"] = forward_screen(points, boundary, progress, radius, acceleration,
                                              gravity, vehicle["max_rotation_rate_radps"])
            entry["delayed_screen"] = delayed_screen(points, boundary, progress, radius, acceleration,
                                                       gravity, vehicle["max_rotation_rate_radps"])
        upright.append(entry)
    return {"case_id": case_id, "conflict_x_m": conflict["position_m"]["x"],
            "conflict_time_s": conflict["sim_time_s"], "progress_x_m": progress,
            "local_height_above_source_m": maximum - source["surface_y_m"],
            "admitted_entries": sum(e["admitted"] for e in local["entries"]),
            "reached_continuation_checks": any(r["first_continuation_rejection"] is not None for r in local["row_diagnostics"]),
            "upright": upright}


def analyze():
    root, bound, _, _ = fresh.baseline(fresh.contract())
    report = json.loads(bound("early-exit-sweep.json"))
    # Freeze the structural cohort before any geometry/screen metrics: all
    # primary zero-correction NoClearing records whose first phase is source.
    cohort = []
    for row in report["rows"]:
        result = row["result"]
        if row["cohort"] != "random" or result["correction_count"] != 0 or result["planning_stop"] != "no_clearing":
            continue
        flight = json.loads(bound(f'runs/{row["case_id"]}/flight.json'))
        if not flight["integrity_passed"] or not flight["final_source_replay_passed"]:
            raise ValueError("unverified saved baseline")
        if flight["cycles"][0]["audit"]["clearance_scan"]["first_violation"]["phase"] != "source_bridge":
            continue
        scenario = json.loads(bound(f'scenarios/{row["case_id"]}.json'))
        cohort.append(describe(row["case_id"], flight, scenario))
    if len(cohort) != 46:
        raise ValueError("unexpected frozen structural cohort size")
    return {"schema": "pd-lab.departure-design-read-only.v1",
            "baseline_capture": str(root.relative_to(survey.REPO)),
            "baseline_receipt_sha256": fresh.contract()["baseline_receipt_sha256"],
            "evidence_kind": "saved geometry and polynomial heuristic, no native execution",
            "cohort": cohort}


if __name__ == "__main__":
    print(json.dumps(analyze(), indent=2, sort_keys=True))
