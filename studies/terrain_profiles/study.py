"""Explicit, create-only terrain study. No planner, game imports or publication."""

import argparse
import ast
import hashlib
import html
import inspect
import json
import math
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
VARIANTS = ("baseline", "refined", "fastnoise")
LABELS = {"baseline": "Pylander baseline", "refined": "Pylander refined", "fastnoise": "FastNoiseLite / Perlin"}
COLORS = {"baseline": "#576779", "refined": "#167b68", "fastnoise": "#b95517"}
SOURCES = ("study.py", "plan.json", "fastnoise/Cargo.toml", "fastnoise/Cargo.lock", "fastnoise/src/main.rs")


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_new(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as stream:
        stream.write(data)


def validate_plan(plan):
    if plan["schema"] != "pd-lab.terrain-profile-study.v1":
        raise ValueError("unknown plan schema")
    if len(plan["seeds"]) != 6 or len(set(plan["seeds"])) != 6:
        raise ValueError("exactly six distinct seeds are required")
    if any(type(s) is not int or not 0 <= s <= 2147483246 for s in plan["seeds"]):
        raise ValueError("seed outside supported i32 range")
    left, right = plan["domain_m"]
    start, end = plan["review_span_m"]
    if not left < start < end < right:
        raise ValueError("review span needs both terrain halos")
    for spacing in (plan["reference_spacing_m"], plan["spacing_m"], plan["coarser_spacing_m"]):
        if spacing <= 0 or any((v - left) % spacing for v in (right, start, end)):
            raise ValueError("grids must align with domain and review span")
        if spacing % plan["reference_spacing_m"]:
            raise ValueError("coarse grids must be reference-grid subsets")
    if not 0 <= plan["weighted_strength"] <= 1:
        raise ValueError("invalid weighting")
    if not math.isfinite(plan["source_offset_m"]):
        raise ValueError("invalid source origin")
    for recipe in (plan["baseline"], plan["refined"]):
        if not all(type(v) in (int, float) and math.isfinite(v) for v in recipe.values()):
            raise ValueError("non-finite recipe")
        if recipe["structure_octaves"] not in range(1, 9) or recipe["feature_cell_size"] < 200:
            raise ValueError("unsupported recipe bounds")
        if not 0 <= recipe["feature_density"] <= 1 or not 0 <= recipe["ridge_mix"] <= 1:
            raise ValueError("invalid recipe mixing")


def load_reference(root, plan):
    # Evaluate only two inspected, hash-bound class definitions. Importing the
    # full terrain module would pull pygame/pymunk and execute game setup.
    scope = {"math": math, "hashlib": hashlib}
    for relative, class_name in (("game/core/noise.py", "SimplexNoise"), ("game/core/terrain.py", "LayeredTerrainGenerator")):
        path = root / relative
        data = path.read_bytes()
        if digest(data) != plan["pylander"]["files"][relative]:
            raise ValueError(f"reference source hash mismatch: {relative}")
        classes = [n for n in ast.parse(data).body if isinstance(n, ast.ClassDef) and n.name == class_name]
        if len(classes) != 1:
            raise ValueError(f"missing unique reference class: {class_name}")
        exec(compile(ast.Module(body=classes, type_ignores=[]), str(path), "exec"), scope)
    cls = scope["LayeredTerrainGenerator"]
    defaults = {k: p.default for k, p in inspect.signature(cls).parameters.items() if k != "seed"}
    if defaults != plan["baseline"]:
        raise ValueError("baseline plan differs from actual Pylander defaults")
    return cls


def weighted_structure(generator, x, strength):
    xx = generator._warped_x(x)
    regular = ridged = norm = 0.0
    amp = regular_amp = ridge_amp = 1.0
    freq = generator.structure_frequency
    for _ in range(generator.structure_octaves):
        n = generator._structure_noise.noise2(xx * freq, 23.0)
        r = (1.0 - abs(generator._ridge_noise.noise2(xx * freq, 67.0))) ** 2
        regular += n * regular_amp
        ridged += (2.0 * r - 1.0) * ridge_amp
        norm += amp
        regular_amp *= generator.structure_persistence * (1.0 - strength + strength * min(1.0, max(0.0, (n + 1.0) * 0.5)))
        ridge_amp *= generator.structure_persistence * (1.0 - strength + strength * min(1.0, max(0.0, r)))
        amp *= generator.structure_persistence
        freq *= generator.structure_lacunarity
    return ((regular * (1.0 - generator.ridge_mix) + ridged * generator.ridge_mix) / norm * generator.structure_amplitude)


def make_refined(cls, seed, recipe, strength):
    class Refined(cls):
        def _structure(self, x):
            return weighted_structure(self, x, strength)
    return Refined(seed, **recipe)


def sample_batch(plan, root, binary, reverse=False):
    validate_plan(plan)
    cls = load_reference(root, plan)
    left, right = plan["domain_m"]
    step = plan["reference_spacing_m"]
    xs = [left + i * step for i in range(int((right - left) / step) + 1)]
    seeds = list(plan["seeds"])
    if reverse:
        xs.reverse()
        seeds.reverse()
    profiles = []
    for seed in seeds:
        for variant, generator in (("baseline", cls(seed)), ("refined", make_refined(cls, seed, plan["refined"], plan["weighted_strength"]))):
            profiles.append({"variant": variant, "seed": seed, "points_m": [[x, generator(x + plan["source_offset_m"])] for x in xs]})
    request = {"seeds": seeds, "xs": xs, "source_offset_m": plan["source_offset_m"], "refined": plan["refined"], "weighted_strength": plan["weighted_strength"]}
    result = subprocess.run([str(binary)], input=encoded(request), stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=60)
    external = json.loads(result.stdout)
    if len(external) != len(seeds) or {p["seed"] for p in external} != set(seeds):
        raise ValueError("external runner lost/duplicated a seed")
    for profile in external:
        if profile["variant"] != "fastnoise" or [p[0] for p in profile["points_m"]] != xs:
            raise ValueError("external runner changed coordinates or variant")
        reference = cls(profile["seed"], **plan["refined"])
        expected_features = [reference._features(x + plan["source_offset_m"]) for x in xs]
        observed_features = profile.pop("features_m")
        if len(observed_features) != len(xs) or any(not math.isfinite(y) or abs(a - y) > 1e-10 for a, y in zip(expected_features, observed_features)):
            raise ValueError("Rust sparse features differ from the actual Pylander functions")
    profiles.extend(external)
    for profile in profiles:
        if len(profile["points_m"]) != len(xs) or any(not math.isfinite(y) for _, y in profile["points_m"]):
            raise ValueError("invalid raw samples")
        profile["points_m"].sort()
    profiles.sort(key=lambda p: (VARIANTS.index(p["variant"]), p["seed"]))
    return profiles


def subset(points, spacing):
    return [p for p in points if (p[0] - points[0][0]) % spacing == 0]


def core(points, plan):
    return [p for p in points if plan["review_span_m"][0] <= p[0] <= plan["review_span_m"][1]]


def percentile(values, fraction):
    ordered = sorted(values)
    pos = (len(ordered) - 1) * fraction
    lo = int(pos)
    return ordered[lo] + (ordered[min(lo + 1, len(ordered) - 1)] - ordered[lo]) * (pos - lo)


def interpolation_error(fine, coarse):
    if len(coarse) < 2 or coarse[0][0] != fine[0][0] or coarse[-1][0] != fine[-1][0]:
        raise ValueError("interpolation domain mismatch")
    errors = []
    index = 0
    for x, y in fine:
        while index + 2 < len(coarse) and x > coarse[index + 1][0]:
            index += 1
        x0, y0 = coarse[index]
        x1, y1 = coarse[index + 1]
        if x1 <= x0:
            raise ValueError("non-increasing coordinates")
        errors.append(abs(y - (y0 + (y1 - y0) * (x - x0) / (x1 - x0))))
    return {"max_m": max(errors), "rms_m": math.sqrt(sum(e * e for e in errors) / len(errors))}


def significant_turns(points, height, half_width):
    # Local extrema only; adjacent extrema define prominence, not a global
    # topographic saddle. Width is measured at half that local prominence.
    indices = [0]
    previous_sign = 0
    for i in range(1, len(points)):
        dy = points[i][1] - points[i - 1][1]
        sign = (dy > 0) - (dy < 0)
        if sign:
            if previous_sign and sign != previous_sign:
                indices.append(i - 1)
            previous_sign = sign
    indices.append(len(points) - 1)
    turns = []
    for left, index, right in zip(indices, indices[1:], indices[2:]):
        x, y = points[index]
        prominence = min(abs(y - points[left][1]), abs(y - points[right][1]))
        if prominence < height:
            continue
        peak = y > points[left][1]
        level = y + (-1 if peak else 1) * prominence / 2
        crossings = []
        for direction, stop in ((-1, left), (1, right)):
            inner = index
            for outer in range(index + direction, stop + direction, direction):
                xo, yo = points[outer]
                xi, yi = points[inner]
                if (peak and yo <= level) or (not peak and yo >= level):
                    crossings.append(xi + (xo - xi) * (level - yi) / (yo - yi))
                    break
                inner = outer
        if len(crossings) == 2 and min(x - crossings[0], crossings[1] - x) >= half_width:
            turns.append({"kind": "peak" if peak else "valley", "x_m": x, "y_m": y})
    return turns


def describe(profile, plan):
    fine = core(profile["points_m"], plan)
    primary = subset(fine, plan["spacing_m"])
    ys = [y for _, y in primary]
    slopes = [(b[1] - a[1]) / (b[0] - a[0]) for a, b in zip(primary, primary[1:])]
    return {
        "variant": profile["variant"], "seed": profile["seed"],
        "start_height_m": ys[0], "min_height_m": min(ys), "max_height_m": max(ys),
        "relief_m": max(ys) - min(ys), "endpoint_rise_m": ys[-1] - ys[0],
        "max_abs_slope": max(map(abs, slopes)), "p95_abs_slope": percentile(list(map(abs, slopes)), 0.95),
        "rms_slope": math.sqrt(sum(s * s for s in slopes) / len(slopes)),
        "turns": significant_turns(primary, plan["turn_height_m"], plan["turn_half_width_m"]),
        "sampling": {str(step): interpolation_error(fine, subset(fine, step)) for step in (plan["spacing_m"], plan["coarser_spacing_m"])},
    }


def chart(profiles, plan, extent, width=600, height=270):
    margin = 42
    start, end = plan["review_span_m"]
    ymin, ymax = extent
    sx = lambda x: margin + (x - start) / (end - start) * (width - margin - 12)
    sy = lambda y: 12 + (ymax - y) / (ymax - ymin) * (height - margin - 12)
    parts = [f'<svg viewBox="0 0 {width} {height}" role="img" aria-label="Source-relative terrain height in metres; shared scale">']
    for i in range(5):
        y = ymin + (ymax - ymin) * i / 4
        parts.append(f'<path d="M {margin} {sy(y):.2f} H {width-12}" stroke="#dfe5eb"/><text x="{margin-5}" y="{sy(y)+4:.2f}" text-anchor="end">{y:.0f}</text>')
    for i in range(5):
        x = start + (end - start) * i / 4
        parts.append(f'<text x="{sx(x):.2f}" y="{height-15}" text-anchor="middle">{x:.0f}</text>')
    parts.append(f'<text x="{width/2}" y="{height-1}" text-anchor="middle">horizontal distance (m)</text>')
    for profile in profiles:
        points = subset(core(profile["points_m"], plan), plan["spacing_m"])
        origin = points[0][1]
        path = " ".join(f'{sx(x):.2f},{sy(y-origin):.2f}' for x, y in points)
        parts.append(f'<polyline points="{path}" fill="none" stroke="{COLORS[profile["variant"]]}" stroke-width="2.2"><title>{LABELS[profile["variant"]]} — seed {profile["seed"]}</title></polyline>')
    return "".join(parts) + "</svg>"


def shared_extent(profiles, plan):
    heights = []
    for profile in profiles:
        points = core(profile["points_m"], plan)
        heights.extend(y - points[0][1] for _, y in points)
    return (math.floor(min(heights) / 50) * 50 - 50, math.ceil(max(heights) / 50) * 50 + 50)


def overview_svg(profiles, plan, refined_only=False):
    selected = [p for p in profiles if not refined_only or p["variant"] != "baseline"]
    extent = shared_extent(selected, plan)
    columns = 2 if refined_only else 3
    rows = 3 if refined_only else 6
    width, row_height = columns * 470, 275
    heading = "Refined arms — one shared focus scale" if refined_only else "All 18 profiles — one shared vertical scale"
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{rows*row_height+65}" viewBox="0 0 {width} {rows*row_height+65}"><style>text{{font:13px sans-serif;fill:#243345}}</style><rect width="100%" height="100%" fill="white"/><text x="15" y="24">{heading}</text><text x="15" y="47">Height relative to h(0), metres; no amplitude rescaling. Shape study only — no flights.</text>']
    for row, seed in enumerate(plan["seeds"]):
        if refined_only:
            x, y = (row % 2) * 470 + 10, (row // 2) * row_height + 65
            group = [p for p in selected if p["seed"] == seed]
            title = f"Seed {seed}: green Pylander / orange FastNoiseLite"
            parts.append(f'<text x="{x}" y="{y+18}">{title}</text>')
            parts.append(chart(group, plan, extent, 450, 230).replace('<svg ', f'<svg x="{x}" y="{y+25}" width="450" height="230" ', 1))
        else:
            for column, variant in enumerate(VARIANTS):
                x, y = column * 470 + 10, row * row_height + 65
                group = [p for p in selected if p["seed"] == seed and p["variant"] == variant]
                parts.append(f'<text x="{x}" y="{y+18}">{LABELS[variant]} — seed {seed}</text>')
                parts.append(chart(group, plan, extent, 450, 230).replace('<svg ', f'<svg x="{x}" y="{y+25}" width="450" height="230" ', 1))
    return ("".join(parts) + "</svg>").encode()


def gallery(profiles, summary, plan):
    extent = shared_extent(profiles, plan)
    focus = shared_extent([p for p in profiles if p["variant"] != "baseline"], plan)
    lookup = {(p["variant"], p["seed"]): p for p in profiles}
    metrics = {(p["variant"], p["seed"]): p for p in summary["profiles"]}
    parts = ['<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Terrain profile study — no flights</title><style>body{font:16px system-ui,sans-serif;color:#243345;background:#f5f7fa;margin:24px auto;padding:0 20px;max-width:1500px}h1{font-size:28px}h2{scroll-margin-top:20px}.notice,article{background:white;border:1px solid #d5dce5;border-radius:8px;padding:16px;margin:12px 0}.grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px}svg{width:100%;height:auto;background:#fff}svg text{font:11px system-ui;fill:#536273}table{border-collapse:collapse;width:100%;background:white}th,td{border-bottom:1px solid #dfe5eb;padding:8px;text-align:right}th:first-child,td:first-child{text-align:left}a{color:#175eab}.meta{font-size:13px;color:#536273}.legend{display:flex;gap:24px;flex-wrap:wrap}nav{display:flex;gap:16px;flex-wrap:wrap}@media(max-width:850px){.grid{grid-template-columns:1fr}.table-wrap{overflow-x:auto}}</style><h1>Procedural terrain: 18-profile comparison</h1><div class="notice"><strong>Shape study only — no pads, planner runs, landing results or source promotion.</strong><p>Six fixed seeds × three recipes. All samples retained. The baseline is the actual hash-bound Pylander class; the two refined arms share the same recipe but use different noise primitives.</p><p>Plots subtract h(0) for display only; raw files retain absolute heights and the −160…1360 m halo. No height scaling or per-profile normalisation. Primary spacing: 4 m; reference: 1 m. The vertical scale is shared across all three-arm plots; the separately labelled refined overlays use a second shared scale.</p><p>Equal seed numbers do not imply equivalent worlds across backends. Significant turns require 8 m excursion to adjacent extrema and 40 m width on each side at half local prominence, not obstacles or full topographic prominence. Linear interpolation warnings compare against a 1 m reference, not a continuous-terrain clearance proof.</p><nav><a href="#metrics">Metrics</a><a href="plan.json">Frozen plan</a><a href="summary.json">Summary JSON</a><a href="receipt.json">Provenance receipt</a>']
    parts.extend(f'<a href="#seed-{seed}">Seed {seed}</a>' for seed in plan["seeds"])
    parts.append('</nav></div><div class="legend">')
    parts.extend(f'<span style="color:{COLORS[v]}">● {LABELS[v]}</span>' for v in VARIANTS)
    parts.append('</div>')
    for seed in plan["seeds"]:
        parts.append(f'<section id="seed-{seed}"><h2>Seed {seed} — common three-arm scale</h2><div class="grid">')
        for variant in VARIANTS:
            profile = lookup[variant, seed]
            stat = metrics[variant, seed]
            name = f'{variant}-seed-{seed}.json'
            parts.append(f'<article><h3>{LABELS[variant]}</h3>{chart([profile], plan, extent)}<p class="meta">Relief {stat["relief_m"]:.1f} m · endpoint rise {stat["endpoint_rise_m"]:+.1f} m · max slope {stat["max_abs_slope"]:.2f} m/m · significant turns {len(stat["turns"])}</p><a href="profiles/{name}">Raw heights + reference grid</a></article>')
        parts.append('</div><article><h3>Refined arms — shared focus scale (baseline excluded)</h3>')
        parts.append(chart([lookup[v, seed] for v in ("refined", "fastnoise")], plan, focus, width=1200, height=250))
        parts.append('</article></section>')
    parts.append('<h2 id="metrics">All profiles — no ranking or discarded seeds</h2><div class="table-wrap"><table><tr><th>Variant / seed</th><th>Relief m</th><th>Rise m</th><th>P95 slope</th><th>Max slope</th><th>Turns</th><th>4 m error</th><th>8 m error</th></tr>')
    for stat in summary["profiles"]:
        parts.append(f'<tr><td>{LABELS[stat["variant"]]} / {stat["seed"]}</td><td>{stat["relief_m"]:.2f}</td><td>{stat["endpoint_rise_m"]:+.2f}</td><td>{stat["p95_abs_slope"]:.3f}</td><td>{stat["max_abs_slope"]:.3f}</td><td>{len(stat["turns"])}</td><td>{stat["sampling"]["4"]["max_m"]:.3f} m</td><td>{stat["sampling"]["8"]["max_m"]:.3f} m</td></tr>')
    parts.append('</table></div><p>Recipe/input identity: <code>' + html.escape(summary["plan_sha256"]) + '</code>. Generated offline; not published into the accepted report site.</p></html>')
    return "".join(parts).encode()


def capture(args):
    plan_data = args.plan.read_bytes()
    plan = json.loads(plan_data)
    validate_plan(plan)
    root = args.pylander_root.resolve()
    binary = args.fastnoise_bin.resolve()
    load_reference(root, plan)
    source_bytes = {name: (HERE / name).read_bytes() for name in SOURCES}
    binary_hash = digest(binary.read_bytes())
    if not args.output.resolve().is_relative_to(HERE.parents[1] / "outputs" / "terrain-profile-study"):
        raise ValueError("capture must be inside outputs/terrain-profile-study, never the accepted site")
    args.output.mkdir(parents=True, exist_ok=False)
    write_new(args.output / "plan.json", plan_data)
    for name, data in source_bytes.items():
        write_new(args.output / "inputs" / "study" / name, data)
    for name in plan["pylander"]["files"]:
        write_new(args.output / "inputs" / "pylander" / name, (root / name).read_bytes())
    profiles = sample_batch(plan, root, binary)
    repeated = subprocess.run([
        sys.executable, str(HERE / "study.py"), "sample", "--plan", str(args.output / "plan.json"),
        "--pylander-root", str(args.output / "inputs" / "pylander"), "--fastnoise-bin", str(binary), "--reverse",
    ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=60)
    repeat_profiles = json.loads(repeated.stdout)
    if encoded(profiles) != encoded(repeat_profiles):
        raise ValueError("fresh-process/reversed-seed/reversed-coordinate repeat failed")
    if binary_hash != digest(binary.read_bytes()) or any(data != (HERE / name).read_bytes() for name, data in source_bytes.items()):
        raise ValueError("study source or binary changed during capture")
    load_reference(root, plan)
    if args.plan.read_bytes() != plan_data:
        raise ValueError("plan changed during capture")
    summary = {"schema": plan["schema"], "profile_only": True, "flight_runs": 0,
               "plan_sha256": digest(plan_data), "profile_count": len(profiles),
               "repeat_sha256": digest(encoded(profiles)), "repeat_passed": True,
               "profiles": [describe(p, plan) for p in profiles]}
    for profile in profiles:
        primary = subset(profile["points_m"], plan["spacing_m"])
        name = f'{profile["variant"]}-seed-{profile["seed"]}.json'
        write_new(args.output / "profiles" / name, encoded({"profile_only": True, "variant": profile["variant"], "seed": profile["seed"], "points_m": primary, "reference_points_m": profile["points_m"]}))
    write_new(args.output / "summary.json", encoded(summary))
    write_new(args.output / "index.html", gallery(profiles, summary, plan))
    write_new(args.output / "overview.svg", overview_svg(profiles, plan))
    write_new(args.output / "refined.svg", overview_svg(profiles, plan, refined_only=True))
    receipt = {"schema": plan["schema"], "profile_only": True, "flight_runs": 0,
               "python": sys.version, "fastnoise_binary_sha256": binary_hash,
               "source_sha256": {name: digest(data) for name, data in source_bytes.items()},
               "files": {str(p.relative_to(args.output)): digest(p.read_bytes()) for p in sorted(args.output.rglob("*")) if p.is_file()}}
    write_new(args.output / "receipt.json", encoded(receipt))
    verify(args.output, args.plan)
    print(json.dumps({"capture": str(args.output.resolve()), "profiles": len(profiles), "repeat_passed": True}))


def verify(output, expected_plan):
    receipt = json.loads((output / "receipt.json").read_bytes())
    plan_data = (output / "plan.json").read_bytes()
    if plan_data != expected_plan.read_bytes():
        raise ValueError("capture does not match the independently supplied plan")
    plan = json.loads(plan_data)
    validate_plan(plan)
    if receipt.get("schema") != plan["schema"] or set(receipt["source_sha256"]) != set(SOURCES):
        raise ValueError("capture schema/source inventory mismatch")
    expected_files = set(receipt["files"]) | {"receipt.json"}
    required_files = {"plan.json", "summary.json", "index.html", "overview.svg", "refined.svg", "receipt.json"}
    required_files.update("inputs/study/" + name for name in SOURCES)
    required_files.update("inputs/pylander/" + name for name in plan["pylander"]["files"])
    required_files.update(f"profiles/{variant}-seed-{seed}.json" for variant in VARIANTS for seed in plan["seeds"])
    actual_files = {str(p.relative_to(output)) for p in output.rglob("*") if p.is_file()}
    if actual_files != expected_files or expected_files != required_files:
        raise ValueError("capture file inventory mismatch")
    for name, expected in receipt["files"].items():
        path = output / name
        if not path.resolve().is_relative_to(output.resolve()) or digest(path.read_bytes()) != expected:
            raise ValueError(f"artifact mismatch: {name}")
    for name, expected in receipt["source_sha256"].items():
        if digest((HERE / name).read_bytes()) != expected or digest((output / "inputs" / "study" / name).read_bytes()) != expected:
            raise ValueError(f"capture tool differs from current study source: {name}")
    load_reference(output / "inputs" / "pylander", plan)
    profiles = []
    for variant in VARIANTS:
        for seed in sorted(plan["seeds"]):
            p = json.loads((output / "profiles" / f"{variant}-seed-{seed}.json").read_bytes())
            fine = p["reference_points_m"]
            left, right = plan["domain_m"]
            xs = [left + i * plan["reference_spacing_m"] for i in range(int((right-left)/plan["reference_spacing_m"])+1)]
            if p["variant"] != variant or p["seed"] != seed or not p["profile_only"] or [v[0] for v in fine] != xs:
                raise ValueError("profile identity/grid mismatch")
            if any(not math.isfinite(y) for _, y in fine) or p["points_m"] != subset(fine, plan["spacing_m"]):
                raise ValueError("raw/reference points disagree")
            profiles.append({"variant": variant, "seed": seed, "points_m": fine})
    summary = json.loads((output / "summary.json").read_bytes())
    expected_summary = {"schema": plan["schema"], "profile_only": True, "flight_runs": 0,
                        "plan_sha256": digest(plan_data), "profile_count": 18,
                        "repeat_sha256": digest(encoded(profiles)), "repeat_passed": True,
                        "profiles": [describe(p, plan) for p in profiles]}
    if summary != expected_summary or (output / "index.html").read_bytes() != gallery(profiles, summary, plan):
        raise ValueError("derived summary/gallery mismatch")
    if (output / "overview.svg").read_bytes() != overview_svg(profiles, plan) or (output / "refined.svg").read_bytes() != overview_svg(profiles, plan, True):
        raise ValueError("derived overview mismatch")
    if not receipt["profile_only"] or receipt["flight_runs"] != 0:
        raise ValueError("wrong evidence boundary")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ("capture", "sample"):
        cmd = sub.add_parser(command)
        cmd.add_argument("--plan", type=Path, default=HERE / "plan.json")
        cmd.add_argument("--pylander-root", type=Path, required=True)
        cmd.add_argument("--fastnoise-bin", type=Path, required=True)
        if command == "capture":
            cmd.add_argument("--output", type=Path, required=True)
        else:
            cmd.add_argument("--reverse", action="store_true")
    cmd = sub.add_parser("verify")
    cmd.add_argument("output", type=Path)
    cmd.add_argument("--plan", type=Path, default=HERE / "plan.json")
    args = parser.parse_args()
    if args.command == "capture":
        capture(args)
    elif args.command == "sample":
        print(encoded(sample_batch(json.loads(args.plan.read_bytes()), args.pylander_root, args.fastnoise_bin, args.reverse)).decode(), end="")
    else:
        verify(args.output, args.plan)
        print("Verified 18 profiles, raw grids, metrics, gallery and frozen input identities (no fresh generation).")


if __name__ == "__main__":
    main()
