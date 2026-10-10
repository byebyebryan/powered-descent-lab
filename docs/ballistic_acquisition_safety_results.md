# Acquisition-gate and terminal-safety results

[Documentation home](README.md) · [Frozen plan](ballistic_acquisition_safety_plan.md) ·
[829/1000 reference](ballistic_recovery_consistency_results.md)

The independent acquisition-gate candidate lands **863/1000 (86.3%)**, versus
829: **37 gains and three losses**, a net gain of 3.4 percentage points. All
four terrain recipes improve. The independent terminal-safety candidate lands
**833/1000 (83.3%)**, with four gains and no losses. These are separate ablations
against V17, not a combined candidate or default promotion.

## What changed and what the experiment establishes

Acquisition-only checks the existing destination fit on the existing waypoint
coast query cadence regardless of estimated horizontal braking room. The room
estimate remains diagnostic. The existing native powered-prefix, coast rotation,
realized approach and ballistic reserve audits still decide admission. Failed
optional queries retain the active waypoint. No arc family, profile count,
waypoint placement/ranking, controller configuration or physical guard changes.

Terminal-only first preserves the nominal request and existing pad adapter.
After a native reserve rejection, it tries upright coast, then upright full
support, under the unchanged 24-tick terminal guard. The next pair reevaluates
the ordinary controller. Issued frames retain the original request and selected
fallback in metrics. The standalone coast-to-terminal branch stays unchanged.

The preceding diagnostic's opportunities were not landing promises. Nineteen
of 23 room-gated missing-aim worlds had a passing earlier query among five sampled
coast states. Actual acquisition-only flights land nineteen of those worlds,
but not exactly the same nineteen: sampling was not exhaustive, and safe
acquisition can still end in a later terminal stop. Twelve waypoint-construction
subjects had no passing sampled acquisition and remain stopped. Fifteen of 23
terminal stops had passing upright alternatives; only four complete landings
follow under the fixed terminal-only policy.

## Exact control and focused comparison

The source/native/reader/protocol freeze includes both explicit opt-ins and an
unchanged V17 control. All **106 control feedback files match the reference
byte-for-byte**, including the five repeats. The 101 primary worlds land 36.

| Focused candidate | Landings / 101 | Gains vs V17 | Losses vs V17 |
| --- | --- | --- | --- |
| V17 control | 36 | 0 | 0 |
| Acquisition gate | 69 | 33 | 0 |
| Terminal safety | 40 | 4 | 0 |

Acquisition gains nineteen missing-aim and fourteen terminal-command subjects.
Early acquisition changes the arrival state, so some terminal failures disappear
without changing the landing controller. Its focused ordinary flights remain
exact in 58/101 worlds: 28 earlier successes and thirty stopped worlds.
Terminal-only preserves all 36 successful ordinary flights; fifteen stopped
flights change, including the four new landings. All repeats verify exactly.

## Full paired original 1k

Both candidates execute all original worlds and five separate repeats, with
four workers per stage and unchanged 60-second attempt bounds. The full stages
overlap in wall-clock time; their elapsed times are not a controlled performance
benchmark. Acquisition collection takes 553.82 seconds, terminal collection
508.63 seconds. The slowest attempts are 4.63 and 4.41 seconds respectively.

| Terrain recipe | V17 / 250 | Acquisition / 250 | Acquisition gains / losses | Terminal / 250 |
| --- | --- | --- | --- | --- |
| Mountains, 4x | 210 | 215 | 5 / 0 | 210 |
| Mountains, 8x | 213 | 220 | 9 / 2 | 213 |
| Broad massifs, 8x | 213 | 224 | 11 / 0 | 215 |
| Successive ridges, 8x | 193 | 204 | 12 / 1 | 195 |
| Total | 829 | 863 | 37 / 3 | 833 |

Acquisition's 37 gains are 024, 056, 059, 167, 190, 271, 332, 357, 367, 378,
392, 426, 439, 488, 530, 547, 564, 576, 654, 662, 676, 683, 692, 696, 734,
781, 786, 814, 844, 850, 869, 870, 883, 902, 922, 943 and 971. The original
stop groups are nineteen missing aims, fourteen terminal-command rejections,
two prediction-domain stops and two original-budget stops.

The three losses are **283, 327 and 861**. All remain flying, centered over the
target and descending when the unchanged planning budget stops execution at
79.8 seconds. Their earlier reference flights land at 77.6, 76.6 and 63.225
seconds. Entering terminal control sooner did not make their new complete
descents faster. Do not discard the losses or count an unexecuted continuation.

| Loss | COM above upright touchdown height at stop | Vertical speed | Earlier / new terminal entry |
| --- | --- | --- | --- |
| 283 | 0.59 m | -1.71 m/s | 33.00 / 30.65 s |
| 327 | 11.37 m | -5.35 m/s | 34.50 / 29.87 s |
| 861 | 2.11 m | -2.62 m/s | 49.80 / 40.73 s |

These height differences use the pad surface plus the upright touchdown offset;
they are not rotated-body clearance or proof of eventual landing. This pass
does not extend the budget, continue the stopped missions or tune around them.

