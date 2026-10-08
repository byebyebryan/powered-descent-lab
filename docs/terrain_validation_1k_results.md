# Fresh 1,000-case planner coverage results

Date: 2026-10-07 (local). Completes the [approved validation plan](terrain_validation_1k_plan.md).
All 1000 fresh worlds, three preservation controls and five fixed repeats were
collected and verified: **1008/1008 attempts**, with no retry or replacement.
The allowance is closed. No planner/controller behavior changed in this pass.

## Verdict

The current planner's coverage carries over to unseen seeds under these four
terrain recipes: **733/1000 verified landings (73.3%)**, including **346/613
initially blocked routes (56.4%)**. Every initially clear route lands directly:
**387/387**. Landing means `landed_on_target`, mission success, `landed` planning
stop, integrity passed and original-source replay passed together.

This is evidence against a severe fresh-seed coverage collapse, not proof that
each recent heuristic generalizes or caused an improvement. The previous
100-case corpus was repeatedly used for development; the new population is a
different sample, not a paired experiment. Its numerically higher rate should
not be interpreted as a gain produced by this evaluation-only change.

| Measure | Development population | Fresh population |
| --- | ---: | ---: |
| All worlds landed | 68/100 (68.0%) | 733/1000 (73.3%) |
| Initially clear routes landed | 36/36 (100%) | 387/387 (100%) |
| Initially blocked routes landed | 32/64 (50.0%) | 346/613 (56.4%) |
| Initially blocked share | 64.0% | 61.3% |

The clear/blocked mix differs despite equal recipe allocation. The blocked-route
rate also holds up, so the overall result is not merely more clear routes.
Nevertheless, **267/1000 worlds remain unsolved**. This is useful coverage, not
reliable handling of arbitrary terrain or an estimate restricted to worlds known
to be physically feasible.

## Recipe coverage

Each recipe has 25 development worlds and 250 fresh worlds. All recipe-specific
clear routes land directly; the blocked columns show actual corrected landings.

| Recipe | Development landings | Fresh landings | Fresh blocked landings |
| --- | ---: | ---: | ---: |
| Mountains 4x | 22/25 (88.0%) | 230/250 (92.0%) | 67/87 (77.0%) |
| Mountains 8x | 17/25 (68.0%) | 167/250 (66.8%) | 96/179 (53.6%) |
| Broad massifs 8x | 20/25 (80.0%) | 203/250 (81.2%) | 95/142 (66.9%) |
| Successive ridges 8x | 9/25 (36.0%) | 133/250 (53.2%) | 88/205 (42.9%) |

Successive ridges remain weakest, accounting for 117 of the 267 misses. The
small development sample understated their measured coverage here, but they
still expose the largest obstacle-handling gap. Mountains 8x does not show an
across-the-board increase; this is another reason not to claim paired progress.

The generated worlds retain the original recipe scales, resolved 4 m polylines,
1200 m route, original-height 36 m pad shelves and local 24 m transitions. No
floor cutaway, seed filtering or difficulty quota was added. Across the fresh
sample, full-profile relief ranges from 113.23 to 1493.06 m, target elevation
change from -1055.47 to +914.58 m, and maximum terrain height above the endpoint
chord reaches 1104.09 m. These geometry descriptors are not feasibility proofs.

## What remains unsolved

| Finite stop | Fresh count | Where it occurs |
| --- | ---: | --- |
| `NoClearing` | 149 | 65 before departure; 84 after at least one correction |
| `NoNominal` | 95 | All after at least one executed correction |
| `CorrectionLimit` | 23 | After the existing six-correction limit |

All 1000 initial states establish a nominal and classify it as clear or blocked.
The 95 `NoNominal` results are later airborne replanning failures, not failure to
construct the initial pad-to-target flight. Of the 267 unsolved worlds, **202
(75.7%) already executed at least one correction**. Local progress alone does
not reliably produce a state from which the remaining route can be completed.
These counts locate the gaps; they do not identify a single cause or prove
that an alternative maneuver would succeed.

There are no crashes, fuel-exhaustion stops, timeouts, collection failures,
integrity failures or original-source-replay failures. The stopped physical
records remain `flying` with mission `in_progress`; they are not landings or
proofs of physical impossibility. Finite search exhaustion remains in the
primary denominator rather than being filtered out.

