"""Create-only reader adjudication of the three complete, sealed V14 captures.

No flights, native rebuilds, retries or edits to original captures. Copies the
native evidence byte-for-byte; preserves the original collector errors explicitly.
"""
import argparse
from pathlib import Path
import subprocess

import ballistic_feedback_sweep as sweep
import ballistic_mechanics_panel as panel
import diagnostics as d
import ridge_waypoint_panel as ridge
import study
import survey
import waypoint_entry_panel as entry

SEALS = {
    "exit-consistency": ("d01f7198d2d887a60c32313301f49ca2c73fe078e723fff5001e2faa76328ca4",
                         "2a0748079715c20e6a809079ca952fc75bcb6eef124cc68a4d953f946113d098"),
    "piecewise-early-target": ("f8246cedcb137a774987e5519291449547bcc2be25c2005ea3bb7a60234d4a6a",
                               "fee65ee985e4e01200f58d1cd2fc4d8ab088dcf8e07c04f842c2883048b29226"),
    "recovery-lead": ("63d3e4866b80025e130b385d225c7541941e34cb9d303373e99eed8f308d89e0",
                      "74e3421e3dc4a3bc83ab34c5815ba712684f4bc3e039557d9b5a63d018e42d33"),
}
ALIASES = {"receipt.json": "original-collector-receipt.json",
           "ballistic-feedback-sweep.json": "original-collector-results.json",
           "index.html": "original-collector-index.html"}
READER_FILES = ["studies/terrain_profiles/ballistic_feedback_sweep.py",
                "studies/terrain_profiles/ballistic_mechanics_review.py",
                "studies/terrain_profiles/test_ballistic_mechanics_panel.py"]


def root_for(mode):
    original = panel.capture(mode)
    return original.with_name(original.name + "-reader")


def verify(root):
    summary = panel.verify(root)
    report = d.read(root / "ballistic-feedback-sweep.json")
    repair = report["collector_reader_repair"]
    mode = d.read(root / "plan.json")["waypoint_experiment"]
    original_receipt = (root / ALIASES["receipt.json"]).read_bytes()
    original_results = (root / ALIASES["ballistic-feedback-sweep.json"]).read_bytes()
    if (study.digest(original_receipt), study.digest(original_results)) != SEALS[mode]:
        raise ValueError("reader adjudication lost original collector evidence")
    old = d.read(root / ALIASES["ballistic-feedback-sweep.json"])
    if (repair["additional_native_attempts"] != 0 or repair["native_artifacts_changed"]
            or repair["original_seals"] != list(SEALS[mode]) or len(old["rows"]) != 48
            or old["stopped_reason"] != "evidence_error"
            or report["source_after"] != old["source_after"]
            or report["protected_after"] != old["protected_after"]):
        raise ValueError("reader repair changed flight provenance or hid collector stop")
    files = d.read(root / ALIASES["receipt.json"])["files"]
    for path, digest in files.items():
        copied = root / ALIASES.get(path, path)
        if study.digest(copied.read_bytes()) != digest:
            raise ValueError("copied native/original evidence differs: " + path)
    for path, digest in repair["reader_files"].items():
        if study.digest(d.safe_file(root / "reader/source", path).read_bytes()) != digest:
            raise ValueError("unbound corrected reader")
    for current, previous in zip(report["rows"], old["rows"]):
        frozen = dict(previous, status="recorded", failure=None, result=current["result"])
        if (current != frozen or previous["status"] != "evidence_error" or previous["exit_code"] != 0
                or previous["failure"] != "landing countdown enabled under an older candidate identity"):
            raise ValueError("adjudication altered invocation or unknown collector error")
    return {"summary": summary, "admission": panel.admission(report, mode)}


def run(mode):
    original = panel.capture(mode)
    old_receipt = (original / "receipt.json").read_bytes()
    old_results = (original / "ballistic-feedback-sweep.json").read_bytes()
    if (study.digest(old_receipt), study.digest(old_results)) != SEALS[mode]:
        raise ValueError("unexpected interrupted collector identity")
    files = d.read(original / "receipt.json")["files"]
    survey.check_inventory(original, files)
    old = d.read(original / "ballistic-feedback-sweep.json")
    manifest = d.read(original / "manifest.json")
    if (old["source_after"] != manifest["source"] or old["protected_after"] != manifest["protected"]
            or ridge.rust_digest() != manifest["source"]["rust_source_tree_sha256"]
            or study.digest(panel.NATIVE.read_bytes()) != manifest["source"]["executable_sha256"]):
        raise ValueError("native source/binary/protected seal changed")
    plan = d.read(original / "plan.json")
    rows = []
    for previous in old["rows"]:
        row = dict(previous, status="recorded", failure=None)
        row["result"] = sweep.check_record(original, row, plan)
        rows.append(row)
    root = root_for(mode)
    survey.reserve(root)
    for path in files:
        study.write_new(root / ALIASES.get(path, path), d.safe_file(original, path).read_bytes())
    study.write_new(root / ALIASES["receipt.json"], old_receipt)
    for name in ("pd-eval", "batch-report"):
        (root / "candidate/bin" / name).chmod(0o755)
    reader_files = {}
    for path in READER_FILES:
        data = (survey.REPO / path).read_bytes()
        study.write_new(root / "reader/source" / path, data)
        reader_files[path] = study.digest(data)
    report = dict(old, rows=rows, summary=ridge.metrics(rows), stopped_reason=None,
                  collector_reader_repair={"reason": "recognize V14 inherited terminal identities and blocked early destination queries",
                                           "original_seals": list(SEALS[mode]), "reader_files": reader_files,
                                           "additional_native_attempts": 0, "native_artifacts_changed": False})
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    return verify(root)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("--mode", choices=list(SEALS), required=True)
    args = parser.parse_args()
    print(run(args.mode) if args.action == "run" else verify(root_for(args.mode)))
