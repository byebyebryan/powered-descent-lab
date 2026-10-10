"""Frozen local-height/early-target ablations; shared native proof/report runner.

Four 11-record modes plus 21 preservation records. No tuning, retry or publication.
"""
import argparse
from pathlib import Path

import ballistic_feedback_sweep as sweep
import survey
import waypoint_clearance_panel as previous
import waypoint_entry_panel as entry

PREVIOUS = survey.OUTPUTS / "capture-waypoint-entry-preservation-20261008-v1"
NATIVE = survey.REPO / "target/ballistic-local-waypoint-20261008/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
MODES = {"combined": sweep.ENTRY_CANDIDATES["combined"], **sweep.LOCAL_CANDIDATES}
FOCUS = [715, 349, 6, 807, 928, 139, 268]


def layout(panel=False):
    primary = [f"random-{i:03}" for i in (previous.INDICES if panel else FOCUS)] + previous.CONTROLS
    repeats = ["random-715", "random-349"] if panel else ["random-715"]
    return primary, repeats


SPEC = entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "combined", "local-height-early-target",
                       layout, "docs/ballistic_local_waypoint_plan.md", entry.verify, "waypoint-local")


def verify(root):
    return entry.verify(root, SPEC)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--mode", choices=list(MODES), default="local-height-early-target")
    parser.add_argument("--panel", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    print(verify(root) if args.action == "verify" else entry.run(root, args.mode, args.panel, SPEC))
