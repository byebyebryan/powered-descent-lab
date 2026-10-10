"""Frozen V14 mechanics ablations, conditional combination and conditional 1k.

Reuse the common create-only collector and rich reports. No tuning or retries.
"""
import argparse
from pathlib import Path

import ballistic_feedback_sweep as sweep
import diagnostics as d
import study
import survey
import waypoint_entry_panel as entry

PREVIOUS = survey.OUTPUTS / "capture-terminal-coordination-diagnostic-1k-20261009-v1"
SEAL = ("49a9dd87d8df9725dd594c9979470d8984446aac31e3aecfec483db64910ccfe",
        "7f66120ba2851da54c490937591c4fdc3e9ff370738cb6216dc3d78f87fc6121")
NATIVE = survey.REPO / "target/ballistic-mechanics-20261009/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
MODES = {**sweep.COORDINATION_CANDIDATES, **sweep.MECHANICS_CANDIDATES}
SUBJECTS = {"exit-consistency": [13, 47, 56, 61],
            "piecewise-early-target": [44, 50, 62, 81],
            "recovery-lead": [12, 16, 17, 20, 21]}
OTHER = [0, 55, 35, 94, 896, 48, 755]
PRESERVED = [6, 8, 10, 15, 250, 251, 252, 253, 500, 501, 504, 505,
             750, 756, 757, 759, 84, 142, 349, 715, 974]
DIRECT = [1, 264, 503, 753]
FOCUS = sorted(set(OTHER + PRESERVED + DIRECT + sum(SUBJECTS.values(), [])))
INDEPENDENT = tuple(SUBJECTS)


def capture(mode, full=False):
    stage = "1k" if full else "focus"
    return survey.OUTPUTS / f"capture-ballistic-mechanics-{mode}-{stage}-20261009-v1"


def layout(panel=False):
    indices = range(1000) if panel else FOCUS
    repeats = [44, 47, 20, 349, 715] if panel else [47, 44, 20]
    return ([f"random-{i:03}" for i in indices], [f"random-{i:03}" for i in repeats])


def previous_verify(root):
    if (study.digest((root / "receipt.json").read_bytes()),
            study.digest((root / "ballistic-feedback-sweep.json").read_bytes())) != SEAL:
        raise ValueError("different frozen V13 reference")
    receipt = d.read(root / "receipt.json")["files"]
    survey.check_inventory(root, receipt)
    report = d.read(root / "ballistic-feedback-sweep.json")
    if report["stopped_reason"] is not None or len(report["rows"]) != 1005:
        raise ValueError("incomplete V13 reference")


SPEC = entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "terminal-coordination",
                       "mechanics-combined", layout, "docs/ballistic_mechanics_plan.md",
                       previous_verify, "ballistic-mechanics")


def admission(report, mode):
    rows = [r for r in report["rows"] if r["cohort"] != "repeat"]
    if [r["case_id"] for r in rows] != layout()[0]:
        raise ValueError("wrong focus admission population")
    landed = lambda r: (r.get("result") or {}).get("verified_landing", False)
    gains = [r["case_id"] for r in rows if landed(r) and not r["previous_result"]["verified_landing"]]
    losses = [r["case_id"] for r in rows if not landed(r) and r["previous_result"]["verified_landing"]]
    subjects = {f"random-{i:03}" for i in SUBJECTS.get(mode, sum(SUBJECTS.values(), []))}
    subject_gains = [name for name in gains if name in subjects]
    return {"passed": report["stopped_reason"] is None and not losses and bool(subject_gains),
            "gains": gains, "losses": losses, "subject_gains": subject_gains}


def verify(root):
    summary = entry.verify(root, SPEC)
    plan = d.read(root / "plan.json")
    report = d.read(root / "ballistic-feedback-sweep.json")
    if (plan["previous_receipt_sha256"], plan["previous_results_sha256"]) != SEAL:
        raise ValueError("different portable V13 reference")
    for row in report["rows"]:
        if int(row["case_id"].split("-")[1]) in DIRECT:
            current = d.read(root / row["output_dir"] / "feedback.json")
            prior = d.read(root / "previous-feedback" / (row["case_id"] + ".json"))
            if current["ordinary_flight"] != prior["ordinary_flight"]:
                raise ValueError("direct control flight changed")
    return summary


def require_admission(root, mode, source):
    verify(root)
    report = d.read(root / "ballistic-feedback-sweep.json")
    if report["source_after"] != source:
        raise ValueError("candidate source changed after mechanism admission")
    verdict = admission(report, mode)
    if not verdict["passed"]:
        raise ValueError("mechanism admission failed: " + str(verdict))
    return verdict


def run(mode, full=False):
    if full and mode != "mechanics-combined":
        raise ValueError("only admitted combined source may run the full population")
    if mode == "mechanics-combined":
        source = entry.source(SPEC)
        for independent in INDEPENDENT:
            require_admission(capture(independent), independent, source)
        if full:
            require_admission(capture(mode), mode, source)
    return entry.run(capture(mode, full), mode, full, SPEC)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("--mode", choices=list(MODES), default="terminal-coordination")
    parser.add_argument("--full", action="store_true")
    args = parser.parse_args()
    root = capture(args.mode, args.full)
    if args.action == "verify":
        print(verify(root))
    else:
        print(run(args.mode, args.full))
