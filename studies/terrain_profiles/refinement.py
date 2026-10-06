"""One frozen ridge-slice experiment and offline pad compiler; never fly or publish."""

import argparse
import bisect
import copy
import html
import json
import math
from pathlib import Path
import subprocess
import sys

import study

HERE = Path(__file__).resolve().parent
VARIANTS = ("refined", "fastnoise")
SOURCES = study.SOURCES + ("refinement.py", "refinement_plan.json", "scenario_template.json")
STUDY_OUTPUTS = HERE.parents[1] / "outputs" / "terrain-profile-study"


def plans(path, base_path=None):
    plan = json.loads(path.read_bytes())
    base_path = base_path or HERE / "plan.json"
    base_data = base_path.read_bytes()
    if study.digest(base_data) != plan["control_plan_sha256"]:
        raise ValueError("changed control recipe")
    base = json.loads(base_data)
    study.validate_plan(base)
    if (plan["schema"] != "pd-lab.terrain-ridge-slice-study.v1"
            or plan["variants"] != list(VARIANTS) or plan["ridge_slice_offset"] != 0.37
            or plan["landmarks_m"] != [463, 1063] or plan["landmark_radius_m"] != 25
            or plan["flight_runs"] != 0):
        raise ValueError("outside the frozen single-change experiment")
    pad = plan["pad_preparation"]
    if [pad[k] for k in ("width_m", "transition_m", "source_center_x_m", "target_center_x_m")] != [36, 24, 0, 1200]:
        raise ValueError("changed fixed pad geometry")
    if (pad["source_pad_id"], pad["target_pad_id"], plan["scenario_template"]["path"]) != ("pad_source", "pad_main", "scenario_template.json"):
        raise ValueError("changed pad/template identity")
    if study.digest((HERE / "scenario_template.json").read_bytes()) != plan["scenario_template"]["sha256"]:
        raise ValueError("changed scenario template")
    return plan, base


class RidgeSlice:
    """Shift only ridge queries; reuse the original weighted composition verbatim."""

    def __init__(self, noise, offset):
        self.noise = noise
        self.offset = offset

    def noise2(self, x, y):
        return self.noise.noise2(x, y + self.offset)


def reference_grid(base):
    left, right = base["domain_m"]
    step = base["reference_spacing_m"]
    return [left + i * step for i in range(int((right - left) / step) + 1)]


def sample(plan, base, root, binary, offset=None, reverse=False):
    cls = study.load_reference(root, base)
    offset = plan["ridge_slice_offset"] if offset is None else offset
    xs, seeds = reference_grid(base), list(base["seeds"])
    if reverse:
        xs.reverse()
        seeds.reverse()
    profiles = []
    for seed in seeds:
        generator = study.make_refined(cls, seed, base["refined"], base["weighted_strength"])
        generator._ridge_noise = RidgeSlice(generator._ridge_noise, offset)
        profiles.append({"variant": "refined", "seed": seed, "points_m": [
            [x, generator(x + base["source_offset_m"])] for x in xs]})
    request = {"seeds": seeds, "xs": xs, "source_offset_m": base["source_offset_m"],
               "refined": base["refined"], "weighted_strength": base["weighted_strength"],
               "ridge_slice_offset": offset}
    result = subprocess.run([str(binary)], input=study.encoded(request), capture_output=True, check=True, timeout=60)
    external = json.loads(result.stdout)
    if len(external) != 6 or {p["seed"] for p in external} != set(seeds):
        raise ValueError("external seed inventory changed")
    for profile in external:
        if profile["variant"] != "fastnoise" or [p[0] for p in profile["points_m"]] != xs:
            raise ValueError("external coordinate/variant mismatch")
        features = profile.pop("features_m")
        generator = cls(profile["seed"], **base["refined"])
        expected = [generator._features(x + base["source_offset_m"]) for x in xs]
        if len(features) != len(xs) or any(not math.isfinite(b) or abs(a - b) > 1e-10 for a, b in zip(expected, features)):
            raise ValueError("sparse features changed between backends")
    profiles.extend(external)
    for profile in profiles:
        if len(profile["points_m"]) != len(xs) or any(not math.isfinite(y) for _, y in profile["points_m"]):
            raise ValueError("invalid profile")
        profile["points_m"].sort()
    return sorted(profiles, key=lambda p: (VARIANTS.index(p["variant"]), p["seed"]))


