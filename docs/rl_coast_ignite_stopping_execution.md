# Coast ignition stopping study execution

[Documentation home](README.md) · [Research and frozen design](rl_coast_ignite_stopping_plan.md) · [Original hybrid result](rl_coast_ignite_execution.md)

October 22, 2026. The owner authorized this goal loop after reviewing the
stopping-rule plan. Implementation, bounded evaluation, a separate established
HTML report and local scoped commits are in scope. No training, push, default
promotion, change to burn arithmetic or refresh of the old report is included.

## Frozen allocation

The create-only run is
`outputs/coast-ignite-stopping/loop-20261022T204723Z/`.
`workspace-baseline.json` records the initial Git head, every Git-visible file
hash and original status. `execution-allocation.json` freezes checkpoint hashes,
stream keys, sampling key, bootstrap seed, counts and the two-hour outer deadline.
The initial head is `d3219152479908e58a855b3c57aec906202faa3b`.

Three existing checkpoints are fixed: seeds 281101/281102/281103 at
65,536/131,072/98,304 training decisions respectively. The primary remains
281103. This allocation performs zero optimizer steps. G uses original model
prediction, S uses eight declared random streams per development/validation
arc and weight, and M uses one cumulative median without a quantile sweep.

New streams use generator seeds 331202, 331203 and 331204, with cohort names
`coast-stopping-development-v1-20261022`,
`coast-stopping-validation-v1-20261022` and
`coast-stopping-fresh-v1-20261022`. They contain 128, 256 and conditionally 1,200
arcs. All prior training/validation/fresh physical identities remain excluded.
Retain every raw proposal and the original input-only admission rules.

The sampling key is `coast-stopping-s-v1-20261022-r1`; the bootstrap seed is
331205. SHA-256 over canonical ASCII JSON coordinates yields a midpoint uniform
from its upper 52 bits, strictly between zero and one in binary64. Matching
arcs/replicates/ticks use the same draws across frozen weights.

Native allocation: 16 preservation captures; 4,096 development occurrences,
including 128 captured cost-pilot occurrences; 8,192 validation occurrences;
at most 16 additional validation proof captures; conditionally 6,000 fresh
captures plus 384 secondary primary-policy draws if S wins. The maximum is
18,704 original executions. Replays and independent reconstructions are
additional work and are counted in the native-step budget.

The outer caps are two hours, one native worker, 200 million native steps,
20,000 original executions, 48 GiB of new artifacts and at least 80 GiB free
space before starting. The first-four-arc development pilot is capped at five
minutes, one million original native steps and 1 GiB. Its measurements release
later reservations within those caps. A fresh budget must reserve every rich
capture and proof before fresh inputs are opened.

Validation selects one common rule by equal weight across arcs and weights,
with S repeats averaged within each pair and exact ties ordered G, M, S.
Fresh confirmation opens only if a non-G winner improves both the aggregate
and fixed primary validation scores by at least 0.5 percentage points.
Otherwise the run closes with fresh withheld and retains the existing package.
The original plan owns all paired testing and separate 99% reliability criteria.

## Execution and recovery boundaries

All source used for measured runs is copied into a retained source epoch and
compiled there. Runtime source hashes, the pinned environment and native
artifact bind each capture. New evidence uses create-only files and records
every requested occurrence before execution. Software faults retain their
partial receipts and stop the measured phase; they never become task misses.
Routine fixes require a new source epoch and explicit continuation accounting
within the original allocation, without changing cases, weights, rules or the
nomination criterion. No outcome-based fresh retry is permitted.

The native bridge records the returned admission in a separate sidecar. Old
decision JSON and physical fingerprints remain unchanged. Reference-only
geometry is query evidence with instantaneous-pointing assumptions; core
native contact remains authoritative. The old controller, generator, state,
fuel, clocks and old captures stay unchanged.

Results, checks, scoped commits, source epochs and the separate report link
will be appended as phases close. Existing shared report/server ownership is
preserved; publication uses the running report service without restarting it.
