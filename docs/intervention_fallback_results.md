# Failure-only timing fallback: eight gains, existing successes preserved

Date: 2026-10-07. Completes the bounded
[fallback plan](intervention_fallback_plan.md). This is development reuse of the
unchanged harder-terrain challenge, not a fresh held-out sample or promotion of
the accepted report.

## Verdict and current source

Retain the failure-only fallback. Unlike the rejected
[timing replacement](intervention_timing_results.md), it never evaluates extra
entries when the existing search succeeds. The same 100 challenge scenarios
improve from **57 to 65 verified landings**, with all 57 previous successful
complete flight records preserved apart from wall timings and additive query
diagnostics. There are no lost landings, physical crashes, timeouts or
integrity/source-replay failures.

The working policy-3 search now completes its original entry/template search
first. Only finite exhaustion opens up to four new conflict-relative clocks,
deduplicated against tested ticks. Ranking, 42 templates, five-metre reserve,
two-second continuation, six-correction cap, terrain-blind nominal construction
and actual-handoff replanning remain unchanged. This does not add a landing
suffix, terrain-aware apex adjustment, backtracking or a new executable policy.

## Measured populations

All 163 planned campaign evaluations completed: 17 focus cases, 44 benchmark
inputs, 100 challenge cases and two repeats. The two unsupported benchmark
inputs are preflight-only, not fabricated simulator/replay results. Ordinary
unit/CLI tests are separate from these populations.

| Population | Previous result | Fallback result | Preservation |
| --- | --- | --- | --- |
| Focus, 17 saved worlds | 3 landings | 7 landings | All three successful controls exact |
| Benchmark, 36 core cases | 36 landings | 36 landings | All 38 previously landed core/diagnostic flights preserved |
| Challenge, 100 saved worlds | 57 landings | 65 landings | All 57 previous successful flights exact |
| Fixed same-source repeats | Not a primary denominator | `035` finite, `050` landed | Both complete records exact except wall timings |

The fresh benchmark passes native acceptance: 11 direct core landings, 25
corrected core landings, two diagnostic landings, four finite diagnostic stops
and two unsupported inputs. All 44 pass integrity and all 42 supported cases
pass source replay. Diagnostic `v2_diag_near_target` now clears once and stops
`NoNominal` rather than refusing before departure; this is not a new landing.

Benchmark preservation uses the already sealed V2 comparison contract for the
earlier numerical repair, plus omission of optional query diagnostics from
separate comparison copies. It records 348 permitted clearance exceptions
(maximum absolute delta 3.410605131648481e-13 m) and 56 independently authenticated
derived-identity exceptions. Commands, full states, timing choices, outcomes and
proofs remain exact. No runtime tolerance was added, and raw captures were not
rewritten. The original challenge already includes that numerical repair, so its
57 successful flights compare exactly without those diagnostic exceptions.

## Challenge gains and remaining misses

All eight gains started as `NoClearing` in the original challenge:

| Case | Verified landing corrections |
| --- | ---: |
| `023` | 1 |
| `024` | 5 |
| `025` | 3 |
| `043` | 6 |
| `049` | 1 |
| `056` | 5 |
| `072` | 4 |
| `087` | 4 |

All 36 clear routes still land directly. Blocked-route landings improve from
21/64 to 29/64. Per-recipe outcomes, each over the same 25 worlds, are:

| Recipe | Before | Fallback |
| --- | ---: | ---: |
| Mountains 4x | 20 | 22 |
| Mountains 8x | 12 | 15 |
| Broad massifs 8x | 17 | 19 |
| Successive ridges 8x | 8 | 9 |

The remaining 35 outcomes are honest finite stops: 21 `NoClearing`, 11
`NoNominal` and three `CorrectionLimit`. Four original `NoClearing` cases (`000`,
`060`, `066`, `094`) now clear locally but stop `NoNominal`; `035` reaches the cap.
These are progress through the local search, not successful recovery or proof of
physical infeasibility. The seven original `NoNominal`, two capped and six
duplicate-clock `NoClearing` cases remain outside this fallback's direct coverage.

`035` illustrates the expected limitation: it preserves the recovered first
five handoffs from the rejected replacement, but the unchanged primary search
finds an acceptable sixth correction and therefore prevents fallback. Its final
handoff differs, and the next nominal is still blocked after the correction cap.
Do not tune the cap, ranking or timings to reproduce the rejected trial's landing.
Conversely, `043` lands because its existing later primary search finds a sixth
clearing that the replacement's sampling omitted. The extra post-handoff probe
`081` exhausts both searches and remains a finite `NoClearing` stop.

## Work and planning cost

Fallback executes in 28 challenge cases, over 29 cycles; 16 actual corrections
select fallback rows. No previously successful flight invokes fallback. Complete
primary-stage ranking remains in force, rather than stopping at its first hit.
The 100 cases query 30,198 rows in total, with at most 1,176 in a flight, below
the declared 2,016-row / 725,760-boundary maximum. Identical clocks are not retested.

Observed planning time per challenge flight averages 0.908 s (median 0.836 s,
maximum 2.771 s), versus 0.666 s in the saved original capture. Fallback cases
average 1.454 s; cases without fallback average 0.696 s. These are observed
four-worker capture timings, not a controlled performance benchmark, a pure
fallback-overhead measurement or a per-tick/game-loop latency guarantee.

## Source, validation and retained evidence

The evaluator executable SHA-256 is
`bd0da98b376352ea3aef08c585886a23e1a0202ddad35e16533d6d88b4d97e54`.
Both survey captures copy and authenticate the same 129-file source inventory
and exact original scenario bytes. The benchmark binds the same executable and
unchanged Rust source/input identities. Final working source and executable
match the survey freezes. All 17 focus records also match their full-challenge
records completely except wall timings, including the new diagnostics.

- Focus: `outputs/eval/planner_v2_random_terrain/capture-fallback-20261007-focus`.
- Benchmark: `outputs/eval/planner_v2_lab_suite/capture-fallback-20261007-benchmark`.
- Benchmark comparison copies/receipt:
  `outputs/eval/planner_v2_random_terrain/capture-fallback-20261007-benchmark-preservation`.
- Full challenge/repeats:
  `outputs/eval/planner_v2_random_terrain/capture-fallback-20261007-challenge`.

The maintained eleven-step development gate passes before and after focus,
including the unchanged ridge landing expectation and strengthened checks that
successful ridge/plateau searches have no fallback rows. Seven pure policy tests
and 52 terrain-study Python tests pass. Saved survey verification and native
benchmark acceptance pass. The separate read-only comparison authenticates all
38 previously landed benchmark flights without fresh simulation.

All 48 protected accepted selector/report files remain hash-identical. Native
captures retain common rich reports and collapsed query diagnostics. No report
navigation refresh, separate site publication, accepted promotion, server
operation, commit or push was performed. Historical replacement and challenge
captures/results remain unchanged. See the [evaluation workflow](evaluation.md)
for read-only saved verification; this completed allowance is not permission to
start another campaign.

## Next boundary

Review the implementation/evidence before committing or publishing a separate
comparison site. Further behavior work needs its own analysis and approval.
Do not keep tuning this timing rule toward perfection. The remaining local
search exhaustion and high-energy handoff-to-nominal failures are separate
clusters; a later gamified improvement should target one with a small concrete
rule while preserving these successful flights.