def name(profile):
    return f'{profile["variant"]}-seed-{profile["seed"]}.json'


def read_profiles(folder, base, offset=None):
    profiles = []
    for variant in VARIANTS:
        for seed in sorted(base["seeds"]):
            value = json.loads((folder / f"{variant}-seed-{seed}.json").read_bytes())
            fine = value["reference_points_m"]
            if (value["variant"] != variant or value["seed"] != seed or value["profile_only"] is not True
                    or [p[0] for p in fine] != reference_grid(base)
                    or any(not math.isfinite(y) for _, y in fine)
                    or value["points_m"] != study.subset(fine, base["spacing_m"])):
                raise ValueError("profile identity/grid mismatch")
            if offset is not None and value.get("ridge_slice_offset") != offset:
                raise ValueError("profile ridge slice mismatch")
            profiles.append({"variant": variant, "seed": seed, "points_m": fine})
    return profiles


def peaks(profile, base):
    points = study.core(profile["points_m"], base)
    return [x for i, (x, y) in enumerate(points[1:-1], 1)
            if y > points[i - 1][1] and y >= points[i + 1][1]]


def summary(profiles, controls, plan, base, plan_data):
    pairs = []
    for before, after in zip(controls, profiles):
        if (before["variant"], before["seed"]) != (after["variant"], after["seed"]):
            raise ValueError("mispaired comparison")
        pairs.append({"variant": after["variant"], "seed": after["seed"],
                      "before": study.describe(before, base), "after": study.describe(after, base),
                      "peaks_before_m": peaks(before, base), "peaks_after_m": peaks(after, base)})
    alignment = []
    for variant in VARIANTS:
        group = [p for p in pairs if p["variant"] == variant]
        for landmark in plan["landmarks_m"]:
            alignment.append({"variant": variant, "landmark_m": landmark, "denominator": 6,
                              **{side: sum(any(abs(x - landmark) <= plan["landmark_radius_m"] for x in p[f"peaks_{side}_m"]) for p in group)
                                 for side in ("before", "after")}})
    return {"schema": plan["schema"], "profile_only": True, "flight_runs": 0,
            "plan_sha256": study.digest(plan_data), "profile_count": 12, "control_count": 12,
            "ridge_slice_offset": plan["ridge_slice_offset"], "repeat_passed": True,
            "repeat_sha256": study.digest(study.encoded(profiles)), "zero_offset_matches_controls": True,
            "compatibility_sha256": study.digest(study.encoded(controls)),
            "landmark_alignment": alignment, "pairs": pairs}


def comparison_chart(before, after, base, extent, width=600, height=260):
    old = dict(before, variant="baseline")
    # Keep the endpoint label within the nested SVG's clipping rectangle.
    endpoint = f'x="{width - 12:.2f}" y="{height - 15}" text-anchor="middle">{base["review_span_m"][1]:.0f}</text>'
    return (study.chart([old, after], base, extent, width, height)
            .replace(endpoint, endpoint.replace('text-anchor="middle"', 'text-anchor="end"'))
            .replace("Pylander baseline", "Original refined control")
            .replace("Pylander refined", "Pylander ridge slice 67.37")
            .replace("FastNoiseLite / Perlin", "FastNoiseLite ridge slice 67.37"))


def sheet(profiles, controls, base):
    extent = study.shared_extent(profiles + controls, base)
    lookup = {(p["variant"], p["seed"]): (b, p) for b, p in zip(controls, profiles)}
    parts = ['<svg xmlns="http://www.w3.org/2000/svg" width="1100" height="1740" viewBox="0 0 1100 1740"><style>text{font:13px sans-serif;fill:#243345}</style><rect width="100%" height="100%" fill="white"/><text x="15" y="25">Ridge-only change: 67.0 to 67.37; gray = original, green/orange = new</text><text x="15" y="48">All 12 pairs; one shared vertical scale; display subtracts h(0); no flights.</text>']
    for row, seed in enumerate(base["seeds"]):
        for column, variant in enumerate(VARIANTS):
            x, y = column * 550 + 10, row * 275 + 70
            before, after = lookup[variant, seed]
            parts.append(f'<text x="{x}" y="{y}">{study.LABELS[variant]} — seed {seed}</text>')
            parts.append(comparison_chart(before, after, base, extent, 525, 250)
                         .replace('<svg ', f'<svg x="{x}" y="{y+8}" width="525" height="250" ', 1))
    return ("".join(parts) + "</svg>").encode()


