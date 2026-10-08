# Random procedural-terrain survey — stopped results

[Documentation home](README.md) · [Frozen plan](random_terrain_survey_plan.md) · [Terrain refinement](terrain_ridge_refinement_results.md)

Subsequent work implements the [numerical fix and bounded recheck](random_terrain_survey_recheck_plan.md);
the separate [recheck results](random_terrain_survey_recheck_results.md) stop on a
preservation diagnostic mismatch before any random flights.
The results below remain the original stopped capture's evidence; no rerun or
retroactive proof acceptance is claimed.

## Verdict

The collector and separate common-template report are implemented. The measured
survey stopped correctly after seven attempts: three preservation sentinels and
the first four random cases. Three random flights are verified direct landings.
The fourth physically landed, but its final accumulated replay failed its initial
geometry guard. It is **not** a verified landing.

This is not a physical first-step crash, and it does not establish broad random
terrain coverage. A reproducible floating-point error in the symmetric body
envelope prevents completion of the approved survey. No planner/controller fix,
retry, replacement seed, additional flight, commit or push was made afterward.

## Accounted sample

| Cohort | Frozen allowance | Attempted | Verified target landings | Remaining |
| --- | ---: | ---: | ---: | ---: |
| Random primary cases | 100 | 4 | 3 | 96 unattempted |
| Preservation sentinels | 3 | 3 | 3 | 0 |
| Predeclared repeats | 5 | 0 | 0 | 5 unattempted |

All 100 primary identities remain in the saved report. The random dispositions
are three `recorded`, one `runner_error` and 96 `not_attempted`. This is an
accounting denominator, **not a 3% landing rate**. The first-ten checkpoint was
not reached, and no repeatability claim for random flights is available.

- `random-000`, seed 160502033: verified direct landing, zero corrections.
- `random-001`, seed 1634476889: physical target landing, final replay rejected;
  no verified result projection or accepted flight manifest.
- `random-002`, seed 1332011433: verified direct landing, zero corrections.
- `random-003`, seed 1019237408: verified direct landing, zero corrections.

The three sentinels reproduced their complete saved non-timing records:
`v2_clear_685` (zero corrections), `v2_ridge_early` (one), and
`v2_plateau_reference_900` (three). This preserves existing direct/single/multiple
handoff behavior; it is not evidence of new procedural waypoint coverage.

## Why the replay stopped

The affected live flight completed at 41.35 s / physics step 4962, on target with
mission success and zero corrections. Its nominal contact audit passed, and its
poststep clearance scan passed. The final proof then ran the accumulated command
ledger from the original source. Its geometry guard rejected the initial state
before consuming any of the 2481 commands:

```text
stage: InitialGuard
boundary_physics_step: 0
segment reserve -0.000000000000003552713678800501 below 0 at 0
planning_stop: implementation_error
integrity_passed: false
final_source_replay_passed: false
```

The source pad is flat at 23.206565037797983 m. The upright vehicle center is
28.206565037797983 m, with a 5 m foot offset and 10 m hull height. Direct foot
clearance is exactly zero. Reproducing the arithmetic of
[`body_aabb_from_pose`](../pd-eval/src/planner_flight/geometry.rs) on that saved
input, without simulation or replay, gives:

| Calculation | Result (m) |
| --- | ---: |
| Feet minus source surface | 0 |
| Lower world-coordinate extent | 5 |
| Upper world-coordinate extent | 5.0000000000000036 |
| Symmetric envelope extent (maximum of both) | 5.0000000000000036 |
| Envelope bottom minus source surface | -3.552713678800501e-15 |

Adding 5 m to the center crosses a floating-point spacing boundary; subtracting
the center from that rounded top produces a slightly oversized upper extent.
The symmetric envelope uses that extent below the center too, inventing the
microscopic penetration. [`exact_point_clearance`](../pd-core/src/terrain.rs)
returns the negative reserve, and
[`FlightGuard`](../pd-eval/src/waypoint_v2/execution.rs) strictly rejects
`clearance < required`, with required reserve zero in the source pad corridor.
The reproduced value matches the recorded failure exactly.

The live nominal audit scans **poststep** states; its first airborne reserve was
positive. The accumulated proof also checks **step zero**. That difference
explains why a live landing and passing nominal audit coexist with this failed
final proof. The runner retained the failed native bundle/logs, drained the other
three workers in the wave and launched no further waves.

## Inputs, provenance and preservation

