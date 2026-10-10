"""Append-only recovery of the interrupted October 8 terrain-correction sweep.

No dynamics/source changes or historical overwrites. Missing process bookkeeping
is not reconstructed as an observed exit. At most one worker-wave's orphaned
native attempts is repeated under explicitly separate create-only run roots.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
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


def validate_recovery(root):
    record = d.read(root / "collector-recovery.json")
    report = d.read(root / "ballistic-feedback-sweep.json")
    rows = {row["attempt_id"]: row for row in report["rows"]}
    if record["additional_interrupted_invocations"] != len(record["orphaned_attempts"]):
        raise ValueError("wrong explicit recovery invocation count")
    if study.digest((root / "collector-recovery-runner.py").read_bytes()) != record["runner_sha256"]:
        raise ValueError("recovery runner seal differs")
    for original in record["orphaned_attempts"]:
        case_id = original["attempt_id"]
        row = rows[case_id]
        recovered = original["recovery_attempt_id"]
        if (row["process_attempt_id"] != recovered or row["output_dir"] != "runs/" + recovered
                or row["prior_output_dir"] != "runs/" + case_id
                or d.read(root / row["output_dir"] / "feedback.json") != d.read(root / row["prior_output_dir"] / "feedback.json")):
            raise ValueError("orphan/recovery original flight equality differs")
        for suffix in ("stdout", "stderr"):
            if (root / "logs" / f"{case_id}.{suffix}").read_bytes() != (root / "logs" / f"{recovered}.{suffix}").read_bytes():
                raise ValueError("logical-case log is not the observed recovery process log")
        if study.digest((root / row["prior_output_dir"] / "feedback.json").read_bytes()) != original["retained_feedback_sha256"]:
            raise ValueError("retained orphan feedback changed")
    return record


def run(root):
    expected = REPO / "outputs/eval/planner_v2_random_terrain/capture-terrain-correction-20261008-v1"
    if root != expected.resolve() or root.is_symlink():
        raise ValueError("recovery is restricted to this exact interrupted capture")
    if any((root / name).exists() for name in ("receipt.json", "ballistic-feedback-sweep.json", "collector-recovery.json")):
        raise ValueError("capture already finalized or recovery already started")
    plan = sweep.contract(root / "plan.json")
    if plan["candidate_id"] != sweep.TERRAIN_CANDIDATE:
        raise ValueError("not the frozen terrain-correction candidate")
    manifest = d.read(root / "manifest.json")
    native = REPO / plan["native_path"]
    if (sweep.source_state(native) != manifest["source"]
            or sweep.protected_state(plan) != manifest["protected"]):
        raise ValueError("source or protected state differs before recovery")
    rows = sweep.attempts(manifest["cases"], plan)
    offset = 0
    for path in sorted((root / "progress").glob("wave-*.json")):
        if path.name != f"wave-{offset:04}.json":
            raise ValueError("noncontiguous retained waves")
        wave = d.read(path)
        for actual, frozen in zip(wave, rows[offset:offset + len(wave)]):
            if any(actual[k] != v for k, v in frozen.items() if k != "status"):
                raise ValueError("retained wave identity differs")
            if actual["status"] != "recorded" or sweep.check_record(root, actual, plan) != actual["result"]:
                raise ValueError("retained wave is not verified")
        rows[offset:offset + len(wave)] = wave
        offset += len(wave)
    completed = {row["attempt_id"] for row in rows[:offset]}
    orphan_paths = sorted(p for p in (root / "runs").iterdir() if p.name not in completed)
    next_ids = {row["attempt_id"] for row in rows[offset:offset + plan["workers"]]}
    if len(orphan_paths) > plan["workers"] or any(p.name not in next_ids for p in orphan_paths):
        raise ValueError("unexpected interrupted attempt set")
    orphans = []
    for path in orphan_paths:
        if not (path / "feedback.json").is_file() or any((root / "logs" / f"{path.name}.{s}").exists() for s in ("stdout", "stderr")):
            raise ValueError("orphan is not a complete native run missing process bookkeeping")
        orphans.append({"attempt_id": path.name, "process_exit": "unobserved",
                        "recovery_attempt_id": "recovery-" + path.name,
                        "retained_feedback_sha256": study.digest((path / "feedback.json").read_bytes())})
    runner = Path(__file__).read_bytes()
    record = {"schema": "pd-lab.collector-recovery.v1", "verified_prefix_attempts": offset,
              "additional_interrupted_invocations": len(orphans), "orphaned_attempts": orphans,
              "primary_cases": 1000, "planned_verified_attempts": 1002,
              "maximum_total_native_invocations": 1002 + len(orphans),
              "reason": "Collector process absent before finalization; native orphan exit/stdout unobserved. No tuning or source change.",
              "runner_sha256": study.digest(runner)}
    study.write_new(root / "collector-recovery-runner.py", runner)
    study.write_new(root / "collector-recovery.json", study.encoded(record))
    orphan_ids = {r["attempt_id"] for r in orphans}

    def attempt(row):
        if row["attempt_id"] not in orphan_ids:
            return sweep.attempt(root, row, plan)
        process_id = "recovery-" + row["attempt_id"]
        actual = sweep.attempt(root, dict(row, attempt_id=process_id), plan)
        if actual["status"] != "recorded":
            raise ValueError("recovery attempt failed; retain both attempts")
        if d.read(root / actual["output_dir"] / "feedback.json") != d.read(root / "runs" / row["attempt_id"] / "feedback.json"):
            raise ValueError("orphan and independently repeated feedback differ")
        for suffix in ("stdout", "stderr"):
            study.write_new(root / "logs" / f'{row["attempt_id"]}.{suffix}',
                            (root / "logs" / f"{process_id}.{suffix}").read_bytes())
        actual.update(attempt_id=row["attempt_id"], process_attempt_id=process_id,
                      prior_output_dir="runs/" + row["attempt_id"])
        if sweep.check_record(root, actual, plan) != actual["result"]:
            raise ValueError("recovery logical-case binding differs")
        return actual

    started = (root / "ledger-initial.json").stat().st_mtime
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        for start in range(offset, len(rows), 20):
            if time.time() - started >= plan["campaign_wall_limit_s"]:
                raise ValueError("original campaign wall bound reached")
            wave = list(pool.map(attempt, rows[start:start + 20]))
            rows[start:start + len(wave)] = wave
            study.write_new(root / "progress" / f"wave-{start:04}.json", study.encoded(wave))
            stats = sweep.summary(rows)
            print(f"recorded={stats['recorded_count']} landings={stats['verified_landings']}", flush=True)
            if any(r["status"] != "recorded" for r in wave):
                raise ValueError("new infrastructure/evidence failure; retain append-only waves")
            if sweep.source_state(native) != manifest["source"] or sweep.protected_state(plan) != manifest["protected"]:
                raise ValueError("source or protected state drift")
    report = {"schema": "pd-lab.ballistic-feedback-sweep.v1", "rows": rows,
              "summary": sweep.summary(rows), "stopped_reason": None,
              "collection_wall_s": time.time() - started,
              "source_after": sweep.source_state(native), "protected_after": sweep.protected_state(plan),
              "manifest_sha256": study.digest((root / "manifest.json").read_bytes()),
              "collection_recovery": record}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    rendered = subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, timeout=120)
    study.write_new(root / "logs/renderer.stdout", rendered.stdout)
    study.write_new(root / "logs/renderer.stderr", rendered.stderr)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    if rendered.returncode:
        raise ValueError("renderer failed; sealed native evidence retained")
    sweep.verify(root)
    validate_recovery(root)
    print("Completed original 1000 cases plus two repeats; four interrupted invocations retained separately.", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    if args.verify:
        validate_recovery(args.capture.resolve())
        print("Collector recovery provenance verified.")
    else:
        run(args.capture.resolve())
