# Paired early-exit 1k rerun

Status: **completed; frozen before collection**. See the
[paired results](terrain_early_exit_sweep_results.md).
This extends the completed [bounded early-exit check](terrain_early_exit_results.md),
not its closed nine-mission allowance. No planner/controller changes are in scope.

Use the exact retained early-exit executable and explicitly isolated cap 24 to
rerun the same 1,000 worlds as the [previous cap-24 sweep](terrain_cap_sweep_results.md).
Copy every scenario byte unchanged. Keep the original three controls and seven
repeat indices, for at most 1,010 measured attempts, four workers, 300 seconds per
case and three hours collection time. No retries, replacement inputs, fresh
terrain, normal-pack recapture, ordinary cap promotion, commits or publication.
The [machine contract](../studies/terrain_profiles/early_exit_sweep_plan.json)
pins baseline/candidate captures, binary identities and these bounds.

Require the complete physical/mission/integrity/source-replay landing tuple.
Track new and lost landings, all 387 clear routes, 613 blocked routes, per-recipe
results, finite stop categories, correction counts and early-query dispositions.
Promotion requires preserving all 748 old landings. Honest finite regressions
do not abort this measurement: retain them and finish the fixed population.
Runner, proof, integrity, input/source drift and wall-bound failures stop further
submission after the current four-worker wave; retain every launched result.

For a flight with no committed early exit, require complete non-timing parity
against the cap-24 baseline, excluding only root policy/input identities and
independently checked optional early-query metadata. For a committed exit,
require the original selected clearing proposal and ordinary command/event/sample
prefix through the actual early boundary, plus the exact queued nominal binding.
Require repeats to match all non-timing fields. Report clear-route preservation
and all lost successes explicitly; no case-specific exceptions or tuning.

Retain a create-only source/input/binary snapshot, ordered attempt ledger,
partial failures, baseline comparisons and closed receipt. Reuse rich native
mission reports; do not refresh the existing site or change navigation/selectors.
This is paired development evidence, not a fresh held-out reliability estimate.

Collection correction: the first capture stopped after seven launched missions
because a collector required byte-identical native reserialization of scenario
JSON. All seven flights passed integrity/replay and all scenario values match.
The fixed collector compares complete parsed output values while keeping original
input bytes receipt-pinned. Continue in a new create-only capture by importing
the seven complete authenticated flights and original error ledgers, explicitly
revalidating without execution; only unattempted worlds/repeats may launch. The
stopped capture remains unchanged. This is no new retry or numerical exception.
