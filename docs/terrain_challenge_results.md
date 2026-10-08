# Harder procedural-terrain challenge results

Date: 2026-10-07. Completes the [coverage plan](terrain_challenge_plan.md).
The earlier [100-case all-clear sanity sweep](random_terrain_full_sweep_results.md)
and the accepted 44-case planner benchmark remain unchanged and separate.

## Verdict

The independent held-out population completed all 100 cases. **64 initial
nominal paths were terrain-blocked; 21/64 (32.8%) verified landed after local
corrections. All 36/36 clear paths verified landed directly.** Overall, 57/100
landed and 43/100 stopped at finite planning limits. No case crashed physically,
failed integrity/source replay, timed out or remained unattempted.

This is meaningful procedural waypoint evidence rather than another all-clear
sanity pack. It is also evidence of substantial remaining coverage limits, not
an arbitrary-terrain reliability claim. A finite policy stop is not proof that
the terrain is physically impossible or that a broader search could succeed.

Three separate preservation controls landed with zero, one and three actual
handoffs. All five predeclared repeats match their original complete non-timing
flight records exactly, including two finite stops and a four-handoff landing.
Controls, repeats and the separate calibration are not added to the held-out
100-case denominator. All 108 held-out-capture attempts have passing integrity
and final-source replay evidence.

## Held-out coverage

Each recipe has 25 fresh cases, frozen before generation or flight. Both
calibration and held-out seed pools exclude the original 100 sanity seeds and
the six shape-study seeds; the held-out pool also excludes all 24 calibration
seeds. There was no per-seed rejection, crop selection or outcome-based tuning.

| Global recipe | Vertical / horizontal scale | Initially blocked | Blocked landings | Clear landings | All landings |
| --- | --- | ---: | ---: | ---: | ---: |
| `mountains_4x` | 4 / 1 | 6/25 | 1/6 | 19/19 | 20/25 |
| `mountains_8x` | 8 / 1 | 20/25 | 7/20 | 5/5 | 12/25 |
| `broad_massifs_8x` | 8 / 1.5 | 15/25 | 7/15 | 10/10 | 17/25 |
| `successive_ridges_8x` | 8 / 0.6 | 23/25 | 6/23 | 2/2 | 8/25 |
| **Held-out total** | Four unchanged recipes | **64/100** | **21/64** | **36/36** | **57/100** |

The scales transform Pylander's amplitudes, frequencies, warp distances and
feature-cell widths consistently. Composition, three octaves, weighting, ridge
slice, vehicle, Earth gravity, 120/60 Hz clocks, 90 s deadline and 1200 m route
length stay unchanged. The nominal choice remains terrain-blind. Obstacles are
generated globally, not inserted along an observed trajectory.

Raw 4 m core-span profile relief spans 168.50–1246.16 m and endpoint rise spans
-849.43–805.54 m, compared with 39.79–150.64 m relief in the original sanity pack.
The authoritative surface remains the 4 m polyline, with original-height 36 m
local pad shelves and 24 m transitions. Outside the two local pad patches,
geometry is unchanged. These are not continuous-noise clearance guarantees or
claims that every profile is within the vehicle's feasible envelope.

Thirty-seven cases executed local corrections, with **74 actual handoffs** in
total. Sixteen had at least two handoffs; eight of those landed. Successful
corrected flights include two-, three-, four- and six-handoff landings. Verified
landing times span 37.10–69.75 s. The complete physical/mission/integrity/replay
tuple, not a process exit code or correction count, establishes each landing.

## What did not work

| Final planning stop | Cases | Executed corrections before stop | Interpretation |
| --- | ---: | --- | --- |
| `NoClearing` | 34 | 27 with zero; six with one; one with three | The finite local grid found no acceptable clearing boundary |
| `NoNominal` | 7 | Two with one; three with two; two with three | Airborne nominal establishment failed after an actual handoff |
| `CorrectionLimit` | 2 | Six each | Repeated local clearing reached the existing cap without finishing |

The 27 zero-correction `NoClearing` cases remain at physics step zero on the
source pad. They are planning refusals, **not renewed first-step crashes**.
The other 16 finite stops have real, replay-verified clearing segments. Their
physical state is still flying and the mission is in progress when planning
stops; an integrity pass does not imply recovery or landing after that stop.

The local-search records retain prefix, progress, continuation and entry-clock
rejections. Their boundary counts are many correlated candidate evaluations,
not independent case denominators. For example, `random-000` exhausts 168 rows
without an accepted boundary, dominated by insufficient progress. That does
not by itself diagnose a missing search row or physical impossibility.

Six of the seven post-handoff `NoNominal` cases are close to the destination
(x≈925–1056 m) with forward speeds ≈59–96 m/s. Rejections predominantly name
acquisition acceleration, reverse-forward predictions or terminal feasibility;
the seventh stops near x=121 m. This supports investigating handoff-to-terminal
state correction as a smaller secondary cluster, not assuming one universal
terrain-height problem. The two capped cases have only reached x≈262 and 323 m
after six handoffs, indicating limited route progress in those particular runs.

## Calibration and retained repairs

The separate 24-case calibration completed 15/24 landings: 6/15 blocked landings
and 9/9 clear direct landings. It includes two two-handoff landings and nine
finite stops (seven `NoClearing`, two `NoNominal`). All four recipes were retained
unchanged for held-out generation; calibration judged coverage, not which seeds
or recipes happened to land. Its source identity predates the later pad-admission
repair and is not relabeled as the held-out source.

