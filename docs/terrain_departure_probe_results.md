# Fresh checkpoint and departure probe results

Date: 2026-10-08. Verdict: the retained early-exit planner carries useful coverage
to fresh terrain; the fixed half-lift/half-forward departure candidate adds no
valid handoff on either subject and is **not adopted**. Its core feature/template/
execution edits were removed after measurement. Retained source and executable
remain in the immutable experimental capture; production runtime and cap 6 are
unchanged. See the [bounded plan](terrain_departure_probe_plan.md).

## Fresh 100-world checkpoint

All 108 attempts close and verify: 100 new primary worlds, three separate
preservation controls and five exact complete non-timing repeats. The native
executable is the exact retained early-exit/cap-24 executable, not a new build.
Seeds exclude all 1230 previous shape/sanity/calibration/challenge/1k identities.
Terrain generation, four recipes, vehicle, Earth gravity, 120/60 Hz and local
original-height pad preparation are unchanged. No outcome filtering or tuning.

| Outcome | Fresh primaries |
| --- | ---: |
| Verified target landings | 77/100 |
| Clear direct landings | 35/35 |
| Blocked-route landings | 42/65 |
| NoClearing stops | 18 |
| NoNominal stops | 5 |
| Crashes, fuel stops, actual timeouts, cap stops | 0 |

All 23 finite misses remain physically flying and mission in progress. Every
attempt passes native integrity and original-source replay. The highest actual
correction count is nine. These results validate the existing candidate beyond
its inspected 1k development population; they do not establish arbitrary-terrain
reliability or an exact population rate. Do not combine 77/100 with the earlier
817/1000 denominator. These inputs are now observed validation data, not an
untouched sample for future behavior changes.

| Recipe | Landed / total | Blocked landed / blocked |
| --- | ---: | ---: |
| Mountains 4x | 24/25 | 7/8 |
| Mountains 8x | 19/25 | 14/20 |
| Broad massifs 8x | 20/25 | 11/16 |
| Successive ridges 8x | 14/25 | 10/21 |

Capture: `outputs/eval/planner_v2_random_terrain/capture-fresh-early-exit-20261008-v1`.
The first closed edition relies on the original receipt-pinned baseline for
retained-reference authentication; it was not modified or resealed when saved
verification was strengthened. Captured native and operator sources remain
distinct. Its generated rich `runs/<case>/report.html` files are not published.

## Fixed departure experiment

The isolated candidate retained cap 24 and the existing early-exit behavior for
old maneuvers. Only initial source-bridge exhaustions could invoke its new
seven-row family: existing powered durations split equally into upright lift
and +30-degree advance at the unchanged conservative acceleration cap, followed
by the existing idle coast and certified ordinary H. No extra entry clocks,
progress/clearance relaxation, landing-suffix requirement or per-seed conditions.
It did not query early exits for the new family.

All ten diagnostic primaries and fixed repeats `327/024/030` close and verify:
**13 measured missions**, no retries, no budget growth. The original nominal,
audit, conflict and entire old search prefix remain exact on the subjects. The
eight other primary records match the latest early-exit/cap-24 baseline completely
except explicit root policy/input identity and three wall timings. This preserves
all six current diagnostic landings, including `024/030`, which are no longer
failures in this baseline. Historical diagnostic outcomes were not rewritten.

| Subject | Admitted existing E clocks | Extra rows | Accepted H | Actual result |
| --- | ---: | ---: | ---: | --- |
| 327 | 3 | 21 | 0 | unchanged zero-step NoClearing |
| 791 | 3 | 21 | 0 | unchanged zero-step NoClearing |

All 42 new primary query traces terminate at body reserve below 5 m; none reaches
a supported progress/certificate boundary. In `327`, 12 end during coast, eight
during forward thrust and one while still requesting upright lift. In `791`,
13 end during coast and eight during forward thrust. The shorter burns lose
clearance during coast; longer burns encounter rising terrain during powered
advance. The upright-stage failure also demonstrates that existing horizontal
motion is not erased by requesting upright thrust.

The furthest recorded query stop is x=32.65 m versus required progress x=32.96 m
in `327`, and x=30.35 m versus x=34.93 m in `791`. These are query endpoints, not
executed crashes or measured waypoint positions. The progress threshold is the
existing body-sized distance past the first nominal conflict, not a terrain
feature's far edge. Removing that threshold would not repair the independently
observed body-clearance failures. The ordinary executed records for both subjects
remain exactly unchanged; no vehicle was launched and no new landing is claimed.

This rejects the **fixed 50/50 switch within the existing duration bound** on
these cases, not clearance-then-forward maneuvers in general or physical mission
feasibility. Following the predeclared negative gate, no 46-case expansion,
44-case flight campaign, fresh candidate population or 1k rerun was started.

Capture: `outputs/eval/planner_v2_random_terrain/capture-departure-probe-20261008-v1`.
Its full source, feature build command, binary, scenarios, baseline references,
native reports, queries, ledger and receipts are retained. The collector's `run`
entry fails before outputs/flights now that the rejected runtime is retired;
saved `verify` remains maintained and launches no flight or replay.

## Source review and validation

Before measurement, contract tests verified the seven exact stage splits,
unchanged old serialization, stage-command/clock rejection, actual turning and
incoming-state/fuel bindings. The all-features maintained 11-step developer gate
passed with the candidate code. Afterwards, all six owned runtime/Cargo files
were restored exactly to their pre-pass tracked bytes, so no rejected feature
or template remains in the ordinary workspace. Unrelated report/navigation work
was preserved. The final developer gate and all 112 terrain-study tests pass;
saved verifiers authenticate both captures with native subprocess execution
disabled. Accepted report selection/bodies and the retained release executable
remain unchanged. No commit, push, report publication or server operation.

## Evidence identities

- Fresh executable SHA-256:
  `54ffd58ac4b4e4a4e9a3d557725cfd03238f40a2bb226b8edbbdb85141271509`.
- Fresh manifest SHA-256:
  `52106ee9d8c0ac3442555091a03db8c982b8f565fb29ef1043f7007d78d4c7f6`.
- Fresh receipt SHA-256:
  `a83149bef965e9fc23ea1e8efca7f8f90b62dd0890dbee21f76406ab3dbf8455`.
- Departure experimental executable SHA-256:
  `cac4f58c78b62b7470ea784fcd4c0174fe1d03dcb13f13ccbf1c85bdc99ce2ab`.
- Departure manifest SHA-256:
  `941f648bba4d5ab67e2860cccff557e427e91d5a5b5fcd9b91b2f75da5ed3166`.
- Departure receipt SHA-256:
  `46b24a0f3d524ec11f2d87fc98fc197e7f6522a09a1bbfe5415c2a9879a5a0f3`.

## Next decision

Keep the demonstrated early-exit checkpoint. Do not expand the rejected fixed
split or chase another arbitrary lift/forward ratio. The next design question is
whether a simple terrain-derived clearance target can determine when to start
forward advance while accounting for existing horizontal drift and attitude
change. Inspect saved terrain/entry states first, preserve local progress and
continuation checks, and separate that design from late airborne acquisition.
A new implementation needs its own frozen small candidate and untouched test
population; no further campaign or default-cap promotion is selected here.
The subsequent [clearance design review](terrain_departure_clearance_design.md)
and [implementation plan](terrain_departure_clearance_plan.md) study this question
without new flights. They do not reopen this closed experiment's allowance.
