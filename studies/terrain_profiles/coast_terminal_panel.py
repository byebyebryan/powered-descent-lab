"""Automatic coast-to-terminal: fixed 11-control / 11-candidate comparisons.

No saved takeover clock, tuning, retries, publication or default adoption.
"""
import argparse
from pathlib import Path

import ballistic_feedback_sweep as sweep
import survey
import waypoint_entry_panel as entry
import waypoint_countdown_panel as countdown

PREVIOUS = survey.OUTPUTS / "capture-landing-countdown-preservation-20261009-v1"
NATIVE = survey.REPO / "target/ballistic-coast-terminal-20261009/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
MODES = {"landing-countdown": sweep.COUNTDOWN_CANDIDATES["landing-countdown"],
         "coast-terminal": sweep.COAST_CANDIDATE}


def layout(panel=False):
    if panel:
        raise ValueError("no wider coast-terminal panel authorized")
    return countdown.layout()


SPEC = entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "landing-countdown", "coast-terminal",
                       layout, "docs/ballistic_coast_terminal_plan.md", countdown.verify, "coast-terminal")


def verify(root):
    return entry.verify(root, SPEC)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--mode", choices=list(MODES), default="coast-terminal")
    args = parser.parse_args()
    root = args.root.resolve()
    print(verify(root) if args.action == "verify" else entry.run(root, args.mode, False, SPEC))
