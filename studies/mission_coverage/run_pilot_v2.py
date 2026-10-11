#!/usr/bin/env python3
"""Create-only, declared input calibration using the native V2 frontdoors.

This orchestrates generation and input QA only. It cannot run an agent, train,
publish reports, mutate a server, resume into an existing root or generate holdout.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
DECLARATION = Path(__file__).parent / "pilot_v2" / "pilot.json"
SOURCE_PATHS = ["Cargo.toml", "Cargo.lock", "pd-core", "pd-missions",
                "studies/mission_coverage/run_pilot_v2.py", "studies/mission_coverage/pilot_v2"]


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_json(path: Path, data: object) -> None:
    with path.open("x") as stream:
        json.dump(data, stream, indent=2, allow_nan=False)
        stream.write("\n")


def bounded_file(path: Path, maximum: int = 1024 * 1024) -> bytes:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > maximum:
        raise ValueError(f"expected bounded regular declaration: {path}")
    with path.open("rb") as stream:
        data = stream.read(maximum + 1)
    if len(data) > maximum:
        raise ValueError(f"declaration grew beyond its limit: {path}")
    return data


def load_declaration(path: Path) -> tuple[dict, dict[str, bytes], list[dict], dict]:
    data = bounded_file(path)
    cfg = json.loads(data)
    expected = {"schema_version", "pilot_version", "scope", "collections", "spec_files", "file_sha256",
                "diagnostic_axes", "diagnostic_joints", "reference_qa", "preview_review",
                "flight_training_budget", "held_out_generation", "benchmark_freeze"}
    if set(cfg) != expected or cfg["schema_version"] != 1 or cfg["pilot_version"] != "pd-missions/challenge-calibration-v2-a":
        raise ValueError("unsupported pilot declaration")
    if cfg["scope"] != "exposed_development_inputs_only" or cfg["flight_training_budget"] != 0 or cfg["held_out_generation"] is not False or cfg["benchmark_freeze"] is not False:
        raise ValueError("this runner permits only exposed input-only development calibration")
    files = {"pilot.json": data}
    for name, expected_sha in cfg["file_sha256"].items():
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,80}\.json", name):
            raise ValueError("declaration requires local basename-only JSON references")
        contents = bounded_file(path.parent / name)
        if digest(contents) != expected_sha:
            raise ValueError(f"declaration identity changed: {name}")
        files[name] = contents
    if set(cfg["spec_files"]) != {"geometry", "challenge", "morphology"}:
        raise ValueError("complete metric, challenge and morphology specs are required")
    specs = {key: json.loads(files[name]) for key, name in cfg["spec_files"].items()}
    requests = []
    names = set()
    for collection in cfg["collections"]:
        if set(collection) != {"id", "recipe_file", "seed", "index_start", "index_count", "role", "population"}:
            raise ValueError("unsupported collection declaration fields")
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}", collection["id"]) or collection["id"] in names:
            raise ValueError("collection ids must be distinct safe basenames")
        names.add(collection["id"])
        if collection["role"] != "development" or collection["population"] != {"kind": "broad"}:
            raise ValueError("this calibration declares separate broad development streams")
        for field in ["seed", "index_start", "index_count"]:
            if type(collection[field]) is not int or not 0 <= collection[field] <= 2**64 - 1:
                raise ValueError(f"collection {field} must be an unsigned integer")
        start, count = collection["index_start"], collection["index_count"]
        if not 1 <= count <= 20000 or start + count - 1 > 2**64 - 1:
            raise ValueError("collection schedule exceeds native bounds")
        requests.append({"schema_version": 2, "role": "development", "recipe": json.loads(files[collection["recipe_file"]]),
                         "geometry_spec": specs["geometry"], "challenge_rule": specs["challenge"],
                         "morphology_spec": specs["morphology"], "population": collection["population"],
                         "seed": collection["seed"], "indices": list(range(start, start + count))})
    if not 1 <= len(requests) <= 2:
        raise ValueError("calibration requires one or two bounded separate streams")
    qa = cfg["reference_qa"]
    if set(qa) != {"spacing_m", "regular_index_stride", "regular_index_offsets", "boundary_extra_per_backend",
                   "boundary_score", "maximum_per_backend", "diagnostic_flags", "flags_are_not_benchmark_acceptance"} or qa["flags_are_not_benchmark_acceptance"] is not True:
        raise ValueError("unsupported or misrepresented reference QA declaration")
    preview = cfg["preview_review"]
    if set(preview) != {"maximum_per_backend", "selection", "missions_per_page", "open_bands_at_once",
                       "normalize_display_direction", "metadata_filters", "membership_independent_of_display"} or preview["membership_independent_of_display"] is not True or preview["missions_per_page"] != 12 or preview["open_bands_at_once"] != 1 or preview["normalize_display_direction"] is not True or preview["metadata_filters"] != ["backend", "physical_direction"]:
        raise ValueError("unsupported bounded review declaration")
    audit_spec = {"schema_version": 1, "audit_version": "pd-missions/input-coverage-v1",
                  "coverage": {"schema_version": 1, "axes": cfg["diagnostic_axes"], "joints": cfg["diagnostic_joints"]},
                  "reference_qa": {k: v for k, v in qa.items() if k not in {"boundary_score", "flags_are_not_benchmark_acceptance"}},
                  "preview_maximum": preview["maximum_per_backend"]}
    return cfg, files, requests, audit_spec


def git(*args: str) -> bytes:
    return subprocess.check_output(["rtk", "proxy", "git", *args], cwd=ROOT)


def source_binding() -> dict[str, str]:
    names = git("ls-files", "-z", "--", *SOURCE_PATHS).split(b"\0")
    return {name.decode(): digest((ROOT / name.decode()).read_bytes()) for name in names if name}


def committed_declaration(path: Path, files: dict[str, bytes]) -> None:
    for name, data in files.items():
        actual_path = path if name == "pilot.json" else path.parent / name
        relative = actual_path.resolve().relative_to(ROOT).as_posix()
        if git("show", f"HEAD:{relative}") != data:
            raise ValueError(f"commit the declaration before generation: {relative}")
    subprocess.run(["rtk", "proxy", "git", "diff", "--quiet", "HEAD", "--", *SOURCE_PATHS], cwd=ROOT, check=True)


def run_stage(output: Path, name: str, args: list[str]) -> None:
    print(f"{name}: started", flush=True)
    with (output / f"{name}.log").open("x") as stream:
        subprocess.run(["rtk", "proxy", *args], cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT, check=True)
    print(f"{name}: completed", flush=True)


def run_pilot(path: Path, output: Path) -> dict:
    cfg, files, requests, audit_spec = load_declaration(path)
    committed_declaration(path, files)
    before = source_binding()
    output.mkdir()
    (output / "declaration").mkdir()
    for name, contents in files.items():
        with (output / "declaration" / name).open("xb") as stream:
            stream.write(contents)
    write_json(output / "audit-spec.json", audit_spec)
    write_json(output / "run-request.json", {"schema_version": 1, "head": git("rev-parse", "HEAD").decode().strip(),
               "source_sha256": before, "declaration_sha256": digest(files["pilot.json"]),
               "scheduled_total": sum(len(r["indices"]) for r in requests), "flight_training_budget": 0})
    try:
        run_stage(output, "build", ["cargo", "build", "--release", "-p", "pd-missions", "--example", "missions"])
        cli = ROOT / "target/release/examples/missions"
        cli_sha256 = digest(cli.read_bytes())
        if source_binding() != before:
            raise ValueError("source changed while building calibration tools")
        for collection, request in zip(cfg["collections"], requests, strict=True):
            request_path = output / f"request-{collection['id']}.json"
            write_json(request_path, request)
            run_stage(output, f"check-{collection['id']}", [str(cli), "check-v2", "--request", str(request_path), "--audit-spec", str(output / "audit-spec.json")])
        results = []
        admitted_physical: dict[str, list[str]] = {}
        for collection in cfg["collections"]:
            name = collection["id"]
            destination = output / name
            run_stage(output, f"collect-{name}", [str(cli), "collect", "--request", str(output / f"request-{name}.json"), "--output", str(destination)])
            audit_path = output / f"audit-{name}.json"
            run_stage(output, f"audit-{name}", [str(cli), "audit-v2", "--collection", str(destination), "--spec", str(output / "audit-spec.json"), "--output", str(audit_path), "--usage-id", "input-calibration"])
            audit = json.loads(bounded_file(audit_path, 128 * 1024 * 1024))
            manifest_sha = digest(bounded_file(destination / "manifest.json", 64 * 1024 * 1024))
            if not audit["complete_saved_input_verification"] or audit["immutable_file_sha256"]["manifest.json"] != manifest_sha or audit["request"]["seed"] != collection["seed"]:
                raise ValueError("native audit did not verify the declared collection")
            for row in audit["rows"]:
                if row["disposition"]["kind"] == "accepted":
                    admitted_physical.setdefault(row["proposal"]["physical_sha256"], []).append(name)
            results.append({"id": name, "audit_file": audit_path.name, "audit_sha256": digest(audit_path.read_bytes()),
                            "manifest_sha256": manifest_sha, "counts": audit["counts"], "reference_qa": len(audit["reference_qa"]),
                            "preview_inputs": len(audit["preview_indices"]), "complete_saved_input_verification": True})
        if source_binding() != before:
            raise ValueError("source changed during input calibration")
        if digest(cli.read_bytes()) != cli_sha256:
            raise ValueError("native executable changed during input calibration")
        record = {"schema_version": 1, "completed": True, "declaration_sha256": digest(files["pilot.json"]),
                  "cli_sha256": cli_sha256, "source_unchanged": True, "collections": results,
                  "flight_training_budget": 0, "benchmark_frozen": False, "report_publication": False}
        record["exact_shared_admitted_inputs"] = sum(len(names) > 1 for names in admitted_physical.values())
        write_json(output / "run-result.json", record)
        return record
    except Exception as exc:
        write_json(output / "run-failed.json", {"schema_version": 1, "completed": False, "detail": str(exc),
                   "source_unchanged": source_binding() == before, "flight_training_budget": 0})
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--declaration", type=Path, default=DECLARATION)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--output", type=Path)
    mode.add_argument("--plan", action="store_true", help="read-only declaration/file check; native preflight occurs during run")
    args = parser.parse_args()
    if args.plan:
        cfg, files, requests, spec = load_declaration(args.declaration)
        print(json.dumps({"declaration_sha256": digest(files["pilot.json"]), "collections": cfg["collections"],
                          "scheduled_total": sum(len(r["indices"]) for r in requests), "audit_spec": spec,
                          "native_preflight_pending": True, "generation_started": False}, indent=2))
    else:
        record = run_pilot(args.declaration.absolute(), args.output.absolute())
        print(json.dumps(record, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(str(exc), file=sys.stderr)
        raise SystemExit(1) from exc
