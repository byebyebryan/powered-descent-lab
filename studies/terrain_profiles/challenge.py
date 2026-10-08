"""Global harder recipes; no solver feedback, seed filtering or corridor edits."""

import json
import math
from pathlib import Path
import random

import refinement as ref
import study

HERE = Path(__file__).resolve().parent
SCHEMA = "pd-lab.terrain-challenge-plan.v1"


def plan_filename(phase):
    names = {"calibration": "challenge_calibration_plan.json",
             "held_out": "challenge_plan.json",
             "validation_1k": "challenge_validation_1k_plan.json"}
    if phase not in names:
        raise ValueError("unknown challenge phase")
    return names[phase]


def validate(plan, refinement, base):
    phase = plan.get("phase")
    expected = {"calibration": (2026100701, 24, 6, [], 27),
                "held_out": (2026100702, 100, 25, [0, 25, 50, 75, 1], 108),
                "validation_1k": (2026100703, 1000, 250, [0, 250, 500, 750, 1], 1008)}.get(phase)
    if (plan.get("schema") != SCHEMA or expected is None
            or tuple(plan.get(k) for k in ("master_seed", "seed_count", "cases_per_recipe", "repeat_indices", "maximum_measured_attempts")) != expected
            or plan.get("excluded_seeds") != base["seeds"]
            or [plan.get(k) for k in ("workers", "checkpoint_count", "case_wall_limit_s", "campaign_wall_limit_s", "policy_version", "seed_max")]
            != [4, 10, 300, 10800, 3, 2147483246]
            or plan.get("sentinels") != ["v2_clear_685", "v2_ridge_early", "v2_plateau_reference_900"]
            or plan.get("control_plan_sha256") != study.digest((HERE / "plan.json").read_bytes())
            or plan.get("refinement_plan_sha256") != study.digest((HERE / "refinement_plan.json").read_bytes())):
        raise ValueError("changed challenge population/bounds")
    recipes = plan["recipes"]
    if (len(recipes) != 4 or len({r["id"] for r in recipes}) != 4
            or any(set(r) != {"id", "vertical_scale", "horizontal_scale"}
                   or not r["id"].replace("_", "").isalnum()
                   or not 4 <= r["vertical_scale"] <= 16
                   or not 0.6 <= r["horizontal_scale"] <= 1.5 for r in recipes)):
        raise ValueError("invalid global challenge recipes")
    if (phase == "validation_1k"
            and recipes != json.loads((HERE / "challenge_plan.json").read_bytes())["recipes"]):
        raise ValueError("fresh validation must retain the original four recipes")
    return plan


def draw(master, count, excluded):
    rng, used, values = random.Random(master), set(excluded), []
    while len(values) < count:
        seed = rng.randrange(2147483247)
        if seed not in used:
            used.add(seed)
            values.append(seed)
    return values


def seeds(plan):
    excluded = set(plan["excluded_seeds"])
    sanity = draw(2026100601, 100, excluded)
    excluded.update(sanity)
    if plan["phase"] in ("held_out", "validation_1k"):
        excluded.update(draw(2026100701, 24, excluded))
    if plan["phase"] == "validation_1k":
        excluded.update(draw(2026100702, 100, excluded))
    return draw(plan["master_seed"], plan["seed_count"], excluded)


def recipe(base, descriptor):
    value = dict(base["refined"])
    height, width = descriptor["vertical_scale"], descriptor["horizontal_scale"]
    for key in ("macro_amplitude", "structure_amplitude"):
        value[key] *= height
    for key in ("macro_frequency", "structure_frequency", "warp_frequency"):
        value[key] /= width
    value["warp_amplitude"] *= width
    value["feature_cell_size"] *= width
    return value


def sample(plan, base, refinement, root, reverse=False):
    cls = study.load_reference(root, base)
    xs = list(range(base["domain_m"][0], base["domain_m"][1] + 1, base["spacing_m"]))
    assignments = list(enumerate(seeds(plan)))
    if reverse:
        assignments.reverse()
        xs.reverse()
    profiles = []
    for index, seed in assignments:
        descriptor = plan["recipes"][index // plan["cases_per_recipe"]]
        generator = study.make_refined(cls, seed, recipe(base, descriptor), base["weighted_strength"])
        generator._ridge_noise = ref.RidgeSlice(generator._ridge_noise, refinement["ridge_slice_offset"])
        points = sorted([x, generator(x + base["source_offset_m"])] for x in xs)
        if any(not math.isfinite(y) for _, y in points):
            raise ValueError("nonfinite challenge terrain")
        profiles.append({"variant":descriptor["id"], "seed":seed, "points_m":points})
    return sorted(profiles, key=lambda p: p["seed"])


def prepared(profile, plan, refinement, base):
    template = json.loads((HERE / "scenario_template.json").read_bytes())
    value, geometry = ref.scenario(profile, refinement, base, template, study.encoded(plan))
    descriptor = next(r for r in plan["recipes"] if r["id"] == profile["variant"])
    identifier = f'challenge_{profile["variant"]}_seed_{profile["seed"]}'
    value.update(id=identifier, name=identifier,
                 description="Global harder Pylander recipe; unchanged local pad preparation, no corridor clearing.",
                 tags=["procedural_terrain", "terrain_challenge", plan["phase"]],
                 metadata={"study":plan["schema"], "phase":plan["phase"], "recipe":descriptor["id"],
                           "master_seed":str(plan["master_seed"])})
    points = [p for p in profile["points_m"] if 0 <= p[0] <= 1200]
    heights = [y for _, y in points]
    rise = heights[-1] - heights[0]
    geometry.update(recipe_id=descriptor["id"], vertical_scale=descriptor["vertical_scale"],
                    horizontal_scale=descriptor["horizontal_scale"], relief_m=max(heights)-min(heights),
                    endpoint_rise_m=rise,
                    maximum_slope=max(abs((b[1]-a[1])/(b[0]-a[0])) for a,b in zip(points,points[1:])),
                    maximum_height_above_endpoint_chord_m=max(y-heights[0]-rise*x/1200 for x,y in points))
    return value, geometry
