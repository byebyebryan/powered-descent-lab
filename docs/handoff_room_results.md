# Handoff braking-room preference results

Date: 2026-10-07. Completes the [approved bounded plan](handoff_room_plan.md)
over the exact saved challenge worlds. All 171 campaign evaluations completed:
eight logging-only shadow cases, 17 candidate focus cases, 44 unpublished
benchmark cases, 100 challenge cases and two fixed repeats. The allowance is
closed; these results do not authorize another campaign.

## Verdict

Keep the small one-sided preference. Challenge landings improve from **65/100
to 68/100**, with all 65 previous successful complete flights preserved. The
three recovered cases are `028`, `029`, `060`; all have the full physical,
mission, integrity and final-source-replay success tuple. There are no crashes,
fuel-exhaustion stops, timeouts, integrity failures or source-replay failures.

This is useful incremental coverage, not a generalized recovery solver or proof
of arbitrary-terrain reliability. The reused challenge is a development corpus,
not a fresh held-out estimate. The separate easy 100-case direct-flight sanity
sweep and accepted 44-case report remain unchanged.

## What changed and what did not

The old ranking prioritizes later entry E, then earlier eligible handoff H,
lower fuel burn and row ID. It could choose a fast obstruction-clearing handoff
with too little horizontal distance left to brake, despite another already
accepted row leaving more room.

After completing the existing stage, replace its original winner only when its
estimated room is negative and an already accepted nonnegative-room row exists.
Use the original rank within that subset, not maximum room or weighted scoring.
If neither condition holds, preserve the original winner. The pure estimate
uses current handoff mass, the existing nominal 0.925 thrust fraction, gravity-
supporting lateral braking and forward travel during a held-pair-rounded turn.

Keep terrain-blind nominal construction, primary/failure-only timing stages,
42 templates, first eligible H per row, reserve and continuation guards,
six-correction cap, actual-handoff replanning and original-source replay.
No candidate is rejected on room, no landing suffix is requested per candidate,
and a negative estimate cannot open timing fallback. Vertical energy, terrain
and braking fuel consumption remain outside this cheap scalar heuristic.

Actual accepted query states and estimates are additive row diagnostics. The
common rich detail template adds a column to its collapsed query table; plots,
views, annotations and batch trees are retained. Old metadata-free records
round-trip unchanged. Query states remain distinct from executed handoffs.

## Gates and outcomes

The shadow preserved all eight complete flights exactly, excluding only the
three wall timings and additive query diagnostics. Actual query states confirmed
the predicted six switches. `090` has no nonnegative alternative; successful
`064`, with only 0.730 m estimated room, remains unchanged. No comfort threshold,
extra multiplier or parameter tuning was introduced.

The focus improves from six to nine verified landings. All six successful
controls (`034`, `050`, `054`, `023`, `043`, `064`) remain complete-flight exact.
All 17 focus records match their full-challenge records, including diagnostics,
apart from wall timings.

The benchmark passes all 36 mandatory landings: 11 direct and 25 corrected.
It retains two diagnostic landings, three diagnostic `NoClearing` stops, one
diagnostic `NoNominal` stop and two unsupported inputs. All 44 pass integrity
and all 42 supported cases pass source replay. All 38 old benchmark landings
are exact; an additional read-only comparison found all 44 complete records
unchanged apart from wall timings and query diagnostics. No historical numerical
comparison exceptions are needed against this already-repaired baseline.

Across the 100 challenge worlds, 94 complete flights remain exact under the same
narrow exclusions. Only `008`, `028`, `029`, `060`, `066`, `094` change, each
selecting one different handoff. All 36 clear routes remain direct landings;
blocked-route landings improve from 29/64 to **32/64**. Per recipe, each over
the same 25 worlds:

| Recipe | Fallback baseline | Braking-room preference |
| --- | ---: | ---: |
| Mountains 4x | 22 | 22 |
| Mountains 8x | 15 | 17 |
| Broad massifs 8x | 19 | 20 |
| Successive ridges 8x | 9 | 9 |

Both fixed repeats, `028` and `050`, are exact including diagnostics except the
three wall timings. All 102 challenge/repeat attempts pass integrity and replay.

## Limits exposed, not tuned away

- `028`: handoff forward speed falls from 94.71 to 37.64 m/s; the regenerated
  nominal lands after the same three corrections.
- `029`: 69.43 to 45.59 m/s; lands after the same two corrections.
- `060`: 46.48 to 36.49 m/s, with reduced downward speed; lands after one
  correction. Its small negative original estimate was enough to trigger the
  unchanged zero-boundary rule.
- `008`: gets another feasible nominal and executes a third correction, but its
  later handoff still exhausts nominal acquisition. This is not a landing gain.
- `066`: establishes a nominal from the slower handoff, then encounters another
  obstruction and exhausts local clearing. It changes `NoNominal` to
  `NoClearing`, not success.
- `094`: horizontal room becomes positive but downward speed worsens from
  61.49 to 67.92 m/s. Nominal acquisition still fails. Room is demonstrably not
  a sufficient landing-feasibility criterion.

The remaining 32 outcomes are finite stops: 22 `NoClearing`, seven `NoNominal`,
three `CorrectionLimit`. None proves physical impossibility. The original 21
local-exhaustion cases are unchanged; further vertical, nominal or local-family
changes were outside this pass.

## Cost, source and evidence

Total challenge query rows increase from 30,198 to 30,534 because `008` and `066`
progress into another search, not because the family expands. The observed
maximum remains 1,176 rows per flight, below the 2,016-row bound. Planning time
averages 0.926 s versus 0.908 s in the saved baseline; current median is 0.848 s,
maximum 2.787 s. These four-worker observations are not a controlled overhead
benchmark or game-loop latency guarantee.

Final evaluator SHA-256:
`d34f9bfe91126face2ed550a4d44056683ad79fce7b7753683d0eb61d7cfd826`.
Logging-only shadow SHA-256:
`1bf9a8afdc6e118f3e6b255472d906c4f10384ff185b8070a853021c421c02ef`.
Survey captures authenticate 132-file source inventories and unchanged original
scenario bytes. Focus, benchmark audit and challenge bind the same final source
and executable; current working source matches that freeze.

Retained roots:

- Shadow: `outputs/eval/planner_v2_random_terrain/capture-handoff-room-20261007-shadow`.
- Focus: `outputs/eval/planner_v2_random_terrain/capture-handoff-room-20261007-focus`.
- Benchmark: `outputs/eval/planner_v2_lab_suite/capture-handoff-room-20261007-benchmark`.
- Benchmark audit/receipt:
  `outputs/eval/planner_v2_random_terrain/capture-handoff-room-20261007-benchmark-preservation`.
- Challenge/repeats:
  `outputs/eval/planner_v2_random_terrain/capture-handoff-room-20261007-challenge`.

The maintained 11-step development gate passes on shadow and final behavior,
including Rust/CLI tests, Clippy, formatting, 52 Node tests and documentation
links. All 55 terrain-study Python tests pass. The saved collectors verify the
bounded work, estimate arithmetic, selection rule, complete-flight preservation,
source inventories and same-source repeats. The native benchmark checker passes.

All 48 protected accepted selector/report files remain hash-identical. No
commit, push, server operation, accepted promotion, navigation refresh or
separate site publication occurred. Review/commit is the next checkpoint;
further behavior work requires a separately scoped decision. Do not turn this
small preference into a tuned multi-dimensional recovery solver automatically.
