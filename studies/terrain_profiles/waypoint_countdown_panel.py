"""Single retained ballistic fallback arrival; create-only fixed comparisons.

11 same-source V8 controls, 11 focused V9 records, 5 early-only controls and
21 preservation records. No tuning, retry, publication or default promotion.
"""
import argparse
from pathlib import Path

import ballistic_feedback_sweep as sweep
import survey
import waypoint_clearance_panel as previous
import waypoint_entry_panel as entry
import waypoint_landing_panel as landing

PREVIOUS = survey.OUTPUTS / "capture-landing-duration-preservation-20261008-v1"
NATIVE = survey.REPO / "target/ballistic-landing-countdown-20261009/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
MODES = {"landing-duration": sweep.LANDING_CANDIDATES["landing-duration"],
         "landing-countdown": sweep.COUNTDOWN_CANDIDATES["landing-countdown"]}
FOCUS = [715, 84, 142, 268, 349, 6]


def layout(panel=False):
    primary = [f"random-{i:03}" for i in (previous.INDICES if panel else FOCUS)] + previous.CONTROLS
    return primary, ["random-715", "random-349"] if panel else ["random-715", "random-084"]


def verify_early_previous(root):
    return landing.verify(root, True)


SPEC = entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "landing-duration", "landing-countdown",
                       layout, "docs/ballistic_landing_countdown_plan.md", landing.verify, "waypoint-countdown")
EARLY_SPEC = entry.PanelSpec(survey.OUTPUTS / "capture-landing-duration-early-preservation-20261008-v1",
                             NATIVE, RENDERER,
                             {"early-target-landing-countdown": sweep.COUNTDOWN_CANDIDATES["early-target-landing-countdown"]},
                             "early-target-landing-duration", "early-target-landing-countdown", landing.early_layout,
                             SPEC.protocol, verify_early_previous, "waypoint-countdown-early")


def verify(root, early=False):
    return entry.verify(root, EARLY_SPEC if early else SPEC)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["run", "verify"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--mode", choices=[*MODES, *EARLY_SPEC.modes], default="landing-countdown")
    parser.add_argument("--panel", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    spec = EARLY_SPEC if args.mode in EARLY_SPEC.modes else SPEC
    print(entry.verify(root, spec) if args.action == "verify" else entry.run(root, args.mode, args.panel, spec))