def gallery(profiles, controls, stats, base):
    extent = study.shared_extent(profiles + controls, base)
    lookup = {(p["variant"], p["seed"]): (b, p) for b, p in zip(controls, profiles)}
    parts = ['<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Ridge-slice refinement — no flights</title><style>body{font:16px system-ui;color:#243345;max-width:1350px;margin:24px auto;padding:0 20px;background:#f5f7fa}article,.notice{background:white;border:1px solid #d5dce5;border-radius:8px;padding:16px;margin:12px 0}.grid{display:grid;grid-template-columns:1fr 1fr;gap:12px}svg{width:100%;height:auto}svg text{font:12px system-ui}nav{display:flex;gap:16px;flex-wrap:wrap}table{border-collapse:collapse;width:100%}td,th{padding:8px;text-align:right;border-bottom:1px solid #d5dce5}td:first-child,th:first-child{text-align:left}.table{overflow-x:auto}a{color:#175eab}@media(max-width:800px){.grid{grid-template-columns:1fr}}</style><h1>Ridge-slice refinement</h1><div class="notice"><strong>12 new profiles against 12 frozen controls. No flights or source promotion.</strong><p>Only ridge-noise y changes, from 67.0 to 67.37. Macro, ordinary noise, warp, features and physical scales stay fixed. Gray is the original control; green/orange is the new profile. Every plot shares one vertical scale, with display-only subtraction of h(0).</p><p>Peak counts near 463/1063 m are diagnostics, not terrain-quality or planner gates. Sampling warnings remain descriptive; the resolved 4 m line is the proposed authoritative game terrain. The 1 m reference is not another physics surface.</p><nav><a href="#metrics">Metrics</a><a href="plan.json">Frozen refinement plan</a><a href="summary.json">All metrics/peak positions</a><a href="overview.svg">Review sheet</a><a href="receipt.json">Receipt</a>']
    parts.extend(f'<a href="#seed-{seed}">Seed {seed}</a>' for seed in base["seeds"])
    parts.append('</nav></div>')
    for seed in base["seeds"]:
        parts.append(f'<section id="seed-{seed}"><h2>Seed {seed}</h2><div class="grid">')
        for variant in VARIANTS:
            before, after = lookup[variant, seed]
            filename = name(after)
            parts.append(f'<article><h3>{study.LABELS[variant]}</h3>{comparison_chart(before, after, base, extent)}<a href="profiles/{filename}">New raw profile</a> · <a href="controls/{filename}">Frozen control</a></article>')
        parts.append('</div></section>')
    parts.append('<h2 id="metrics">All pairs retained</h2><div class="table"><table><tr><th>Backend / seed</th><th>Relief before → after (m)</th><th>Rise before → after (m)</th><th>Max slope before → after</th><th>4 m error before → after (m)</th></tr>')
    for p in stats["pairs"]:
        before, after = p["before"], p["after"]
        parts.append(f'<tr><td>{p["variant"]} / {p["seed"]}</td><td>{before["relief_m"]:.1f} → {after["relief_m"]:.1f}</td><td>{before["endpoint_rise_m"]:+.1f} → {after["endpoint_rise_m"]:+.1f}</td><td>{before["max_abs_slope"]:.3f} → {after["max_abs_slope"]:.3f}</td><td>{before["sampling"]["4"]["max_m"]:.3f} → {after["sampling"]["4"]["max_m"]:.3f}</td></tr>')
    parts.append('</table></div><h2>Landmark alignment (±25 m)</h2><ul>')
    parts.extend(f'<li>{a["variant"]}, {a["landmark_m"]} m: {a["before"]}/6 → {a["after"]}/6 profiles with a local peak</li>' for a in stats["landmark_alignment"])
    parts.append('</ul><p>Plan identity: <code>' + html.escape(stats["plan_sha256"]) + '</code>. Offline shape review, outside the accepted report site. Pad-prepared scenarios are a separate input-only capture.</p></html>')
    return "".join(parts).encode()


def reserve(output):
    if not output.resolve().is_relative_to(STUDY_OUTPUTS):
        raise ValueError("output must remain inside the isolated terrain study")
    output.mkdir(parents=True, exist_ok=False)