The master seed is 2026100601. All 100 fresh seeds were saved before generation.
The unchanged refined Pylander composition, ridge slice 67.37, 4 m authoritative
polyline, 1200 m separation and fixed local pad patches were used. The vehicle,
Earth gravity, 120/60 Hz clocks and 90 s mission horizon were unchanged. There
was no flight corridor cutaway or terrain-aware arc/recipe tuning.

All 100 native input-only preflights passed. Input support does not imply flight
feasibility. Relief spans approximately 40–151 m; endpoint rise spans roughly
-100 to +84 m. Every vertex outside the two pad patches is unchanged. A fresh
process with reversed generation order reproduced the complete raw arrays.

The first preparation root `capture-random-20261006-v1` contains zero measured
flights. Developer review found command-registry and navigation-test assumptions;
these were corrected before the final source freeze. All 100 raw arrays and 103
prepared scenarios are byte-identical between that root and the measured root.

The canonical measured root is
[`capture-random-20261006-v1-reviewed`](../outputs/eval/planner_v2_random_terrain/capture-random-20261006-v1-reviewed/survey.json).
It contains full source snapshots, frozen inputs, preflights, controls, seven
attempt bundles/logs, all 108 accounted rows and hash receipts. Measured source
and executable identities matched before/after. The source manifest binds 113
files at base commit `ffc59ec28613b35327c23edaeae0b586d4cdf085` plus the reviewed
uncommitted survey changes; the base commit alone is not the measured source.

- Executable SHA-256: `21e48f41d4d6b13f113ae8705de54f36b97f331fd3abac354cbacc3f94791344`
- Raw generation repeat SHA-256: `05a3b02abbea033d905c1e04757a073022ac923cf1ab897eae88470a36660b9c`
- Capture receipt SHA-256: `b08dbb4d64c48148b10598596ebd75d007e956c4e80a23daddbe9cecd810697b`

The 48 protected accepted-benchmark report/selector files remain byte-identical.
Neither this partial survey nor its new navigation entry replaces accepted
44-case planner evidence.

## Validation and reports

Before measured execution, the maintained eleven-step developer gate passed:
workspace/all-feature tests, CLI feature boundaries, formatting, strict Clippy,
52 JavaScript tests and documentation checks. The terrain study/survey Python
suite passed 36 tests; four focused survey report-adapter tests passed.

Saved survey verification passed after the stop and after report publication.
It authenticates retained inputs, records, comparisons and receipts; it executes
zero fresh flights or replays. The common batch check passed; all 108 detail
links exist, six verified records retain rich detail pages, and four sentinel
handoff annotations match their actual raw entry/handoff snapshots exactly.
Root → Report home → Waypoint planning → Procedural terrain survey is connected.
The failed/unattempted rows have status pages, not invented accepted trajectories.
These are automated/static checks, not human browser acceptance.
Final read-only checks passed four report-navigation tests, 602 local report
links across 112 pages, and 578 documentation links across 104 files. Frozen
source/executable identities, the complete partial-capture receipt and all 48
protected accepted files were rechecked after publication.

Open the separate
[survey batch](../outputs/reports/eval/planner_v2_random_terrain/index.html).
Read Diagnostics before interpreting the planned-cohort landing counts. The
[failed flight JSON](../outputs/eval/planner_v2_random_terrain/capture-random-20261006-v1-reviewed/runs/random-001/flight.json)
contains the physical landing and exact proof failure. Existing accepted report
bodies were not regenerated. The LAN report server was found stopped and was
left unchanged.

## Recommended next decision

Authorize a narrow numerical geometry/guard-consistency fix, not another planner
redesign or terrain difficulty search:

1. Add a regression for the exact saved source state, plus flat pad states at
   zero, positive and negative heights around floating-point spacing boundaries.
2. Prefer constructing conservative extents from rotated local geometry rather
   than recovering them by subtracting rounded world coordinates. Validate this
   choice against upright/tilted geometry and the existing landing/contact rules.
3. Keep meaningful penetration, out-of-domain terrain and required airborne
   reserve violations rejected. Do not add a broad source exemption, blanket
   clearance epsilon, raised source pose or cutaway to conceal the error.
4. After review and bounded preservation gates, separately authorize the exact
   failed-case check and a new frozen survey capture using the **same** 100 seeds
   and terrain inputs. Keep this stopped capture intact; do not silently resume
   or relabel it as evidence from the fixed source.

This is a small, evidenced issue worth fixing before judging terrain difficulty.
The current four-case wave is insufficient to conclude that this distribution
needs larger obstacles, more waypoints, or a different planner.
