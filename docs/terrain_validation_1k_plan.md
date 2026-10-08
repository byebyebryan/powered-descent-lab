# Fresh 1,000-case planner coverage validation

Date: 2026-10-07. Approved after the review/commit checkpoint at `41506b8`.
This is a single bounded campaign, not a planner improvement or permission to
commit, push, publish reports, change selectors or operate the report server.

## Question and fixed scope

Does the current policy-3 planner's reused development result of 68/100 overall
and 32/64 initially blocked landings carry over to a larger unseen population?
Use the same four global recipes, 250 fresh worlds each, with the same tested
vehicle, Earth gravity, 120/60 Hz clocks, 90 s mission budget, 1200 m route,
resolved 4 m terrain and original-height local pad preparation. No cutaway,
per-seed filtering, difficulty tuning or changes to planning/controller logic.

The new source-bound
[campaign contract](../studies/terrain_profiles/challenge_validation_1k_plan.json)
uses master seed `2026100703`. Exclude the six profile-study seeds, all 100 sanity
seeds, 24 calibration seeds and 100 harder development seeds. Freeze the ordered
seed list before generating/viewing profiles. Generate twice in separate
processes with reversed traversal; prove byte-identical results. Prepare all
1000 inputs and pass native input-only preflight before any measured flight.

## Allowance and evidence

- Exactly 1000 primary attempts, three separately counted preservation controls
  and five predeclared repeats (`000`, `250`, `500`, `750`, `001`): 1008 total.
- Four workers, 300 s case wall limit, three-hour measured campaign limit.
  Input generation's reverse-process check has a separate 1200 s limit.
- Authenticate the current unpublished handoff-room benchmark before copying
  its direct, ridge and plateau control scenarios/flight records.
- Make only evaluation plumbing changes, then run the maintained developer gate
  and terrain-study tests. Freeze source inventory, Git identity, evaluator hash,
  reference generator, plan and exact scenario bytes before collection.
- Collect complete native flights and common rich detail reports in a new
  create-only capture. This is local capture output, not report-site publication.
- Keep physical/mission outcomes, planning stops, integrity and original-source
  replay distinct. Landing requires the complete tuple. Finite misses remain in
  the denominator; missing or unverified evidence is not a successful flight.
- Ordinary finite misses do not stop collection. Source drift, collection
  failures and integrity/replay failures stop with every attempt retained. Do
  not repair/tune during collection, replace seeds or automatically retry.
- Compare repeats completely except the three wall timings. Check controls with
  the existing authenticated comparison contract, retaining any exception list.
  Protect accepted selector/report hashes and all earlier captures.

## Analysis and closure

Verify the full capture with Python and the native saved-evidence checker.
Report primary completion and verified landings, initially clear/blocked/missing
nominal denominators, blocked-route landings, per-recipe outcomes, finite-stop
categories, correction counts and observed planning cost. Keep controls/repeats
outside the primary denominator. Compare with the 100-case development corpus
as two distinct populations; do not merge them or claim paired improvement.

This tests fresh-seed coverage under the same terrain distribution. It does not
isolate each earlier fix, certify all generated worlds are physically feasible,
establish arbitrary-terrain reliability or measure game-loop latency. Once
results are inspected or used for design, this population is no longer blind
validation for subsequent planner changes. Close the allowance after collection
and verification; further flights or behavior work need a separate decision.