def inventory(output, receipt, required):
    actual = {str(p.relative_to(output)) for p in output.rglob("*") if p.is_file()}
    if actual != required or set(receipt["files"]) | {"receipt.json"} != required:
        raise ValueError("artifact inventory mismatch")
    for relative, expected in receipt["files"].items():
        path = output / relative
        if not path.resolve().is_relative_to(output.resolve()) or study.digest(path.read_bytes()) != expected:
            raise ValueError(f"artifact hash mismatch: {relative}")


def write_receipt(output, extra):
    receipt = dict(extra, files={str(p.relative_to(output)): study.digest(p.read_bytes())
                               for p in sorted(output.rglob("*")) if p.is_file()})
    study.write_new(output / "receipt.json", study.encoded(receipt))


def check_controls(output, plan, base):
    receipt_data = (output / "control_receipt.json").read_bytes()
    if study.digest(receipt_data) != plan["control_receipt_sha256"]:
        raise ValueError("changed independently bound control receipt")
    receipt = json.loads(receipt_data)
    for variant in VARIANTS:
        for seed in base["seeds"]:
            filename = f"{variant}-seed-{seed}.json"
            if study.digest((output / "controls" / filename).read_bytes()) != receipt["files"]["profiles/" + filename]:
                raise ValueError("changed frozen control")
    return read_profiles(output / "controls", base)


def capture(args):
    plan_data = args.plan.read_bytes()
    plan, base = plans(args.plan)
    control = args.control_capture.resolve()
    receipt_data = (control / "receipt.json").read_bytes()
    if study.digest(receipt_data) != plan["control_receipt_sha256"]:
        raise ValueError("unbound control capture")
    # Authenticate every archived input before executing its historical verifier.
    for relative, expected in json.loads(receipt_data)["files"].items():
        path = control / relative
        if not path.resolve().is_relative_to(control) or study.digest(path.read_bytes()) != expected:
            raise ValueError("control archive was changed")
    subprocess.run([sys.executable, "-B", str(control / "inputs/study/study.py"), "verify", str(control),
                    "--plan", str(HERE / "plan.json")], check=True, capture_output=True, timeout=60)
    root, binary = args.pylander_root.resolve(), args.fastnoise_bin.resolve()
    study.load_reference(root, base)
    source_bytes = {relative: (HERE / relative).read_bytes() for relative in SOURCES}
    binary_hash = study.digest(binary.read_bytes())
    reserve(args.output)
    study.write_new(args.output / "plan.json", plan_data)
    study.write_new(args.output / "control_receipt.json", receipt_data)
    for relative, data in source_bytes.items():
        study.write_new(args.output / "inputs/study" / relative, data)
    for relative in base["pylander"]["files"]:
        study.write_new(args.output / "inputs/pylander" / relative, (root / relative).read_bytes())
    for variant in VARIANTS:
        for seed in base["seeds"]:
            filename = f"{variant}-seed-{seed}.json"
            study.write_new(args.output / "controls" / filename, (control / "profiles" / filename).read_bytes())
    controls = check_controls(args.output, plan, base)
    if study.encoded(sample(plan, base, root, binary, offset=0.0)) != study.encoded(controls):
        raise ValueError("zero-offset compatibility failed; stop before new profiles")
    profiles = sample(plan, base, root, binary)
    repeated = subprocess.run([sys.executable, "-B", str(HERE / "refinement.py"), "sample",
                               "--plan", str(args.output / "plan.json"), "--pylander-root", str(args.output / "inputs/pylander"),
                               "--fastnoise-bin", str(binary), "--reverse"], capture_output=True, check=True, timeout=60)
    if study.encoded(profiles) != study.encoded(json.loads(repeated.stdout)):
        raise ValueError("independent reversed-order repeat failed")
    if (args.plan.read_bytes() != plan_data or binary_hash != study.digest(binary.read_bytes())
            or any(data != (HERE / relative).read_bytes() for relative, data in source_bytes.items())):
        raise ValueError("generation inputs changed during capture")
    study.load_reference(root, base)
    stats = summary(profiles, controls, plan, base, plan_data)
    for profile in profiles:
        study.write_new(args.output / "profiles" / name(profile), study.encoded({
            "profile_only": True, "variant": profile["variant"], "seed": profile["seed"],
            "ridge_slice_offset": plan["ridge_slice_offset"],
            "points_m": study.subset(profile["points_m"], base["spacing_m"]),
            "reference_points_m": profile["points_m"]}))
    study.write_new(args.output / "summary.json", study.encoded(stats))
    study.write_new(args.output / "overview.svg", sheet(profiles, controls, base))
    study.write_new(args.output / "index.html", gallery(profiles, controls, stats, base))
    write_receipt(args.output, {"schema": plan["schema"], "flight_runs": 0, "profile_only": True,
                               "python": sys.version, "fastnoise_binary_sha256": binary_hash,
                               "source_sha256": {relative: study.digest(data) for relative, data in source_bytes.items()}})
    verify(args.output, args.plan)
    print(json.dumps({"capture": str(args.output.resolve()), "new_profiles": 12, "zero_offset_matches_controls": True, "repeat_passed": True}))


