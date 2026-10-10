"""Finish the same 1002 planned invocations; keep contained domain exits unverified.

Never overwrite the interrupted capture or rerun an already attempted world.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib.util
import json
import math
from pathlib import Path
import re
import subprocess
import time

SCRIPT = Path(__file__).resolve()
spec = importlib.util.spec_from_file_location("centering_sweep", SCRIPT.with_name("run-terminal-centering-sweep.py"))
original = importlib.util.module_from_spec(spec)
spec.loader.exec_module(original)
sweep, d, study, survey = original.sweep, original.d, original.study, original.survey
PARTIAL = original.ROOT
ROOT = survey.OUTPUTS / "capture-terminal-centering-sweep-20261009-v1-complete"
PROTOCOL = "docs/ballistic_terminal_centering_continuation_plan.md"
PARTIAL_SHA = "616d6e9471714330be654f308597c98661918dd31c61cf518f1ddb47924c6d87"
SKIP = {"receipt.json", "ballistic-feedback-sweep.json", "index.html", "logs/renderer.stdout", "logs/renderer.stderr"}


def domain_error(root, row):
    if row["status"] != "runner_error" or row["exit_code"] != 1 or row["result"] is not None:
        return False
    stderr = (root / "logs" / (row["attempt_id"] + ".stderr")).read_text()
    match = re.fullmatch(r"Error: exact body-envelope terrain query failed: terrain query x=([-+0-9.eE]+) lies outside domain \[-160, 1360\]\n", stderr)
    return bool(match and math.isfinite(float(match[1])) and not -160 <= float(match[1]) <= 1360)


def verify(root=ROOT):
    report = sweep.verify(root, contract_reader=original.contract, gate_reader=original.require_gate)
    record = d.read(root / "collection-continuation.json")
    prior_bytes = (root / "interrupted-results.json").read_bytes()
    receipt_bytes = (root / "interrupted-receipt.json").read_bytes()
    before = json.loads(prior_bytes)
    files = json.loads(receipt_bytes)["files"]
    if (study.digest(receipt_bytes) != PARTIAL_SHA or files["ballistic-feedback-sweep.json"] != study.digest(prior_bytes)
            or record["original_receipt_sha256"] != PARTIAL_SHA
            or record["runner_sha256"] != study.digest((root / "continuation-runner.py").read_bytes())
            or record["protocol_sha256"] != study.digest((root / "continuation-plan.md").read_bytes())
            or record["retries"] != 0 or record["maximum_total_native_invocations"] != 1002
            or record["retained_invocations"] != 280 or record["remaining_invocations"] != 722
            or report["rows"][:280] != before["rows"][:280]):
        raise ValueError("continuation/prefix binding differs")
    for name, digest in files.items():
        if name not in SKIP and study.digest(d.safe_file(root, name).read_bytes()) != digest:
            raise ValueError("copied interrupted evidence changed: " + name)
    if not report.get("collection_complete") or report["stopped_reason"] != "completed_with_terrain_domain_errors":
        raise ValueError("continuation did not finish the complete original inventory")
    errors = [r for r in report["rows"] if r["status"] != "recorded"]
    if (any(not domain_error(root, r) for r in errors)
            or report["terrain_domain_error_ids"] != [r["attempt_id"] for r in errors]
            or len(report["rows"]) != 1002
            or record["retained_terrain_domain_error_ids"] != ["random-261", "random-278"]):
        raise ValueError("non-domain error, missing attempts or wrong exception inventory")
    return report


def run():
    if study.digest((PARTIAL / "receipt.json").read_bytes()) != PARTIAL_SHA:
        raise ValueError("different interrupted capture")
    before = sweep.verify(PARTIAL, contract_reader=original.contract, gate_reader=original.require_gate)
    plan = original.contract(PARTIAL / "plan.json")
    original.require_gate(plan)
    manifest = d.read(PARTIAL / "manifest.json")
    if (before["stopped_reason"] != "runner_error:random-261:None"
            or any(r["status"] == "not_attempted" for r in before["rows"][:280])
            or any(r["status"] != "not_attempted" for r in before["rows"][280:])
            or [r["attempt_id"] for r in before["rows"][:280] if domain_error(PARTIAL, r)] != ["random-261", "random-278"]
            or original.collection_source(original.prior.NATIVE) != manifest["source"]
            or sweep.protected_state(plan) != manifest["protected"]):
        raise ValueError("different invocation prefix/source/protected state")
    for row in before["rows"][:280]:
        if row["status"] != "recorded" and not domain_error(PARTIAL, row):
            raise ValueError("uncontained interrupted error")
    survey.reserve(ROOT)
    old_receipt = (PARTIAL / "receipt.json").read_bytes()
    for name, digest in json.loads(old_receipt)["files"].items():
        if name in SKIP:
            continue
        data = d.safe_file(PARTIAL, name).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("interrupted capture changed during copy")
        study.write_new(ROOT / name, data)
    for name in ("pd-eval", "batch-report"):
        (ROOT / "candidate/bin" / name).chmod(0o755)
    study.write_new(ROOT / "interrupted-receipt.json", old_receipt)
    study.write_new(ROOT / "interrupted-results.json", (PARTIAL / "ballistic-feedback-sweep.json").read_bytes())
    runner, protocol = SCRIPT.read_bytes(), (original.REPO / PROTOCOL).read_bytes()
    study.write_new(ROOT / "continuation-runner.py", runner)
    study.write_new(ROOT / "continuation-plan.md", protocol)
    record = {"schema": "pd-lab.terminal-centering-continuation.v1", "original_receipt_sha256": PARTIAL_SHA,
              "runner_sha256": study.digest(runner), "protocol_sha256": study.digest(protocol),
              "retained_invocations": 280, "remaining_invocations": 722, "maximum_total_native_invocations": 1002,
              "retries": 0, "retained_terrain_domain_error_ids": ["random-261", "random-278"]}
    study.write_new(ROOT / "collection-continuation.json", study.encoded(record))
    rows = before["rows"]
    started, stopped = time.monotonic(), None
    with ThreadPoolExecutor(max_workers=plan["workers"]) as pool:
        for offset in range(280, len(rows), 20):
            if before["collection_wall_s"] + time.monotonic() - started >= plan["campaign_wall_limit_s"]:
                stopped = "campaign_wall_limit"
                break
            wave = list(pool.map(lambda row: sweep.attempt(ROOT, row, plan), rows[offset:offset + 20]))
            rows[offset:offset + len(wave)] = wave
            study.write_new(ROOT / "progress" / f"wave-{offset:04}.json", study.encoded(wave))
            stats = sweep.summary(rows)
            errors = [r["attempt_id"] for r in rows if r["status"] != "recorded" and r["status"] != "not_attempted"]
            print(json.dumps({"attempted": sum(r["status"] != "not_attempted" for r in rows),
                              "verified_landings": stats["verified_landings"], "native_error_ids": errors}), flush=True)
            bad = next((r for r in wave if r["status"] != "recorded" and not domain_error(ROOT, r)), None)
            abnormal = next((r for r in wave if r["status"] == "recorded"
                             and r["result"]["physical_outcome"] not in ("flying", "landed_on_target")), None)
            if bad or abnormal:
                stopped = f"uncontained_error:{(bad or abnormal)['attempt_id']}"
                break
            if (original.collection_source(original.prior.NATIVE) != manifest["source"]
                    or sweep.protected_state(plan) != manifest["protected"]
                    or study.digest((PARTIAL / "receipt.json").read_bytes()) != PARTIAL_SHA
                    or SCRIPT.read_bytes() != runner or (original.REPO / PROTOCOL).read_bytes() != protocol):
                stopped = "source_protected_or_protocol_drift"
                break
    complete = stopped is None and all(r["status"] != "not_attempted" for r in rows)
    report = {"schema": "pd-lab.ballistic-feedback-sweep.v1", "rows": rows, "summary": sweep.summary(rows),
              "stopped_reason": stopped or "completed_with_terrain_domain_errors", "collection_complete": complete,
              "terrain_domain_error_ids": [r["attempt_id"] for r in rows if domain_error(ROOT, r)],
              "collection_wall_s": before["collection_wall_s"] + time.monotonic() - started,
              "source_after": original.collection_source(original.prior.NATIVE), "protected_after": sweep.protected_state(plan),
              "manifest_sha256": study.digest((ROOT / "manifest.json").read_bytes())}
    study.write_new(ROOT / "ballistic-feedback-sweep.json", study.encoded(report))
    rendered = subprocess.run([str(ROOT / "candidate/bin/batch-report"), str(ROOT)], capture_output=True, timeout=120)
    study.write_new(ROOT / "logs/renderer.stdout", rendered.stdout)
    study.write_new(ROOT / "logs/renderer.stderr", rendered.stderr)
    study.write_new(ROOT / "receipt.json", study.encoded({"files": survey.inventory(ROOT)}))
    if rendered.returncode or not complete:
        raise ValueError("continuation stopped or renderer failed; sealed partial evidence retained")
    verify()
    print(json.dumps({"output": str(ROOT), "summary": report["summary"], "native_errors": report["terrain_domain_error_ids"]}), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("run", "verify"))
    args = parser.parse_args()
    if args.action == "run":
        run()
    else:
        report = verify()
        print(json.dumps({"complete": report["collection_complete"], "native_errors": report["terrain_domain_error_ids"],
                          "summary": report["summary"]}))
