# Failure-only intervention-timing fallback

Date: 2026-10-07. Implements the small follow-up to the rejected
[timing replacement](intervention_timing_results.md), not a new terrain sample.
The [completed results](intervention_fallback_results.md) close this allowance;
the sequence below records the executed contract, not an active task list.

## Behavior and boundaries

Complete the existing four-entry local search with unchanged ranking. If it
finds a clearing maneuver, do not evaluate fallback entries. Only after finite
exhaustion, try four quarter-window samples in the existing local 14-second
horizon before the first conflict, bounded by the actual live origin. Align to
command clocks and skip already-tested ticks. Ordinary entry admission still
rejects grounded and insufficient-reserve states.

Keep the 42 templates, five-metre reserve, two-second continuation, six-correction
cap and actual-handoff replanning. Do not change nominal construction, landing
control, ranking, terrain, state/fuel/clocks or proofs. No fallback follows
`NoNominal`, a correction-cap stop or integrity errors. Maximum local work is
eight entries per cycle (336 rows), 2,016 rows and 725,760 candidate handoff
boundaries across six corrections. Source/executable hashes bind the intentional
policy-3 extension; no alternative selector or historical relabeling is added.

## Frozen validation sequence

The [contract](../studies/terrain_profiles/intervention_fallback_plan.json) binds
the original challenge manifest and scenario receipt. Copy exact saved inputs;
do not regenerate, filter or tune terrain.

1. Clock, deduplication and work-bound tests; tracked successful searches must
   have no fallback entries and retain the existing complete-stage ranking.
2. Source-freeze and run 17 cases: the previous 16 plus `081`, a long post-handoff
   conflict interval. Controls `034`, `050`, `054` must remain motion-exact
   verified landings, and new verified landings must be demonstrated. `023`,
   `049`, `056` are strong recovery hypotheses; `035` remains uncertain, not an
   outcome to tune into success.
3. On a passing focus gate, run the maintained development gate and one fresh
   44-case benchmark with `--no-publish`. Require existing acceptance and exact
   preservation of every previously landed benchmark case; diagnostics may
   improve without changing their inputs or acceptance rules.
4. Only then run the unchanged 100 challenge cases and two fixed repeats, `035`
   and `050`. Preserve all previous successful trajectories and require complete
   physical/mission/integrity/replay evidence for gains. Same-source repeats are
   exact apart from the three wall timings.

At most 163 campaign evaluations: 17 + 44 + 100 + 2. Unit/CLI tests are separate.
Four workers, 300 seconds per survey case and three hours per survey capture
remain the bounds. Use create-only, source-frozen captures and account for every
attempt. Do not restart a stopped root. Failed focused preservation/recovery
closes the bounded experiment without conditional expansion; retain all evidence.
Finite misses are not proofs of physically impossible terrain.

Benchmark comparison uses the existing sealed V2 diagnostic comparison contract
for the earlier numerical repair: only its already-declared tiny body-clearance
and authenticated derived-hash exceptions are permitted and recorded. New query
metadata is removed from separate comparison copies, never from captured flights.
Commands, full states, timing choices, outcomes and proofs remain exact. Challenge
successes use exact complete-flight comparison apart from wall timings and query
metadata because that baseline already includes the numerical repair.

Six original `NoClearing` cases have no new fallback ticks, seven cases stop
`NoNominal` and two hit the cap. Those are outside this extension's direct scope.

## Handoff

Document final source, gains/losses, fallback usage, work and planning cost.
Native captures retain common rich mission reports and query diagnostics.
No commit, push, server operation, navigation refresh, separate site publication
or accepted-capture/report promotion belongs to this pass.
