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

Existing shared report/server ownership is preserved. Delivery uses the running
report service without restarting it or changing the accepted report selector.

## Completed development and validation

All 16 preservation captures reproduce the old G decisions, physical
fingerprints, fuel/clocks and controller feedback exactly. All 4,096 development
occurrences complete on 128 new arcs: G averages 96.61%, M 99.74% and S 98.24%
across the three frozen weights. Validation completes on 256 disjoint arcs:

| Frozen weight | G | M | S across eight draws per arc |
| --- | --- | --- | --- |
| 281101 at 65,536 | 239/256 | 253/256 | 1,975/2,048 |
| 281102 at 131,072 | 249/256 | 255/256 | 2,028/2,048 |
| 281103 at 98,304 | 251/256 | 254/256 | 2,013/2,048 |

The equal-weight averages are G 96.22%, M 99.22% and S 97.92%. M wins by
2.995 percentage points overall and 1.172 points for the fixed primary weight,
passing both prospective 0.5-point nomination conditions. The selected package
is frozen before fresh release. The 16 scheduled selected/G validation proof
captures pass physical replay and both independent reconstructions.

## Pre-release source continuation

Epoch A completes preservation, development, validation, selection and validation
proof. Its conservative fixed reservation stops before generating any fresh
input: maximum pilot capture size plus presumed duplicated rich reports does
not fit the original 48 GiB cap. The retained fault and guard receipt record
that stop. No physical outcome or validation verdict is changed.

Display-only calibration of the completed 128-occurrence pilot measures maximum
capture size 4,598,243 bytes and maximum rich bundle 2,312,667 bytes. Epoch B
uses one rich copy, a 128 KiB context reserve per occurrence and 10% contingency.
The resulting 46,477,081,200-byte fresh/delivery reservation fits the unchanged
cap after retained artifacts are counted. The two-hour deadline also remains.

The explicit continuation carries all resource counters and reuses every
completed input, score, selection and proof capture. It repeats zero development
or validation episodes. Runtime source comparison permits only runner/display
changes; models, generator, hazard, features, stopping rules and controller/plant
arithmetic remain unchanged. Both source epochs and original selection survive.
Fresh generation happens once, in epoch B.

## Independent fresh result

All 6,000 occurrences complete on 1,200 independent new arcs. Every capture
passes physical replay, controller reconstruction, live planner reconstruction
and diagnostic/random-stream binding. No optimizer step runs.

| Frozen package | Verified target landings | Rate |
| --- | --- | --- |
| M, weight 281101 | 1,189/1,200 | 99.083% |
| M, weight 281102 | 1,197/1,200 | 99.750% |
| M, primary weight 281103 | 1,193/1,200 | 99.417% |
| G, primary weight 281103 | 1,176/1,200 | 98.000% |
| Fixed three-second delay | 854/1,200 | 71.167% |

The primary comparison has 21 gained and four lost landings: lift 1.417
percentage points, paired-bootstrap 95% interval 0.667–2.250 points, exact
one-sided paired p `0.0004552602767944336`. The improvement criterion passes
for the fixed primary package on this supplied-arc task. Other weights lack
matched fresh G controls; their results do not establish individual decoder
effects or broad training-seed qualification. No strongest fresh weight is selected.

The primary exact two-sided 95% interval is 98.802–99.765%. Its one-sided 95%
lower bound is 98.907%, below the separate 99% target. That reliability criterion
remains unmet: it required at least 1,194 successes. The seven primary misses
are six typed above-maximum-thrust rejections and one crash after an accepted
burn. Failures remain retained without retries or another timing adjustment.

## Evidence and HTML delivery

The [complete primary HTML tree](http://starship:8000/coast-ignite-stopping/loop-20261022T204723Z/campaign/report/result-v2.html)
contains five matched attempts under each fresh arc. It links separate
preservation, development-pilot and validation-proof trees with their own policy
sets and denominators. All 6,160 captured occurrences have rich flight details,
saved references, stopping traces, admission reasons and incoming-contact
geometry. Summary-only ledgers remain distinct from replay-certified captures.

Shared tree browser code and rich templates remain intact. Per-population
projections preserve dense policy slots, with primary median as the default
fresh view. Camera bounds use saved flight/pad/reference bounds while retaining
full terrain and complete payloads. Immutable final indexes add descriptive
phase, direction, nominal-duration and mechanical-energy subsets of the same
1,200 arcs; those subsets do not affect selection or acceptance.

Visual review catches an initial phone table overflow missed by comparison
against an expanded layout viewport. Final indexes use explicit table scroll
containers, and corrected tests enforce the requested 390 px phone width.
Desktop and phone pass all four trees, unique pagination, matched navigation,
rich plots, stopping diagnostics and intact terrain with zero browser errors.
Previous indexes/receipts survive; rich flight files are not rewritten.

The independent read-only audit verifies all 6,160 capture/decision/sidecar
bindings, source/model hashes, 1,584 disjoint new inputs, paired results and
resource counters. All 2,400 old confirmation captures retain their hashes.
The measured study has 18,320 original executions and 100,344,147 native steps:
37,671,927 original steps, 37,603,332 neutral replay steps and 12,534,444 in
each reconstruction family. Pure diagnostics execute no native forecast.

Final developer/Python checks, exact served bytes, workspace preservation,
artifact totals and owned-browser cleanup are recorded in `closure.json` under
the run root. Further learning, stronger controller admission, push and default
promotion remain separate decisions.

The follow-up [timing and efficiency plan](rl_coast_ignite_efficiency_plan.md)
records the read-only paired fuel analysis and proposes a separate discrete
handoff characterization. It does not reopen this study's fresh confirmation,
change its qualification verdict or allocate further training.
