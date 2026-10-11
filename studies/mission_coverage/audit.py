"""Read-only geometry audit of manifest-bound saved missions; no flights or imports
of controllers, planners, generation, game code, checkpoints or rewards.

JSON goes to stdout. Ordinary tests require no retained output collections.
This research tool is not the future native qualification classifier.
"""

import argparse
import bisect
import hashlib
import json
import math
from pathlib import Path
import statistics


REPO = Path(__file__).resolve().parents[2]
LEARNING = REPO / "outputs/procedural-learning/run-20261010T190900Z"
OLD = REPO / "outputs/eval/planner_v2_random_terrain"
DEFAULTS = [
    ("value-noise", LEARNING / "value-noise-held-out", "collection"),
    ("layered-perlin", LEARNING / "layered-held-out", "collection"),
    ("old-original", OLD / "capture-ballistic-acquisition-safety-acquisition-full-20261010-v1", "legacy"),
    ("old-fresh", OLD / "capture-ballistic-combined-validation-inputs-20261010-v1", "legacy"),
]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_bound_file(root, name, expected=None):
    path = root / name
    path.resolve().relative_to(root.resolve())
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 64 * 1024 * 1024:
        raise ValueError(f"not a bounded regular input: {path}")
    data = path.read_bytes()
    if expected is not None and digest(data) != expected:
        raise ValueError(f"saved input digest mismatch: {path}")
    return json.loads(data), digest(data)


def quantile(values, fraction):
    values = sorted(values)
    offset = (len(values) - 1) * fraction
    index = int(offset)
    return values[index] + (values[min(index + 1, len(values) - 1)] - values[index]) * (offset - index)


def weighted_quantile(values, fraction):
    threshold = sum(weight for _, weight in values) * fraction
    cumulative = 0.0
    for value, weight in sorted(values):
        cumulative += weight
        if cumulative >= threshold:
            return value
    return max(value for value, _ in values)


def longest_above(points, height):
    """Longest connected horizontal interval at/above height, exact for a polyline."""
    intervals = []
    for (ax, ay), (bx, by) in zip(points, points[1:]):
        if ay < height and by < height:
            continue
        left, right = ax, bx
        if ay < height:
            left = ax + (height - ay) * (bx - ax) / (by - ay)
        elif by < height:
            right = ax + (height - ay) * (bx - ax) / (by - ay)
        if intervals and math.isclose(left, intervals[-1][1], rel_tol=0, abs_tol=1e-7):
            intervals[-1][1] = right
        else:
            intervals.append([left, right])
    return max((b - a for a, b in intervals), default=0.0)


def measure(scenario):
    world = scenario["world"]
    if world["terrain"]["kind"] != "heightfield":
        raise ValueError("audit requires an authoritative heightfield")
    pads = world["landing_pads"]
    target_id = scenario["mission"]["goal"]["target_pad_id"]
    if len(pads) != 2 or sum(p["id"] == target_id for p in pads) != 1:
        raise ValueError("audit requires exactly one source and one target pad")
    target = next(p for p in pads if p["id"] == target_id)
    source = next(p for p in pads if p["id"] != target_id)
    if not math.isclose(source["center_x_m"], scenario["initial_state"]["position_m"]["x"], abs_tol=1e-7):
        raise ValueError("audit requires a source-pad start")
    a, b = sorted((source["center_x_m"], target["center_x_m"]))
    xs = [p["x"] for p in world["terrain"]["points_m"]]
    ys = [p["y"] for p in world["terrain"]["points_m"]]
    if (len(xs) < 2 or not all(math.isfinite(v) for v in xs + ys + [a, b])
            or any(x >= y for x, y in zip(xs, xs[1:])) or a >= b
            or a < xs[0] or b > xs[-1]):
        raise ValueError("terrain must be finite, ordered and contain the transfer")

    def sample(x):
        i = bisect.bisect_right(xs, x) - 1
        if i == len(xs) - 1:
            return ys[i]
        return ys[i] + (ys[i + 1] - ys[i]) * (x - xs[i]) / (xs[i + 1] - xs[i])

    for pad in pads:
        if not math.isclose(sample(pad["center_x_m"]), pad["surface_y_m"], rel_tol=0, abs_tol=1e-7):
            raise ValueError("pad elevation disagrees with prepared physical terrain")
    points = [(a, sample(a))] + [(x, y) for x, y in zip(xs, ys) if a < x < b] + [(b, sample(b))]
    rise = target["surface_y_m"] - source["surface_y_m"]
    chord_rise = points[-1][1] - points[0][1]
    chord_relative = [(x, y - points[0][1] - chord_rise * (x - a) / (b - a)) for x, y in points]
    edges = [p["center_x_m"] + sign * p["width_m"] / 2 for p in pads for sign in (-1, 1)]
    slopes = []
    ordinary = []
    segments = []
    for (ax, ay), (bx, by) in zip(points, points[1:]):
        slope = abs((by - ay) / (bx - ax))
        touches_edge = any(math.isclose(x, e, rel_tol=0, abs_tol=1e-7) for x in (ax, bx) for e in edges)
        slopes.append((slope, bx - ax))
        segments.append((slope, touches_edge))
        if not touches_edge:
            ordinary.append(slope)
    barrier = max(y for _, y in chord_relative)
    maximum_slope = max(slope for slope, _ in slopes)
    return {
        "distance_m": b - a,
        "target_width_m": target["width_m"],
        "relief_m": max(y for _, y in points) - min(y for _, y in points),
        "signed_pad_rise_m": rise,
        "abs_pad_rise_m": abs(rise),
        "barrier_m": barrier,
        "barrier_distance_ratio": barrier / (b - a),
        "p95_slope": weighted_quantile(slopes, 0.95),
        "max_slope": maximum_slope,
        "max_slope_without_pad_edges": max(ordinary, default=0.0),
        # A linear pad transition can tie several segments. Classify the entire
        # maximum set rather than whichever segment comes first after reflection.
        "max_slope_at_pad_edge": maximum_slope > 0 and any(
            edge and math.isclose(slope, maximum_slope, rel_tol=1e-8, abs_tol=1e-9)
            for slope, edge in segments),
        **{f"longest_barrier_{height}m_width_m": longest_above(chord_relative, height)
           for height in (50, 100, 200)},
    }


