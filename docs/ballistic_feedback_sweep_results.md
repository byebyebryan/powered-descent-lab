# Ballistic feedback paired 1k diagnostic — 2026-10-08

[Documentation home](README.md) · [Construction results](ballistic_feedback_results.md)
· [Reviewed design](ballistic_aim_correction_plan.md)

## Verdict

The unchanged opt-in candidate lands **360/1000** original procedural worlds:
235 with zero actual handoffs, 109 with one, and 16 with two. State-based ballistic
correction and waypoint-to-waypoint execution can work beyond the construction
panel. This candidate is nevertheless a substantial coverage regression from the
retained early-exit experiment's **817/1000**. Do not promote it.

All 640 remaining cases stop with physical outcome `flying` and mission outcome
`in_progress`. There are no physical crashes, but this is not proof that rejected
commands or unexecuted suffixes would have been safe. Eighty-three cases stop
before the first physics step. A finite constructor miss is not a proof that the
world is impossible.

The full paired acceptance campaign remains deferred. This separately authorized
run is broad diagnosis of the construction candidate, not a claim that it passed
the maintained 44-case pack, easier/harder sets, separate 100-world validation,
portable CLI acceptance, or a new untouched population. Policy 3, accepted
selectors, published report bodies and existing navigation remain unchanged.

## Collection contract and evidence

The [frozen diagnostic contract](../studies/terrain_profiles/ballistic_feedback_sweep_plan.json)
and [collector](../studies/terrain_profiles/ballistic_feedback_sweep.py) retain
all 1000 original scenario byte streams, seeds, recipe assignments and baseline
results from `capture-early-exit-sweep-20261008-v2`. Its saved verifier passed
before collection. No terrain, vehicle, gravity, fuel, deadline or physics/control
cadence was tuned. No flight candidate was rebuilt or changed.

The cap is 24; four workers collect in bounded waves; each invocation has a
60-second wall bound and the campaign a 7200-second bound. The complete 1000
primary plus two external repeat attempts finish without retries or collection
errors. The native invocations each additionally reproduce planning decisions
and replay actual commands from the original source. External repeats of `000`
and `715` have identical complete `FeedbackResult` records, not just matching
outcomes. Repeats do not inflate the primary denominator. Flight collection
takes approximately 165 seconds, excluding preparation, reporting and verification.

Capture:
[`capture-ballistic-feedback-20261008-v1`](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/).
The [local batch index](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/index.html)
uses the common report shell/tree/actual-flight SVG preview and links the
unchanged rich native detail reports. Failure groups come first; repeats are
separate. This is an isolated diagnostic capture, not a report-site publication.
The existing outputs-root server can serve it without a server change.

Retained hashes:

- Flight executable: `2ffa627cdc2cc98df224a8aadafc7859e9d6ab1dec32c5afa9a267079e7cc6e7`.
- Rust tree/manifests: `e86de0959ea7e5c84d7a67ceb4992bf2fce8133d500f12f9b03809be19f2ecbf`.
- Report script: `0dd790899fa3aa4078857ad16d16e60ec17b5b775bdaf404645ff1380bd361b0`.
- Capture receipt: `7fad761233c0a475f450f46a4e0dd81729ed5f37ef4ca9295118e306ee9e5e80`.
- Results: `3affd91d0f42531e5d1ccb6d475f9e06da555d90e3fe58bd352bf858e2d75297`.

The source/executable snapshots and renderer binary are inside the capture.
Every case binds input bytes, candidate/source/report-script identities, raw
execution, complete final state, command replay, deterministic decisions and
native stdout. Saved verification checks the full receipt and re-derives all
summary projections without launching flights or altering outputs.

## Paired outcomes

Of the 817 baseline successes, 350 remain successes and 467 are lost. Ten former
misses now land, for a net loss of 457. This comparison changes the entire flight
algorithm; it is not an isolated effect of the shared numeric cap 24.

| Original baseline cohort | Worlds | Candidate landings |
| --- | ---: | ---: |
| Clear nominal | 387 | 253 |
| Blocked nominal | 613 | 107 |

These are **baseline** classifications. The candidate's first desired arc is
clear in 536 cases and blocked in 464, reflecting different arc construction and
the actual state when first planning. Zero-H landings must not be substituted for
the old clear-nominal cohort. A first clear ideal arc can still need later
correction when the real vehicle drifts while acquiring it.

| Terrain recipe | Worlds | Candidate landings |
| --- | ---: | ---: |
| mountains_4x | 250 | 122 |
| mountains_8x | 250 | 86 |
| broad_massifs_8x | 250 | 103 |
| successive_ridges_8x | 250 | 49 |

