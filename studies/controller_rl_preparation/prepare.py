"""Input-only preparation accounting. Never imports a controller or policy.

Native collection tools own generation and verification. This script inventories
retained exposure, serializes declarations and reconciles physical identities.
It never loads individual final-test geometry or creates previews.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path


def read(path):
    return json.loads(Path(path).read_text())


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def write_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")


def resolved_entries(manifest):
    if manifest["schema_version"] == 1:
        return manifest["entries"]
    if manifest["schema_version"] == 2:
        return [r["proposal"] for r in manifest["receipts"] if r["proposal"] is not None]
    raise ValueError("unsupported manifest schema")


def inventory(root):
    collections, workers, seeds, physical, errors = [], [], set(), set(), []
    for directory, children, files in os.walk(root):
        directory = Path(directory)
        if "manifest.json" in files:
            path = directory / "manifest.json"
            try:
                manifest = read(path)
                if isinstance(manifest, dict) and "request" in manifest and "entries" in manifest:
                    request = manifest["request"]
                    entries = resolved_entries(manifest)
                    seeds.add(request["seed"])
                    physical.update(e["physical_sha256"] for e in entries)
                    usage = sorted(directory.glob("usage-*.json"))
                    collections.append({
                        "path": str(path), "sha256": digest(path),
                        "schema_version": manifest["schema_version"],
                        "role": request["role"], "seed": request["seed"],
                        "recipe": request["recipe"], "resolved": len(entries),
                        "admitted": len(manifest["entries"]),
                        "usage_files": {str(p): digest(p) for p in usage},
                        "exposure": "conservatively reserved entire generated namespace",
                    })
                    children[:] = []
                    continue
            except (ValueError, KeyError, OSError) as error:
                errors.append({"path": str(path), "error": str(error)})
        for name in sorted(files):
            path = directory / name
            if name.startswith("worker-") and name.endswith(".jsonl"):
                count, worker_seeds = 0, set()
                with path.open() as stream:
                    for line in stream:
                        item = json.loads(line)
                        if item.get("kind") == "proposal":
                            count += 1
                            worker_seeds.add(item["seed"])
                            if item.get("physical_sha256"):
                                physical.add(item["physical_sha256"])
                seeds.update(worker_seeds)
                workers.append({"path": str(path), "sha256": digest(path),
                                "proposals": count, "seeds": sorted(worker_seeds)})
            elif name.startswith("characterization-") and name.endswith(".json"):
                item = read(path)
                seeds.add(item["seed"])
        # Saved rich reports/captures are redundant projections. Individual
        # proposal/mission files are represented by native manifests above.
        children[:] = [n for n in children if n not in
                       ("captures", "missions", "proposals", "episodes", "run", "node_modules")
                       and "profile" not in n and "report" not in n
                       and "review" not in n and not n.startswith("chrome")]
    return {"schema_version": 1, "scope": str(root),
            "collections": sorted(collections, key=lambda x: x["path"]),
            "worker_logs": sorted(workers, key=lambda x: x["path"]),
            "reserved_seeds": sorted(seeds), "physical_sha256": sorted(physical),
            "errors": errors,
            "limits": ["Historical authored/ballistic worlds lack this versioned collection inventory.",
                       "Characterization reserves whole seed namespaces; physical identities were not retained.",
                       "No claim about unrecorded external runs; freeze is relative to this audited checkout.",
                       "Copied or incomplete artifacts may retain their original exposure role."]}


def panels():
    def c(axis, low, high):
        return {"axis": axis, "range": {"min": low, "max": high}}
    return [
        {"id": "wide-source-close-departure", "quota": 64,
         "constraints": [c("source_width", 64., 80.), c("departure_distance", 0., 40.)]},
        {"id": "narrow-target-close-approach", "quota": 64,
         "constraints": [c("target_width", 24., 32.), c("approach_distance", 0., 40.)]},
        {"id": "tight-significant-crests", "quota": 64,
         "constraints": [c("crest_count", 2., 100000.), c("min_crest_spacing", 0., 40.)]},
        {"id": "long-sustained-obstruction", "quota": 64,
         "constraints": [c("sustained_barrier_width", 800., 100000.)]},
    ]


def declare(exposure, template, output):
    exposure_sha256 = digest(exposure)
    exposure, template, output = read(exposure), read(template), Path(output)
    if exposure["errors"]:
        raise ValueError("unresolved inventory parse errors")
    if 20261012047 in exposure["reserved_seeds"]:
        raise ValueError("stress seed collides with audited exposure")
    requests = []
    declarations = [
        ("stress-support-a", "development", "layered_perlin", 20261012041, 128),
        ("stress-support-b", "development", "layered_perlin", 20261012042, 128),
        ("layered-validation", "validation", "layered_perlin", 20261012043, 512),
        ("value-validation", "validation", "value_noise", 20261012044, 512),
        ("layered-final-test", "held_out", "layered_perlin", 20261012045, 2048),
        ("value-final-test", "held_out", "value_noise", 20261012046, 2048),
    ]
    for name, role, backend, seed, count in declarations:
        if seed in exposure["reserved_seeds"]:
            raise ValueError("new seed collides with audited exposure")
        request = json.loads(json.dumps(template))
        request.update(role=role, seed=seed, indices=list(range(count)))
        request["recipe"]["backend"] = backend
        if name.startswith("stress-support"):
            request["population"] = {"kind": "stress", "panels": panels()}
        if name.endswith("support-b"):
            request["recipe"]["octaves"] = 2
            request["recipe"]["terrain"]["structure_wavelength_m"] = {"min": 100., "max": 400.}
        path = output / f"{name}.json"
        write_new(path, request)
        requests.append({"id": name, "file": path.name, "sha256": digest(path),
                         "role": role, "seed": seed, "proposals": count})
    write_new(output / "declaration.json", {
        "schema_version": 1, "exposure_inventory_sha256": exposure_sha256,
        "collections": requests, "initial_proposal_slots": sum(x["proposals"] for x in requests),
        "remaining_stress_slots": 512, "optional_resolution_slots": 128,
        "stress_seed": 20261012047, "stress_selection": "max minimum unique matching panel count, then sum, then A",
        "final_test_access": "native structural/hash verification and identity reconciliation only",
        "training_transitions": 0, "final_test_flights": 0,
    })


def stress_selection(roots):
    candidates = []
    for root in roots:
        manifest = read(Path(root) / "manifest.json")
        counts = Counter({p["id"]: 0 for p in manifest["request"]["population"]["panels"]})
        seen = set()
        for receipt in manifest["receipts"]:
            proposal = receipt["proposal"]
            if (proposal is None or not receipt["challenge"]["qualifies"]
                    or proposal["physical_sha256"] in seen):
                continue
            seen.add(proposal["physical_sha256"])
            counts.update(receipt["matching_stress_panels"])
        candidates.append({"root": str(root), "manifest_sha256": digest(Path(root)/"manifest.json"),
                           "matching_counts": dict(counts), "minimum": min(counts.values()),
                           "memberships": sum(counts.values())})
    winner = max(range(len(candidates)), key=lambda i:
                 (candidates[i]["minimum"], candidates[i]["memberships"], -i))
    return {"candidates": candidates, "selected": candidates[winner]["root"]}


def reconcile(prior, roots):
    seen = set(prior["physical_sha256"])
    cohorts, collisions = [], []
    for root in roots:
        manifest = read(Path(root)/"manifest.json")
        identities = {e["physical_sha256"] for e in manifest["entries"]}
        overlap = sorted(identities & seen)
        if overlap:
            collisions.append({"root": str(root), "physical_sha256": overlap})
        # Excluded proposals in earlier development/validation populations also
        # constitute known inputs; a later admitted final-test identity must not
        # collide with them. This reads manifest hashes, never individual worlds.
        seen.update(e["physical_sha256"] for e in resolved_entries(manifest))
        cohorts.append({"root": str(root), "manifest_sha256": digest(Path(root)/"manifest.json"),
                        "role": manifest["request"]["role"], "seed": manifest["request"]["seed"],
                        "population": manifest["request"]["population"]["kind"],
                        "admitted": len(manifest["entries"]), "counts": manifest["counts"]})
    return {"cohorts": cohorts, "collisions": collisions, "passed": not collisions,
            "scope": "admitted identities against historical inventory and all earlier resolved proposals"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("inventory"); p.add_argument("--root", type=Path, required=True); p.add_argument("--output", type=Path, required=True)
    p = sub.add_parser("declare"); p.add_argument("--exposure", type=Path, required=True); p.add_argument("--template", type=Path, required=True); p.add_argument("--output", type=Path, required=True)
    p = sub.add_parser("stress-selection"); p.add_argument("--collection", action="append", type=Path, required=True); p.add_argument("--output", type=Path, required=True)
    p = sub.add_parser("reconcile"); p.add_argument("--exposure", type=Path, required=True); p.add_argument("--collection", action="append", type=Path, required=True); p.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "inventory":
        result = inventory(args.root); write_new(args.output, result)
        print(json.dumps({"collections": len(result["collections"]), "worker_logs": len(result["worker_logs"]),
                          "unique_physical_inputs": len(result["physical_sha256"]), "errors": result["errors"]}))
    elif args.command == "declare":
        declare(args.exposure, args.template, args.output)
    elif args.command == "stress-selection":
        write_new(args.output, stress_selection(args.collection))
    else:
        result = reconcile(read(args.exposure), args.collection); write_new(args.output, result)
        if not result["passed"]:
            raise SystemExit("cohort collisions: freeze failed; no replacement authorized")


if __name__ == "__main__":
    main()
