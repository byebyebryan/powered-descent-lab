# Simple conflict-relative intervention timing pass

Date: 2026-10-07. Approved after the read-only investigation of the
[harder terrain challenge](terrain_challenge_results.md). This is development
on the same frozen worlds, not another independent reliability sample.

## Scope and hypothesis

Initial clearing entries are tied to launch-boost duration, while later entries
are conflict-relative. Five saved cases have no entry before their conflict;
nine near-source cases have only one or two; seven late-arrival cases cannot
satisfy the existing finite maneuver/certificate family from their early entries
even under optimistic terrain-free bounds. These are search-policy limitations,
not proofs of physically impossible terrain.

Replace those clocks with one four-choice rule over the interval from
`max(current, conflict - local_horizon)` to the first conflict. The local horizon
is derived from the existing maximum eight-second burn plus six-second coast.
Sample at 0, 1/4, 1/2 and 3/4 of that interval, round down to command boundaries,
deduplicate and keep all entries strictly before the conflict. Existing admission
rejects grounded or insufficient-reserve entries; do not add a launch solver.
This is a bounded game-oriented heuristic, not a completeness theorem.

Keep nominal construction, terminal control, 42 maneuver templates, ranking,
five-metre reserve, two-second continuation, correction cap, mission clocks and
source replay unchanged. Do not add landing-suffix requirements, feature
segmentation, a larger grid, per-seed tuning or terrain-aware nominal apex changes.

## Execution and acceptance

The [frozen pass contract](../studies/terrain_profiles/intervention_timing_plan.json)
binds the prior manifest and input receipt. Copy exact saved scenario bytes;
do not regenerate or alter terrain. Every phase has a separate create-only
source snapshot, bounded four-process execution and a sealed outcome ledger.

1. Add compact query-row diagnostics with exact cutoff reasons, stop states,
   minimum reserve, required progress, rejection counts and the first failed
   handoff/continuation. Do not retain every trace or every boundary. Historical
   absent metadata must round-trip unchanged.
2. Run the 16 frozen cases with unchanged timing and require complete flight
   equality to saved results, excluding only the three wall timings and additive
   row diagnostics. This proves logging did not change selection or motion.
3. Apply only the shared timing rule and run the same 16. Controls `034`, `050`
   and `054` must remain verified landings. Expand only if formerly stopped cases
   show verified recovery; merely exchanging one finite stop for another is not
   enough. A failed timing hypothesis closes this pass with retained evidence.
4. Conditional expansion: maintained developer gate, one accepted 44-case pack
   without publication/promotion, the unchanged 100 challenge cases and fixed
   same-source repeats `035` and `050`. At most 178 case evaluations across the
   two focus captures, pack and full recheck; unsupported pack inputs remain
   preflight-only. No extra measured tuning loop is implicitly authorized.
5. Require accepted-pack landing expectations and integrity/source-replay proofs;
   corrected trajectories may change intentionally. Compare challenge gains and
   losses explicitly, keep clear-direct preservation separate, and label this
   sample as development reuse. Finite misses are allowed; do not reinterpret
   them as physical infeasibility.

Preserve prior captures/report pages and the accepted selector/site. New report
views use the common rich detail and batch templates, clearly separated in normal
navigation. No commit, push, server lifecycle change or accepted promotion is
authorized. Nominal/braking/progress-ranking fixes require a separate decision.