Coverage weakness spans all four recipes. The only physical-terminal outcomes
are the 360 target landings, all with mission success, integrity, source replay
and deterministic-decision reproduction. Every landing finishes under the
unchanged maintained terminal controller; ballistic aiming is not a claim that
contact itself is unpowered.

## Remaining finite stops

| Recorded stop mechanism | Cases |
| --- | ---: |
| Short held-command prediction rejected | 187 |
| No finite ballistic aim | 130 |
| Actual waypoint coast continuation rejected | 111 |
| Newly generated waypoint arc still blocked | 101 |
| Active waypoint arc becomes blocked | 95 |
| Cannot construct acquisition of proposed waypoint | 16 |

These counts are mutually exclusive and total 640. No case exhausts the hop cap;
maximum actual handoffs is three. A larger hop budget does not address this result.

Useful observed distinctions, derived from the saved refresh/update records:

- Of 187 short-command stops, the last executed phase is correction in 112,
  coast in 74 and maintained terminal in one. Thus 186 happen before terminal
  operation. There are 141 last waypoint-goal refreshes and 46 destination-goal
  refreshes. This does not identify the rejected next command's exact phase or
  prove every reserve violation has the same root cause.
- Of 130 no-aim stops, 97 have already executed at least one H; 33 have none.
  The last refresh targets the destination in 96 and a waypoint in 34. The late-H
  problem is widespread, but it is not the only kind of constructor exhaustion.
- All 95 active-waypoint blocked stops occur before the first H. The current
  candidate stops rather than finding another bounded local waypoint.
- All 111 continuation rejections follow an executed coast phase, before the
  new H is admitted; 100 have no previous H. Reaching the waypoint region is not
  sufficient to establish the actual two-second continuation.
- Of 101 newly generated blocked-arc stops, 83 have no physics execution.
  These are pre-departure planning misses, not takeoff collisions.

## Suggested review order

Open the batch tree's landing branches first to verify what is already working:

1. [`001`](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/runs/random-001/report.html):
   zero H, landed at 52.958 s.
2. [`006`](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/runs/random-006/report.html):
   one actual H, landed at 57.008 s; first desired arc was clear, so review the later
   state-aware replan rather than inferring launch blockage.
3. [`142`](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/runs/random-142/report.html):
   two actual H, landed at 34.175 s; first desired arc was blocked.

Then compare three distinct misses without treating them as crashes:

1. [`030`](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/runs/random-030/report.html):
   flying stop at tick 882, 7.35 s; predicted body reserve 4.894548 m versus 5 m
   at tick 906. The rejected prediction is not an executed collision.
2. [`715`](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/runs/random-715/report.html):
   two H, then no target aim at tick 3188, 26.567 s. It reproduces the final
   construction result without benefiting from cap 24.
3. [`000`](../outputs/eval/planner_v2_random_terrain/capture-ballistic-feedback-20261008-v1/runs/random-000/report.html):
   initially clear desired arc; later proposed waypoint cannot be acquired;
   flying stop at tick 614, 5.117 s.

## Recommended next decision

Keep the aiming/correction math, live propagation, cutoff, simple destination
safeguards and unchanged landing adapter. The 125 waypoint landings are positive
evidence for the direction, not justification for promoting the present rules.

Review two shared mechanisms before more implementation: (1) a cheap useful
waypoint exit-state bound, without demanding a landing suffix, and (2) a bounded
local replan when an active arc or short actual-command prediction becomes unsafe.
Do not simply raise every waypoint, relax reserves, increase the hop cap or add
seed-specific exceptions. Use successful/failing state contrasts from this full
population, rather than diagnosing every miss as a launch-clearance failure.

This diagnostic run did not change those mechanisms. Any next implementation
needs a separate bounded plan and paired regression; the separate observed
100-world sample was not touched in this pass.

## Validation

The ordinary 11-check maintained developer gate passes, including strict Clippy,
formatting, workspace/CLI boundaries, 59 Node tests and docs links. Terrain-study
unittest discovery passes 127 tests, including six new projection/denominator
tests. The new saved-only example renderer's test passes; it does not invent
flight previews for unattempted cases. Complete saved-bundle verification passes
1002/1002 recorded attempts with exact external repeats and no integrity/source
drift. HTTP checks return 200 for the batch and representative two-H detail via
the existing server. These are data/template/reachability checks, not browser
pixel acceptance or human approval of the visualization.