Terminal-only gains **538, 650, 796 and 883**, with no losses. It issues 274
fallback frames across fifteen worlds; eleven still stop later. All 1,000
preterminal entry clocks and command prefixes match V17, and 985 complete
ordinary flights remain exact, including all 829 earlier landings. Acquisition
retains 597 complete ordinary flights exactly, including 474 earlier wins.
The broader trajectory change therefore still needs generalization review.

There are no executed crashes or off-target landings in either candidate.
Acquisition has 200 landings after actual H and 663 without H; terminal-only has
444 after H and 389 without H. Zero-H does not mean uninterrupted direct flight:
local recovery or waypoint-directed acquisition can occur without an actual
handoff being committed. Original baseline cohorts stay separate: acquisition
lands 344/387 originally clear and 519/613 originally blocked worlds.

Against the older policy-3 cap-24 817 reference, acquisition gains 145 different
worlds and loses 99, while terminal-only gains 142 and loses 126. Neither is
strict dominance or accepted replacement evidence for that different planner.

## Remaining stops and next decision

| Stop group | V17 | Acquisition | Terminal safety |
| --- | --- | --- | --- |
| No safe recovery command | 69 | 69 | 69 |
| No ballistic aim | 41 | 20 | 41 |
| Waypoint construction miss | 21 | 21 | 21 |
| Terminal short-command rejection | 23 | 11 | 19 |
| Original budget | 13 | 14 | 13 |
| Prediction terrain domain | 2 | 0 | 2 |
| Local proposal budget | 2 | 2 | 2 |

Acquisition leaves 137 protected stops: 112 preterminal and 25 terminal. The
69 recovery failures now account for roughly half of the remaining population;
they and the 21 waypoint misses are not repaired by this gate change. 407 and
715 remain exact stopped ordinary flights, while 349 remains an exact landing.

**Use acquisition-only as the stronger opt-in development reference.** The
horizontal-room trigger was a demonstrated missed-opportunity gate, not a reason
to add more terrain-height or arc-profile tuning. The terminal fallback is a
small, independently validated protection improvement, not a landing-controller
solution. No combination was run, and its result cannot be inferred by adding
the separate gains.

Before promotion, choose a separate combination/preservation and held-out pass.
Review the three deadline regressions and the original reserve-accounting policy
before tuning their trajectories. Further recovery work should investigate
earlier response capability and actual command sequences, not simply shorten
the guard or prescribe lift-first. The existing remaining waypoint/715 mechanisms
stay separate. This is development reuse, not fresh generalization, real-time
game-loop acceptance or arbitrary vehicle/gravity coverage.

## Evidence and validation

Five create-only captures are
`capture-ballistic-acquisition-safety-{control,acquisition-focus,terminal-focus,acquisition-full,terminal-full}-20261010-v1`
under `outputs/eval/planner_v2_random_terrain/`. All **2,328 declared records**
verify: exact original inputs, complete physical/mission/integrity/source-replay
tuples, reproduced decisions, exact repeats and full focused overlap. Common
rich batch/detail reports remain; no stripped-down presentation is introduced.

An independent audit authenticates each full 13,287-file inventory, all 2,010
full records/repeats, both 101-case overlaps, complete replay states, unchanged
initial arcs, all 223 frozen source files and 62 protected paths. Plant, pure
planner and controller dependencies match the V17 source in all 35 declared
files. The original reference inventory and existing navigation/server sources
remain unchanged. JSON object key order is not numeric state equality; empty
optional terminal-frame arrays remain correctly absent on preterminal stops.
This is automated artifact/data validation, not manual browser acceptance.

Scoped ballistic feedback tests pass 55/55; terrain-tool tests pass 199/199. The
ordinary maintained eleven-check developer gate passes, including 69 Node tests,
formatting, strict Clippy, CLI boundaries and docs links. Explicit exact numerical
parity passes for all 44 bound default-planner inputs against the later
`capture-early-exit-20261007-native` source checkpoint. The older published
October-5 capture still fails at its documented approximately 9e-16 first-clearance
scalar difference. Its comparator, source attribution and historical result
remain unchanged; do not describe that older parity check as green.

Read-only saved verification:

```sh
rtk proxy python3 -B studies/terrain_profiles/ballistic_acquisition_safety_panel.py verify acquisition-full
rtk proxy python3 -B studies/terrain_profiles/ballistic_acquisition_safety_panel.py verify terminal-full
```

```text
native:              20a2f81086700e158fa17f5878a23a67365c1a12f38cade7daa15a11910cc169
Rust tree:           639425a6c7b495dea84330a3513ebf0d2a3ad90459654be3cf1f6f1ac8f78a24
acquisition receipt: c673075623673007dfd839cccf45a734991a1a5e68fb9fa2e2f838ebce2fd973
acquisition results: dc75abe4c82975afd1a2ed2c4a7870eddbc51bf4cf6b2facd11d247477babea9
terminal receipt:    83623257f676ec635504c7f490c822412a6191614bba1ac860756e9236b22f28
terminal results:    76da156d5da5e1e821051e0b0ef31f489c81b7dcac886a23b0f43ae8df153989
```

No commits, pushes, default/accepted-selector promotion, historical report
rewrites, manual root publication or server lifecycle changes were made. The
already-authorized dynamic library can discover the new report captures; its
generated navigation is separate from retained flight evidence. All pre-existing
navigation work remains preserved for a separately requested review/commit.
