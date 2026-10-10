"""Append-only continuation after the same-clock verifier false positive.

Keeps the original wave/evidence error and all native flights. Only the reader
changes; freezes its sole source change explicitly alongside the original seal.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
from pathlib import Path
import subprocess
import sys
import time

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "studies/terrain_profiles"))
import ballistic_feedback_sweep as sweep
import diagnostics as d
import study
import survey


def run(root):
    if root != (REPO / "outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1").resolve():
        raise ValueError("continuation restricted to the interrupted frozen capture")
    if any((root / p).exists() for p in ("receipt.json", "ballistic-feedback-sweep.json", "collector-verifier-repair.json")):
        raise ValueError("already finalized or continuation reserved")
    plan = sweep.contract(root / "plan.json")
    manifest = d.read(root / "manifest.json")
    native = REPO / plan["native_path"]
    path = "studies/terrain_profiles/ballistic_feedback_sweep.py"
    repaired = (REPO / path).read_bytes()
    repair = {"path": path, "original_sha256": manifest["source"]["files"][path],
              "repaired_sha256": study.digest(repaired),
              "reason": "Previous-goal H and next-goal recovery can share a tick. Check active goal revision and ordering, not time alone."}
    expected = dict(manifest["source"], files=dict(manifest["source"]["files"]))
    expected["files"][path] = repair["repaired_sha256"]
    if sweep.source_state(native) != expected or sweep.protected_state(plan) != manifest["protected"]:
        raise ValueError("change beyond the sole verification reader repair")
    rows = sweep.attempts(manifest["cases"], plan)
    offset, rechecks = 0, []
    for wave_path in sorted((root / "progress").glob("wave-*.json")):
        if wave_path.name != f"wave-{offset:04}.json":
            raise ValueError("noncontiguous retained waves")
        wave = d.read(wave_path)
        for i, actual in enumerate(wave):
            frozen = rows[offset + i]
            if any(actual[k] != v for k, v in frozen.items() if k != "status"):
                raise ValueError("wave identity differs")
            if actual["status"] == "evidence_error":
                if actual["failure"] != "recovery invented waypoint handoff" or actual["exit_code"] != 0:
                    raise ValueError("not the known verification false positive")
                fixed = dict(actual, status="recorded", failure=None)
                fixed["result"] = sweep.check_record(root, fixed, plan)
                rechecks.append({"attempt_id": actual["attempt_id"], "original_wave": str(wave_path.relative_to(root)),
                                 "original_wave_sha256": study.digest(wave_path.read_bytes()),
                                 "original_disposition": actual["status"], "original_failure": actual["failure"],
                                 "rechecked_record": fixed})
                wave[i] = fixed
            elif actual["status"] != "recorded" or sweep.check_record(root, actual, plan) != actual["result"]:
                raise ValueError("other retained evidence/collection failure")
        rows[offset:offset + len(wave)] = wave
        offset += len(wave)
    if len(rechecks) != 1 or rechecks[0]["attempt_id"] != "random-716":
        raise ValueError("unexpected verification repair set")
    study.write_new(root / "collector-verifier-source.py", repaired)
    study.write_new(root / "collector-verifier-repair.json", study.encoded(dict(repair, rechecks=rechecks)))
    study.write_new(root / "collector-continuation-runner.py", Path(__file__).read_bytes())
    started = (root / "ledger-initial.json").stat().st_mtime
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        for start in range(offset, len(rows), 20):
            if time.time() - started >= plan["campaign_wall_limit_s"]:
                raise ValueError("original campaign wall bound reached")
            wave = list(pool.map(lambda row: sweep.attempt(root, row, plan), rows[start:start + 20]))
            rows[start:start + len(wave)] = wave
            study.write_new(root / "progress" / f"wave-{start:04}.json", study.encoded(wave))
            stats = sweep.summary(rows)
            print(f"recorded={stats['recorded_count']} landings={stats['verified_landings']}", flush=True)
            if any(r["status"] != "recorded" for r in wave):
                raise ValueError("new collection failure; retain all evidence")
            if sweep.source_state(native) != expected or sweep.protected_state(plan) != manifest["protected"]:
                raise ValueError("source/protected state changed beyond recorded reader repair")
    report = {"schema": "pd-lab.ballistic-feedback-sweep.v1", "rows": rows,
              "summary": sweep.summary(rows), "stopped_reason": None,
              "collection_wall_s": time.time() - started, "source_after": sweep.source_state(native),
              "protected_after": sweep.protected_state(plan),
              "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "collection_recovery": d.read(root / "collector-recovery.json"),
              "collector_verifier_repair": repair}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    rendered = subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, timeout=120)
    study.write_new(root / "logs/renderer.stdout", rendered.stdout)
    study.write_new(root / "logs/renderer.stderr", rendered.stderr)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if rendered.returncode:
        raise ValueError("renderer failed; sealed native evidence retained")
    sweep.verify(root)
    spec = importlib.util.spec_from_file_location("recovery", REPO / "scripts/resume-ballistic-feedback-sweep.py")
    recovery = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(recovery)
    recovery.validate_recovery(root)
    print("Complete: 1000 primary worlds, two repeats; reader repair and four interrupted invocations retained.", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    args = parser.parse_args()
    run(args.capture.resolve())