def verify(output, expected_plan):
    plan_data = (output / "plan.json").read_bytes()
    if plan_data != expected_plan.read_bytes():
        raise ValueError("capture does not match independent refinement plan")
    plan, base = plans(expected_plan)
    receipt = json.loads((output / "receipt.json").read_bytes())
    if (receipt["schema"] != plan["schema"] or receipt["flight_runs"] != 0 or receipt["profile_only"] is not True
            or set(receipt["source_sha256"]) != set(SOURCES)):
        raise ValueError("source inventory/evidence boundary mismatch")
    required = {"plan.json", "control_receipt.json", "summary.json", "overview.svg", "index.html", "receipt.json"}
    required.update("inputs/study/" + relative for relative in SOURCES)
    required.update("inputs/pylander/" + relative for relative in base["pylander"]["files"])
    required.update(f"{folder}/{variant}-seed-{seed}.json" for folder in ("profiles", "controls") for variant in VARIANTS for seed in base["seeds"])
    inventory(output, receipt, required)
    for relative, expected in receipt["source_sha256"].items():
        if study.digest((HERE / relative).read_bytes()) != expected or study.digest((output / "inputs/study" / relative).read_bytes()) != expected:
            raise ValueError("generation source differs; use the archived verifier or a new capture")
    study.load_reference(output / "inputs/pylander", base)
    controls = check_controls(output, plan, base)
    profiles = read_profiles(output / "profiles", base, plan["ridge_slice_offset"])
    stats = summary(profiles, controls, plan, base, plan_data)
    if ((output / "summary.json").read_bytes() != study.encoded(stats)
            or (output / "overview.svg").read_bytes() != sheet(profiles, controls, base)
            or (output / "index.html").read_bytes() != gallery(profiles, controls, stats, base)):
        raise ValueError("derived comparison differs from raw data")
    return plan, base, profiles


def height_at(points, x):
    if not math.isfinite(x) or not points[0][0] <= x <= points[-1][0]:
        raise ValueError("terrain query outside resolved domain")
    index = max(0, min(bisect.bisect_right([p[0] for p in points], x) - 1, len(points) - 2))
    x0, y0 = points[index]
    x1, y1 = points[index + 1]
    if x == x0:
        return y0
    if x == x1:
        return y1
    return y0 + (y1 - y0) * (x - x0) / (x1 - x0)


def prepare_points(raw, pad):
    patches = []
    xs = {x for x, _ in raw}
    for role in ("source", "target"):
        center = pad[f"{role}_center_x_m"]
        half = pad["width_m"] / 2
        left, right = center - half, center + half
        outer_left, outer_right = left - pad["transition_m"], right + pad["transition_m"]
        anchors = [height_at(raw, outer_left), height_at(raw, outer_right)]
        surface = height_at(raw, center)
        xs.update((outer_left, left, right, outer_right))
        patches.append({"pad_id": pad[f"{role}_pad_id"], "center_x_m": center, "surface_y_m": surface,
                        "width_m": pad["width_m"], "shelf_interval_m": [left, right],
                        "edited_interval_m": [outer_left, outer_right], "outer_heights_m": anchors})
    if patches[0]["edited_interval_m"][1] >= patches[1]["edited_interval_m"][0]:
        raise ValueError("pad patches overlap")
    resolved = []
    for x in sorted(xs):
        y = height_at(raw, x)
        for patch in patches:
            outer_left, outer_right = patch["edited_interval_m"]
            left, right = patch["shelf_interval_m"]
            surface = patch["surface_y_m"]
            if left <= x <= right:
                y = surface
            elif outer_left < x < left:
                anchor = patch["outer_heights_m"][0]
                y = anchor + (surface - anchor) * (x - outer_left) / (left - outer_left)
            elif right < x < outer_right:
                anchor = patch["outer_heights_m"][1]
                y = surface + (anchor - surface) * (x - right) / (outer_right - right)
        resolved.append([x, y])
    for patch in patches:
        left, right = patch["edited_interval_m"]
        patch["maximum_vertex_change_m"] = max(abs(y - height_at(raw, x)) for x, y in resolved if left <= x <= right)
    resolved_by_x = dict(resolved)
    if any(resolved_by_x[x] != y for x, y in raw
           if not any(p["edited_interval_m"][0] < x < p["edited_interval_m"][1] for p in patches)):
        raise ValueError("pad preparation changed a vertex outside its local patches")
    return resolved, patches