The planner executes 1118 corrections across 548 worlds. Of the 346 corrected
landings, 185 use one correction and **161 use two to six**. Successful correction
counts from zero through six are respectively `387, 185, 81, 38, 23, 10, 9`.
The piecewise clear-and-replan loop genuinely works on unseen terrain, including
multiple handoffs; neither universally raising the direct arc nor requiring a
complete landing suffix at every local candidate was introduced.

## Collection, preservation and verification

Master seed `2026100703` selects 250 worlds per unchanged recipe. The 1000 unique
seeds have no overlap with the actual earlier shape, sanity, calibration or
development seed pools. The ordered seed list was saved before generation.
Separate-process generation in reverse traversal reproduced identical bytes;
all 1000 scenarios passed input-only preflight before measured flights.

The direct, ridge and plateau preservation controls all land with respectively
zero, one and three corrections. Complete non-timing comparisons against the
authenticated current benchmark are exact, with **no comparison exceptions**.
Fixed repeats `000`, `250`, `500`, `750`, `001` are complete-record exact except
the three wall timings. Repeat `750` correctly reproduces its finite
`NoClearing` stop rather than being counted as a landing. Controls and repeats
are not part of the 1000-world denominator.

The runner's final verification, an additional read-only Python verification and
native `check-terrain-survey` all pass. The native checker applies the same saved
evidence contract as rendering, but writes no report and executes no new flight
or replay. The complete receipt authenticates 10217 files. Source inventories
before and after collection are identical; all 133 selected source files still
match the freeze, as does the evaluator. All 48 protected accepted
selector/report files remain hash-identical.

Only evaluation plumbing changed: a separate 1k contract/seed stream, explicit
contract admission in the collector and native verifier, pure study tests, and
a read-only native verification command. The previous 132-file source inventory
differs only in those five evaluation/test files; the new contract is the sole
additional selected source file. Planner, controller and simulation source are
unchanged. Historical plan files and captures are preserved.

## Cost and retained evidence

Primary-flight planning time averages 0.832 s, with median 0.780 s, 95th
percentile 1.987 s and maximum 2.702 s. There are 248052 local query rows, a
maximum of 1176 per flight. These are four-worker wall-time observations,
not a controlled speed comparison or a game-loop latency guarantee.

Capture root:
`outputs/eval/planner_v2_random_terrain/capture-validation-1k-20261007-v1`.
It retains exact inputs, source snapshot, preflights, controls, ledger, full
flights, common rich detail reports, survey and receipt (about 5.1 GiB).
The comparison population remains
`outputs/eval/planner_v2_random_terrain/capture-handoff-room-20261007-challenge`.

- Evaluator SHA-256:
  `fdbf20a694de3ac82b79492d119fe850e4bfea5d3c4c70a3ca854a72cfe5d2af`.
- Manifest SHA-256:
  `c0a844f772f45dd2e39e362622e81e4cf5ceb04831925cdc492dd3e4989e2ee8`.
- Receipt SHA-256:
  `0ae5032ac69f4428f3e79649deafed76dfefedeca1139dd4bbdc3e8fc5b22f21`.
- Recorded Git base: `41506b8`; the dirty-source inventory, not HEAD alone,
  binds this evaluation-only implementation.

All 11 maintained development checks and all 57 terrain-study Python tests pass
before the freeze. Closure documentation is checked separately without more
flights. No commit, push, report-site publication, selector change or server
operation occurred. Local detail reports are capture evidence, not a newly
published navigable batch site.

## Recommended next decision

Keep the terrain-blind direct / local clearing / actual-handoff replanning
architecture. The fresh sample supports it as a useful gamified baseline, while
confirming a substantial local-clearing and post-handoff gap. More tiny threshold
patches or simply increasing the correction cap are not justified by these
counts alone.

Before another behavior change, inspect a small, balanced sample of saved
`NoClearing` and post-handoff failures across recipes, alongside successful
controls. Use that to decide whether one simple, geometry-driven clearing
maneuver or a more useful handoff can address a broad failure class. Avoid
per-seed rules, an end-to-end recovery solver or an exhaustive maneuver-family
expansion without evidence. This is a recommendation, not a new campaign or
implementation allowance. The inspected 1k population becomes development data
for future changes; a later generalization claim needs another untouched sample.