def saved_rows(root, kind):
    manifest, manifest_sha = read_bound_file(root, "manifest.json")
    source = {"path": str(root.relative_to(REPO)) if root.is_relative_to(REPO) else str(root),
              "manifest_sha256": manifest_sha, "kind": kind}
    if kind == "collection":
        entries = manifest["entries"]
        source["role"] = manifest["request"]["role"]
        records = [(e["index"], e["file"], e["sha256"], e) for e in entries]
    else:
        entries = [r for r in manifest["rows"] if r["cohort"] == "random"]
        records = [(r["case_id"], r["scenario_path"], r["scenario_sha256"], r) for r in entries]
    if not records or len({r[0] for r in records}) != len(records):
        raise ValueError("empty or repeated saved mission schedule")
    rows = []
    bound = []
    for index, filename, sha, entry in records:
        data, actual_sha = read_bound_file(root, filename, sha)
        scenario = data["scenario"] if kind == "collection" else data
        if kind == "collection" and data["physical_sha256"] != entry["physical_sha256"]:
            raise ValueError("collection physical identity declaration disagrees")
        group = str(data["terrain_stratum"]) if kind == "collection" else entry["geometry"]["recipe_id"]
        envelope = {"sim": scenario["sim"], "vehicle": scenario["vehicle"], "gravity": scenario["world"]["gravity_mps2"]}
        rows.append({"index": index, "group": group, "envelope": envelope, **measure(scenario)})
        bound.append((filename, actual_sha))
    # This checks input file bytes, not Rust's native physical-identity/replay proof.
    source["bound_inputs_sha256"] = digest(json.dumps(bound, separators=(",", ":")).encode())
    return rows, source


def summarize(rows):
    metrics = [k for k, v in rows[0].items()
               if k != "index" and isinstance(v, (int, float)) and not isinstance(v, bool)]
    result = {"n": len(rows)}
    for key in metrics:
        values = [r[key] for r in rows]
        result[key] = {"p50": round(statistics.median(values), 3), "p90": round(quantile(values, .9), 3)}
    result["max_slope_at_pad_edge_count"] = sum(r["max_slope_at_pad_edge"] for r in rows)
    result["barrier_over_200m_count"] = sum(r["barrier_m"] > 200 for r in rows)
    result["sustained_barrier_80m_count"] = {
        str(height): sum(r[f"longest_barrier_{height}m_width_m"] >= 80 for r in rows)
        for height in (50, 100, 200)
    }
    return result


def audit(inputs):
    if len({name for name, _, _ in inputs}) != len(inputs):
        raise ValueError("population labels must be unique")
    populations = {}
    all_envelopes = set()
    for name, root, kind in inputs:
        rows, source = saved_rows(root, kind)
        all_envelopes.update(json.dumps(r["envelope"], sort_keys=True) for r in rows)
        populations[name] = {"source": source, "total": summarize(rows),
                             "groups": {g: summarize([r for r in rows if r["group"] == g])
                                        for g in sorted({r["group"] for r in rows})}}
        near_1200 = [r for r in rows if 1000 <= r["distance_m"] <= 1400]
        if near_1200:
            populations[name]["distance_1000_1400m"] = summarize(near_1200)
    return {"schema": "pd-lab.saved-mission-geometry-review.v1",
            "metric_scope": "prepared source-to-target polyline; no morphology, solvability or native replay certification",
            "source_file_sha256": digest(Path(__file__).read_bytes()),
            "identical_vehicle_gravity_and_clocks": len(all_envelopes) == 1,
            "populations": populations}


def population(value, kind):
    name, sep, path = value.partition("=")
    if not sep or not name or not path:
        raise argparse.ArgumentTypeError("expected LABEL=COLLECTION_DIRECTORY")
    return name, Path(path).resolve(), kind


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collection", action="append", default=[], type=lambda v: population(v, "collection"))
    parser.add_argument("--legacy", action="append", default=[], type=lambda v: population(v, "legacy"))
    args = parser.parse_args()
    result = audit(args.collection + args.legacy or DEFAULTS)
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
