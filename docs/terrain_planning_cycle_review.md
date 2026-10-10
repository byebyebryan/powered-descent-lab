# Terrain planning cycle diagnostics

Date: 2026-10-08. The remaining failures are not one launch-clearance problem.
The retained early-exit sweep still has 817 verified landings and 183 finite
planning stops. The diagnostic report adds state-aware proposals for each
planning cycle to the existing rich views; it does not change those flights.
The departure lift/advance design is parked while the actual give-up reasons
are reviewed.

## What the remaining stops describe

The final planning cycle in each stopped primary mission gives the following
disjoint groups. These are locations of proposed conflicts, not actual crashes
or complete root-cause classifications.

| Final proposed conflict or missing proposal | Unlaunched | After actual handoffs |
| --- | ---: | ---: |
| Source bridge | 46 | 0 |
| Ballistic coast | 6 | 19 |
| Nominal acquisition | 0 | 27 |
| Terminal bridge | 13 | 37 |
| No nominal available | 0 | 35 |
| Total | 65 | 118 |

All 183 actual endpoints remain flying / in progress. In particular, an
unlaunched mission can be rejected because its proposed terminal approach is
blocked far away. A local row's later terrain/domain failure also is not a flown
failure: the rejected query can continue beyond its earliest candidate handoff.

Of the 35 missing-nominal cases, 33 contain acquisition-demand rejections and
27 contain predicted negative-forward-velocity rejections. These categories
overlap, refer to failed proposal attempts, and do not establish physical
infeasibility. The supported proposal family is narrower than every possible
flight maneuver.

## Why mission 715 gave up

The first correction really happened. Intervention E is at 14.65 s, position
(140.01, −11.75) m and velocity (29.89, 23.96) m/s. Actual H1 is at 20.67 s,
position (323.71, 195.54) m and velocity (30.54, 44.77) m/s. The correction gains
207.29 m of height and 20.81 m/s of upward velocity while preserving almost the
same forward speed. Its two-second continuation certificate is a query, not an
additional flown coast or another waypoint.

Planning restarts at that actual H1. The next recorded nominal reaches a body
reserve conflict at 49.00 s, x=1150.00 m, during terminal approach. That is about
50 m before the target center, not the first obstacle. Eight intervention entries
and 336 local rows are searched; none meets every local handoff guard. The actual
flight therefore ends at H1, not at the future conflict.

The recorded rejected row `live_0_quarters_row_41` illustrates an important
guard interaction. Its required progress is x≥1162.81 m. At its first
continuation-rejected query H, x=1163.41 m and vx=82.76 m/s: it has passed the
progress coordinate. Only 0.45 s later, x=1200.66 m crosses the target center.
The two-second forward-only incoming-family guard rejects that continuation.
This is a supported-family/target-crossing rejection, not terrain contact at
that point, and not evidence of an accepted waypoint or a safe landing.

Thus more launch lift is not a diagnosis of this case. The progress/continuation
interaction is genuine query evidence, but it is not the only issue: the nominal
being corrected also needs review before changing those guards.

### Ballistic continuation alignment

The saved H1 proposal coasts for six seconds, then uses about 33 seconds of
powered terminal flight. The ballistic apex is near 25.23 s; powered flight
starts at 26.67 s, around x=507 m, with the target at x=1200 m. It is a smooth
powered reference, not a literal straight-line interpolation or an executed
continuation. Its pale suffix after predicted terrain contact is counterfactual.

The current no-acquisition candidate can retain an off-target ballistic path
because the powered terminal solver subsequently reaches the pad. That differs
from establishing a new ballistic transfer to the target. The
[updated aim and correction plan](ballistic_aim_correction_plan.md) selects
repeated aiming from actual state, terrain-driven waypoint targets and simple
destination approach safeguards rather than a long powered landing suffix.

## Other comparison cases

Mission `280` completes eight handoffs, then predicts a conflict in acquisition
at 54.13 s, x=1053.65 m. Its final boundary counts include 53,505 incomplete
prefixes, 6,498 insufficient-progress checks and 477 unsafe continuations. It is
a different stage from `715`; a prefix may run out of reserve before its handoff
or certificate completes.

Mission `983` completes one handoff and has no next nominal. Its actual endpoint
is x=1052.40 m with vx=71.55 m/s and vy=−37.00 m/s, leaving about 148 m to the
target. Saved attempts reject acquisition demand and forward reversal. The
report leaves its nominal overlay empty instead of drawing a fictional route.

Mission `327` remains unlaunched with a source-bridge conflict. Mission `999`
also remains unlaunched, but its proposed conflict is on terminal approach.
Direct landing `000`, corrected landing `030`, and the 13-handoff landing `565`
are retained positive comparisons, not new successes from this review.

## Report and reconstruction contract

The separately versioned report is
`/reports/eval/planner_v2_random_terrain/recheck-early-exit-20261008-cycles-v5/`.
Use Report home → Waypoint planning → Latest 1,000-world sweep. Open `715`, then
switch **Inspect cycle** between Launch and After H1 above the trajectory plot.
The original time plots, telemetry, actual handoffs, rich reference views and
failure-first batch tree remain available.

Across 1010 saved attempts, 2,165 planning cycles contain 2,130 recorded nominal
programs and 35 explicit missing nominals. Six rejected rows are reenacted in
the predeclared examples. Rejected query paths are hidden unless requested.

The purple dashed path reenacts the recorded nominal command program from the
cycle's actual state, including velocity, attitude and fuel. Pale continuation
after the audited contact is explicitly terrain-blind and counterfactual.
Dotted orange is a simple unpowered ballistic projection from that current
position/velocity; it is not the proposal or an admission test. Green highlights
the actual executed E-to-H correction, and the red cross marks the proposed
future terrain conflict.

Origins come from reenacting original source actions and matching every saved
sample, never from stepping a
restored saved snapshot. Each origin, actual E/H, saved final endpoint, nominal
conflict and audit endpoint must match exactly. The compiled dynamics and
fixed-row source files match the capture's source seal. This is a different
diagnostic executable, not the retained flight executable or a replacement
acceptance/source-replay proof.

Eight predeclared examples are `715/280/983/327/999/000/030/565`. Only already
recorded rejected rows are reenacted for query examples, with exact saved stop
state, reserve minimum, boundary counts and first-rejection checks. Neither
new nominal search nor route selection occurs. Other detail pages receive all
their nominal cycles and saved aggregate reasons without extra rejected-row
examples. The create-only receipt binds pages, sidecars and reconstruction
provenance; raw captures and accepted benchmark pages stay unchanged.

The maintained 11-step development gate passes, including 58 Node tests.
Selected retained cases match exact reconstructed boundaries and reasons;
Chromium checks confirm cycle changes update actual traces and pixels, original
rich views remain usable, overlay toggles work, missing nominals stay empty,
and mobile layout has no page overflow. These are automated presentation
checks, not human acceptance of the planner's behavior.

## Next decision

The selected next proposal is the [ballistic aim and correction plan](ballistic_aim_correction_plan.md).
Demonstrate the corrected aiming loop on `715`, review its actual powered-state
and execution-proof boundaries, then retest the full 1k and testing missions under
a frozen candidate. Do not restrict the repair to a new filter on the old powered
tail or tune the near-target progress guard to this one case. The other mechanisms
and direct/multi-handoff successes remain regression cases. The parked departure
design applies only to its 46-world cohort. Implementation, flights and promotion
remain separate from this diagnostic report update.
