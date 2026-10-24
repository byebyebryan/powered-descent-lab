"""Input-only independent mixture and predeclared endpoint panel; no flights."""
import argparse
import random
from pathlib import Path

from composition_inputs import FAMILIES, proposal
from arrival import read, sha, write_new

PRIMARY_SEED = 2310241853
BOUNDARY_SEED = 2310241854
# height, offset, lateral speed, sink, attitude: 0/1 choose range endpoints.
PATTERNS = [(0, 0, 0, 0, 0), (1, 1, 1, 1, 1), (0, 1, 0, 1, 0),
            (1, 0, 1, 0, 1), (0, 1, 1, 1, 1)]


def scenarios(anchor, cohort, test_seed=None):
    if cohort not in ["primary", "boundary"]:
        raise ValueError("unknown qualification cohort")
    seed = (PRIMARY_SEED if cohort == "primary" else BOUNDARY_SEED) if test_seed is None else test_seed
    rng = random.Random(seed)
    cells = [(rng.randrange(4), rng.randrange(3), None) for _ in range(300)] if cohort == "primary" else [
        (f, s, p) for f in range(4) for s in range(3) for p in range(5)]
    for index, (family, sink, pattern) in enumerate(cells):
        scenario, _ = proposal(anchor, "reference-"+cohort, seed, family, sink, index)
        scenario.update(id=f"reference-{cohort}-{index:03}", seed=seed+index,
            name=f"{cohort} {FAMILIES[family]} sink-band {sink} original {index}",
            description="Frozen input-only timing qualification; no outcome admission or replacements")
        scenario["metadata"].update(generator="timing-reference-input-v1", cohort=cohort,
            independent_mixture=str(cohort == "primary").lower(), original=str(index))
        scenario["tags"] = ["timing_reference_v1", "flat", cohort, FAMILIES[family]]
        if pattern is not None:
            h, x, lateral, down, angle = PATTERNS[pattern]
            direction = 1 if index % 2 == 0 else -1
            speed = 0 if family == 0 else ([22., 28.] if family != 2 else [37., 43.])[lateral]
            scenario["initial_state"] = {"position_m": {"x": 0 if family == 0 else -direction*[310.,390.][x], "y": [725.,875.][h]},
                "velocity_mps": {"x": direction*speed, "y": -([13.,28.,43.][sink]+4*down)},
                "attitude_rad": (direction*.25 if family == 3 else 0)+[-.04,.04][angle],
                "angular_rate_radps": 0.}
            scenario["metadata"].update(direction=str(direction), endpoint_pattern=str(pattern))
        yield scenario


def generate(anchor, cohort, output):
    root = Path(output)
    root.mkdir()
    entries = []
    for scenario in scenarios(anchor, cohort):
        path = root/"scenarios"/(scenario["id"]+".json")
        write_new(path, scenario)
        case = {"id": scenario["id"]+"-direct", "state_id": scenario["id"],
            "population": cohort+"_"+scenario["metadata"]["family"], "choice": "direct",
            "scenario_path": str(path.resolve()), "scenario_sha256": sha(path), "generator_sha256": sha(__file__)}
        case_path = root/"cases"/(case["id"]+".json")
        write_new(case_path, case)
        entries.append({"id": case["id"], "path": str(case_path.resolve()), "sha256": sha(case_path)})
    write_new(root/"result.json", {"cohort": cohort, "cases": entries, "generator_sha256": sha(__file__),
        "native_steps": 0, "admissions": 0, "replacements": 0})
    return entries


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("anchor", type=Path)
    parser.add_argument("cohort", choices=["primary", "boundary"])
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    generate(read(args.anchor), args.cohort, args.output)
