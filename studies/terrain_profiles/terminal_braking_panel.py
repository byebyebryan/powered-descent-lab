"""One opt-in braking guard, four focus records then 21 conditional preservation records.

No tuning, retries, default promotion, accepted publication or automatic 1k run.
Reuse the sealed common collector, rich reports and exact original-source proofs.
"""
import argparse
from pathlib import Path

import ballistic_feedback_sweep as sweep
import coast_terminal_validation as previous
import diagnostics as d
import survey
import waypoint_entry_panel as entry

NATIVE = survey.REPO / "target/ballistic-terminal-braking-20261009/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
FOCUS_ROOT = survey.OUTPUTS / "capture-terminal-braking-focus-20261009-v1"
MODES = sweep.BRAKING_CANDIDATES
PROTOCOL = "docs/ballistic_terminal_braking_plan.md"


def focus_layout(panel=False):
    if panel:
        raise ValueError("focus allowance cannot expand")
    return ["random-142", "random-974"], ["random-142", "random-974"]


FOCUS_SPEC = entry.PanelSpec(
    survey.OUTPUTS / "capture-coast-terminal-sweep-20261009-v1", NATIVE, RENDERER,
    MODES, "coast-terminal", "landing-braking-guard", focus_layout, PROTOCOL,
    sweep.verify, "terminal-braking-focus")
PANEL_SPEC = entry.PanelSpec(
    previous.PANEL_ROOT, NATIVE, RENDERER, MODES, "coast-terminal", "landing-braking-guard",
    previous.layout, PROTOCOL, previous.verify, "terminal-braking-preservation")


def gate(rows, panel=False):
    primary, repeats = previous.layout(True) if panel else focus_layout()
    if [r["attempt_id"] for r in rows] != primary + ["repeat-" + name for name in repeats]:
        raise ValueError("braking guard inventory differs")
    if any(r["status"] != "recorded" for r in rows):
        return {"passed": False, "lost_landings": [], "abnormal_outcomes": []}
    lost = [r["case_id"] for r in rows if r["cohort"] != "repeat"
            and r["previous_result"]["verified_landing"] and not r["result"]["verified_landing"]]
    abnormal = [r["attempt_id"] for r in rows
                if r["result"]["physical_outcome"] not in ("flying", "landed_on_target")]
    successes = all(r["result"]["verified_landing"] for r in rows) if not panel else all(
        r["result"]["verified_landing"] for r in rows if r["cohort"] == "control")
    return {"passed": successes and not lost and not abnormal,
            "lost_landings": lost, "abnormal_outcomes": abnormal}


def verify(root, panel=False):
    spec = PANEL_SPEC if panel else FOCUS_SPEC
    summary = entry.verify(root, spec)
    return {"summary": summary, **gate(d.read(root / "ballistic-feedback-sweep.json")["rows"], panel)}


def run(root, panel=False):
    spec = PANEL_SPEC if panel else FOCUS_SPEC
    if panel:
        if not verify(FOCUS_ROOT)["passed"]:
            raise ValueError("focused failures do not authorize preservation flights")
        if entry.source(spec) != d.read(FOCUS_ROOT / "manifest.json")["source"]:
            raise ValueError("source/binary/renderer changed after focus")
    entry.run(root, "landing-braking-guard", panel, spec)
    return verify(root, panel)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--panel", action="store_true")
    args = parser.parse_args()
    print(run(args.root.resolve(), args.panel) if args.action == "run"
          else verify(args.root.resolve(), args.panel))