def scenario(profile, plan, base, template, plan_data):
    raw = study.subset(profile["points_m"], base["spacing_m"])
    points, patches = prepare_points(raw, plan["pad_preparation"])
    value = copy.deepcopy(template)
    identifier = f'terrain_ridge_slice_{profile["variant"]}_{profile["seed"]}'
    value.update(id=identifier, name=identifier, seed=profile["seed"],
                 description="Fixed procedural profile and local pads; input-only, not flown.",
                 tags=["procedural_terrain", "ridge_slice_study", "input_only"],
                 metadata={"study": plan["schema"], "primitive": profile["variant"],
                           "profile_plan_sha256": study.digest(plan_data), "flight_status": "not_flown"})
    value["world"]["terrain"] = {"kind": "heightfield", "points_m": [{"x": x, "y": y} for x, y in points]}
    value["world"]["landing_pads"] = [{key: p[key] for key in ("pad_id", "center_x_m", "surface_y_m", "width_m")} for p in patches]
    for p in value["world"]["landing_pads"]:
        p["id"] = p.pop("pad_id")
    source = patches[0]
    value["initial_state"]["position_m"] = {"x": source["center_x_m"], "y": source["surface_y_m"] + value["vehicle"]["geometry"]["touchdown_base_offset_m"]}
    return value, {"variant": profile["variant"], "seed": profile["seed"], "patches": patches,
                   "terrain_point_count": len(points), "raw_point_count": len(raw), "outside_patches_unchanged": True}


def checked_preflight(binary, path, pad):
    result = subprocess.run([str(binary), "waypoint-v2-flight", str(path), "--source-pad", pad["source_pad_id"],
                             "--target-pad", pad["target_pad_id"], "--preflight-only"], capture_output=True, check=True, timeout=60)
    value = json.loads(result.stdout)
    validate_preflight(value)
    return value


def validate_preflight(value):
    required = {"schema_id", "status", "supported", "input_identity", "policy", "planning_stop", "reason",
                "correction_count", "integrity_passed", "final_source_replay_passed", "physical_outcome", "mission_outcome", "output_dir"}
    if (not required.issubset(value) or value.get("schema_id") != "planner_v2_cli_flight_v1"
            or value.get("policy") != {"policy_id": "piecewise_local_clearing_v2_policy_3", "maximum_corrections": 6}
            or value.get("status") != "preflight_only" or value.get("supported") is not True
            or value.get("correction_count") != 0 or not isinstance(value.get("input_identity"), str)
            or not value["input_identity"].startswith("fnv1a64:")
            or any(value.get(key) is not None for key in ("planning_stop", "reason", "physical_outcome", "mission_outcome", "integrity_passed", "final_source_replay_passed", "output_dir", "error"))):
        raise ValueError("native output is not a supported, flight-free preflight")


def prepare(args):
    plan, base, profiles = verify(args.capture, args.plan)
    plan_data = args.plan.read_bytes()
    source_receipt = (args.capture / "receipt.json").read_bytes()
    template = json.loads((HERE / "scenario_template.json").read_bytes())
    binary = args.preflight_bin.resolve()
    binary_hash = study.digest(binary.read_bytes())
    reserve(args.output)
    study.write_new(args.output / "plan.json", plan_data)
    study.write_new(args.output / "source_receipt.json", source_receipt)
    rows = []
    for profile in profiles:
        filename = name(profile)
        study.write_new(args.output / "raw" / filename, (args.capture / "profiles" / filename).read_bytes())
        value, stats = scenario(profile, plan, base, template, plan_data)
        path = args.output / "scenarios" / filename
        study.write_new(path, study.encoded(value))
        study.write_new(args.output / "preflight" / filename, study.encoded(checked_preflight(binary, path, plan["pad_preparation"])))
        rows.append(stats)
    if binary_hash != study.digest(binary.read_bytes()):
        raise ValueError("native preflight binary changed during validation")
    verify(args.capture, args.plan)
    stats = {"schema": "pd-lab.terrain-prepared-inputs.v1", "flight_runs": 0, "scenario_count": 12,
             "native_preflight_supported_count": 12, "plan_sha256": study.digest(plan_data),
             "source_receipt_sha256": study.digest(source_receipt), "profiles": rows}
    study.write_new(args.output / "summary.json", study.encoded(stats))
    write_receipt(args.output, {"schema": stats["schema"], "flight_runs": 0, "native_preflight_binary_sha256": binary_hash})
    verify_prepared(args.output, args.capture, args.plan)
    print(json.dumps({"prepared_inputs": str(args.output.resolve()), "supported_preflights": 12, "flight_runs": 0}))