| Retained capture | Measured flight invocations | Outcome |
| --- | ---: | --- |
| `capture-challenge-20261007-calibration-v1` | 7 | Three controls and four random attempts; rich-output validation defect stopped collection |
| `capture-challenge-20261007-calibration-v2` | 27 | Complete 24-case calibration plus three controls |
| `capture-challenge-20261007-main-v1` | 0 | Input preparation stopped on `random-034`; partial source/input snapshot and failure receipt retained |
| `capture-challenge-20261007-main-v2` | 108 | Complete 100 held-out cases, three controls and five repeats |

There are 142 retained flight invocations across these attempts, not 142
independent worlds. Neither repair changed trajectory selection, controls,
collision thresholds, raw evidence or frozen seeds:

1. **Off-cadence finite-stop presentation.** Calibration `random-000` cleared an
   obstacle, then stopped `NoNominal` at actual handoff step 3130. Replay passed;
   the last periodic raw sample was step 3120. Rich-output validation incorrectly
   required a sample exactly at every finite endpoint. It now accepts only a
   proven finite stop with the expected last sampling tick and uses the exact
   saved final state as an explicitly labeled display-only observation. No
   simulator step, command, raw sample or proof component is added. All four
   first-wave random full non-timing flight records match the retry exactly.
2. **Exact shelf-vertex admission.** Held-out `random-034`, seed `1276303247`,
   has a stored source edge at height `7.765405875857499`. Interpolation from
   the preceding slope rounded that query by `8.881784197001252e-16` m, so a
   bitwise flat-shelf test rejected the input. Admission now uses the exact
   supplied vertex at an exact vertex coordinate, while retaining strict domain
   validation and exact interior-height checks. No tolerance is added and the
   terrain query/collision implementation is unchanged. Regression tests still
   reject a 1e-12 m non-flat vertex and out-of-domain pads. The same 100 seed
   order and all 35 profiles/scenarios saved before the failure are byte-identical
   in the retry. `random-034` then lands directly with both proof flags passing.

## Provenance and validation

The held-out capture is
`outputs/eval/planner_v2_random_terrain/capture-challenge-20261007-main-v2`.
It binds 123 source files at Git HEAD
`ffc59ec28613b35327c23edaeae0b586d4cdf085` plus the recorded dirty source snapshot;
HEAD alone is not its source identity. Final workspace source/executable checks
match that snapshot.

- Evaluator SHA-256: `cde404007b79c1cdc16450edcab2f8d4be5d59175d9fa1e38c697dd8bd257d89`.
- Manifest SHA-256: `a2bdf3d7bf7a5a7c36d5ed15b58e2018eb6dee9ad31fc2963d21188f9035b416`.
- Receipt SHA-256: `c1517aaff3f6ea0a35cf9b70e3003b10ea46aae5c62234c98ca0d9d83016e4ff`.

The maintained 11-step gate passes, including 220 evaluator library tests,
12 evaluator CLI tests, workspace/CLI boundaries, formatting, strict Clippy,
52 Node tests and documentation links. All 47 Python study tests and four
navigation tests pass. No retained 44-case numerical flight campaign was rerun
or relabeled. The three source-frozen preservation controls use the unchanged
comparison V2 contract; same-source repeats have no diagnostic exceptions.

Saved verification passes for all complete/stopped survey captures. All 496
protected prior files remain byte-identical, including the accepted selector
and report bodies, all previous stopped reports and the completed sanity site.
The 244 files sealed in the partial preparation failure also remain unchanged.
Native report publication independently validates each capture before rendering,
without new simulation or fresh replay.

Static report QA checks 170 pages, 141 rich details, 21 explicit stopped/unattempted
status details, 104 exact executed handoffs and 1500 resolving local links across
the three new sites and ordinary navigation. Eighteen rich details have labeled
display-only final observations; every raw sample still matches the plotted data.
Existing LAN serving returns HTTP 200 for both navigation and representative
batch/details. This is data/link validation, not browser visual or human acceptance.

## What to open and what comes next

From `/` or `/reports/`, choose **Waypoint planning → Harder terrain challenge**.
The accepted benchmark stays first; calibration, the sanity sweep and earlier
stopped captures are separately labeled. The common batch review tree and rich
trajectory/chart/sample views remain intact.

- Main batch: `/reports/eval/planner_v2_random_terrain/recheck-challenge-20261007-main-v2/`.
- `random-050`: four-handoff broad-massif landing, also repeated exactly.
- `random-054`: six-handoff broad-massif landing, not a repeat-tested case.
- `random-098`: three-handoff successive-ridge landing.
- `random-008`: two actual handoffs followed by an honest `NoNominal` stop.
- `random-000`: no departure; finite local clearing exhaustion, not a crash.

The recommended next pass is **diagnostic first, not another larger random sweep
or blind grid expansion**. Keep this population as a frozen development challenge:

1. Classify the 27 pre-departure `NoClearing` cases using saved terrain, conflict
   clocks and candidate traces. Separate early-entry/search coverage gaps from
   demanded climb/actuation outside the supported vehicle envelope. Include the
   steep local pad-transition geometry; do not assume every generated case is
   reasonably flyable.
2. Inspect the seven post-handoff `NoNominal` cases as a separate state-correction
   cluster, especially near-target forward energy. Any future simplification
   should preserve terrain-blind nominal construction and local clearing without
   demanding an end-to-end landing guarantee at every waypoint.
3. Only after identifying a bounded, physically plausible search blind spot,
   plan one small planner change and replay the unchanged challenge/control
   population. Keep the all-clear sanity baseline separate. Do not loosen safety
   guards, lift the nominal arc for terrain, tune individual seeds or increase
   the correction cap without an explicit progress/mission-time reason.

These are follow-up proposals, not automatically started work. No commit, push,
server start/stop or accepted-benchmark promotion was performed.
