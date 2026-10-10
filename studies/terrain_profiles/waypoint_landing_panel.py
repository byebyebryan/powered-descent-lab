"""One failure-only terminal duration; frozen comparisons and common rich reports.

9 same-source controls, 9 focused candidates, 5 early-only preservation records
and 21 combined preservation records. No tuning, retry, publication or adoption.
"""
import argparse
from pathlib import Path

import ballistic_feedback_sweep as sweep
import survey
import waypoint_clearance_panel as previous
import waypoint_entry_panel as entry
import waypoint_local_panel as local

PREVIOUS = survey.OUTPUTS / "capture-waypoint-local-preservation-20261008-v1"
NATIVE = survey.REPO / "target/ballistic-landing-duration-20261008/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
MODES = {"local-height-early-target": sweep.LOCAL_CANDIDATES["local-height-early-target"],
         "landing-duration": sweep.LANDING_CANDIDATES["landing-duration"]}
FOCUS = [715, 84, 142, 349, 6]


def layout(panel=False):
    primary = [f"random-{i:03}" for i in (previous.INDICES if panel else FOCUS)] + previous.CONTROLS
    return primary, ["random-715", "random-349"] if panel else ["random-715"]


def early_layout(panel=False):
    if panel:
        raise ValueError("no early-only full panel authorized")
    return ["random-715", *previous.CONTROLS], ["random-715"]


SPEC = entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "local-height-early-target", "landing-duration",
                       layout, "docs/ballistic_landing_duration_plan.md", local.verify, "waypoint-landing")
EARLY_SPEC = entry.PanelSpec(survey.OUTPUTS / "capture-waypoint-local-early-target-20261008-v1",
                             NATIVE, RENDERER,
                             {"early-target-landing-duration": sweep.LANDING_CANDIDATES["early-target-landing-duration"]},
                             "early-target", "early-target-landing-duration", early_layout,
                             SPEC.protocol, local.verify, "waypoint-landing-early")


def verify(root, early=False):
    return entry.verify(root, EARLY_SPEC if early else SPEC)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--mode", choices=[*MODES, *EARLY_SPEC.modes], default="landing-duration")
    parser.add_argument("--panel", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    spec = EARLY_SPEC if args.mode in EARLY_SPEC.modes else SPEC
    print(entry.verify(root, spec) if args.action == "verify" else entry.run(root, args.mode, args.panel, spec))