def verify_prepared(output, capture_root, expected_plan):
    plan, base, profiles = verify(capture_root, expected_plan)
    plan_data = expected_plan.read_bytes()
    source_receipt = (capture_root / "receipt.json").read_bytes()
    if (output / "plan.json").read_bytes() != plan_data or (output / "source_receipt.json").read_bytes() != source_receipt:
        raise ValueError("prepared inputs have an unbound source")
    receipt = json.loads((output / "receipt.json").read_bytes())
    if receipt["schema"] != "pd-lab.terrain-prepared-inputs.v1" or receipt["flight_runs"] != 0:
        raise ValueError("prepared input evidence boundary mismatch")
    required = {"plan.json", "source_receipt.json", "summary.json", "receipt.json"}
    required.update(f"{folder}/{name(p)}" for folder in ("raw", "scenarios", "preflight") for p in profiles)
    inventory(output, receipt, required)
    template = json.loads((HERE / "scenario_template.json").read_bytes())
    rows = []
    for profile in profiles:
        filename = name(profile)
        if (output / "raw" / filename).read_bytes() != (capture_root / "profiles" / filename).read_bytes():
            raise ValueError("raw input changed")
        value, stats = scenario(profile, plan, base, template, plan_data)
        if (output / "scenarios" / filename).read_bytes() != study.encoded(value):
            raise ValueError("scenario differs from fixed pad compiler")
        validate_preflight(json.loads((output / "preflight" / filename).read_bytes()))
        rows.append(stats)
    stats = {"schema": "pd-lab.terrain-prepared-inputs.v1", "flight_runs": 0, "scenario_count": 12,
             "native_preflight_supported_count": 12, "plan_sha256": study.digest(plan_data),
             "source_receipt_sha256": study.digest(source_receipt), "profiles": rows}
    if (output / "summary.json").read_bytes() != study.encoded(stats):
        raise ValueError("prepared summary differs from source geometry")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ("sample", "capture", "verify", "prepare", "verify-prepared"):
        cmd = sub.add_parser(command)
        cmd.add_argument("--plan", type=Path, default=HERE / "refinement_plan.json")
        if command in ("sample", "capture"):
            cmd.add_argument("--pylander-root", type=Path, required=True)
            cmd.add_argument("--fastnoise-bin", type=Path, required=True)
        if command == "sample":
            cmd.add_argument("--reverse", action="store_true")
        if command == "capture":
            cmd.add_argument("--control-capture", type=Path, required=True)
        if command in ("capture", "prepare"):
            cmd.add_argument("--output", type=Path, required=True)
        if command in ("verify", "verify-prepared"):
            cmd.add_argument("output", type=Path)
        if command in ("prepare", "verify-prepared"):
            cmd.add_argument("--capture", type=Path, required=True)
        if command == "prepare":
            cmd.add_argument("--preflight-bin", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "sample":
        plan, base = plans(args.plan)
        print(study.encoded(sample(plan, base, args.pylander_root, args.fastnoise_bin.resolve(), reverse=args.reverse)).decode(), end="")
    elif args.command == "capture":
        capture(args)
    elif args.command == "verify":
        verify(args.output, args.plan)
        print("Verified 12 new profiles, 12 independently bound controls, grids and derived comparison (no generation).")
    elif args.command == "prepare":
        prepare(args)
    else:
        verify_prepared(args.output, args.capture, args.plan)
        print("Verified 12 fixed-pad scenarios and saved native preflights (no simulator).")


if __name__ == "__main__":
    main()
