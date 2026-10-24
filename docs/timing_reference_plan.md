# Timing reference qualification

This pass follows the [timing comparison](arrival_timing_selection_execution.md).
It establishes a bounded offline timing reference, measures selection separately
from control execution, and releases one independently sampled qualification.
No learner, controller redesign or default promotion is included.

## Frozen behavior and reference choice

Preserve the current vehicle, initial fuel, Earth gravity, flat terrain,
120 Hz plant/60 Hz commands, native attitude/fuel/contact physics, original
90 s mission clock and Poly burn arithmetic. Retained arrival programs keep
their 30 s operation deadline, original goals/tolerances and 0.72 rad tilt bound.
Timing is a planner decision; the deterministic executor prepares attitude and
executes the accepted timed burn. The experimental wrapper need not be refactored.

Keep legacy first-feasible timing and the existing 63-pair fuel-ranked bank
exactly unchanged. One explicit candidate uses coast seconds
`[0, .5, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]` and burn seconds
`[6, 8, 10, 12, 14, 16, 18, 20, 24, 30, 36]`: 132 pairs including all 63 originals.
Use the same native preparation, admission and contact forecasts, minimum fuel
ranking and exact grid-order tie break. Resource exhaustion is a fault, never
a partially searched optimum. Best within either bank is not global optimality.

On all 60 exposed direct originals, verify exact old-selector results and
capture every selected expanded-bank execution. Select the expanded reference
only if all 60 target landings verify and mean fuel saves at least 1% against
the 63-pair bank. Otherwise retain the 63-pair reference. One candidate only;
no outcome-driven bank expansion, tolerance change or flight-logic correction.
The reference choice is frozen before qualification inputs are generated.

## Independent input populations

Freeze the input-only rule before development outcomes. Primary seed is
2310241853. For each of 300 originals, independently choose one of four families
uniformly and one of three sink bands uniformly, then use independent hash-stream
jitter following the existing composition population: height 725–875 m,
horizontal position zero or 310–390 m from the pad, lateral speed zero/22–28/
37–43 m/s, sink 13–17/28–32/43–47 m/s, and family-conditioned modest attitude.
The 12-cell mixture has no fixed quotas. Unique physical states and separation
from exposed inputs are audited. IDs and seeds are provenance, not policy inputs.

The separate boundary panel has 60 originals, seed 2310241854: all 12 cells and
five fixed endpoint patterns. Directions alternate; height, horizontal offset,
lateral speed, sink and attitude take declared range endpoints. Patterns are
all-low, all-high, alternating low/high, alternating high/low, and low height
with high speed/sink. Apply the same family conditioning as primary generation.
It is an engineering diagnostic panel, not another random reliability sample.
There is no native outcome admission, exclusion or replacement in either panel.

All 300 primary and 60 boundary originals compare legacy first-feasible, the
63-pair bank and the chosen reference. Identical chosen/63 profiles share an
execution and capture rather than create duplicate samples. The maximum is
900 primary and 180 boundary captures. Retain all 21 previously witnessed local
arrival instructions with the selected continuation as separate handoff
regressions, preserving native prefixes through actual arrival resolution.

## Acceptance and interpretation

Every qualification execution is captured, including misses. Require complete
physical/mission/integrity/replay evidence, independent controller and input-goal
reconstruction, Summary equality and selected-forecast/actual agreement.
Physical misses remain results; software/source/resource faults retain receipts
and stop their phase. No confirmation-driven tuning or replacement cohort.

The selected reference must verify all 300 primary target landings to pass the
predeclared 99% criterion: for zero failures, the one-sided 95% exact binomial
lower bound is `0.05**(1/300) = 0.9900639181`. This is conditional on the frozen
independent mixture and plant, not every family, corner, arbitrary vehicle or
uncertain dynamics. Primary cases, boundary cases, regressions and controller
attempts have separate denominators. Boundary failures block an engineering
claim over those endpoints and are never hidden by the primary average.

Report paired fuel differences, mean/median/tails and original-level bootstrap
intervals. A general paired efficiency claim requires complete verified success
of both compared profiles; if a control fails, report all mission outcomes and
label any successful-pair fuel summaries conditional. Do not credit a crash's
low fuel as efficiency. The fuel gain need not reproduce the exposed 20.45%.
Report native incoming-contact speed, lateral motion, attitude/angular rate and
pad offset before final-velocity reset. Preserve the existing physical criteria;
no new margin threshold is invented after outcomes. No promotion follows a pass.

## Cost pilot and isolated timing measurement

Before main collection, use exposed originals 000/015/030/045 with all three
profiles: 12 Summary executions. Fixed pilot captures are expanded timing on
those four originals. Cap: 25 million charged steps and four captures.
Freeze later phase reservations from costs, without reducing the primary sample
or altering its rule to fit observed results. Stop before release if complete
qualification and proof cannot fit the outer allocation.

Instrument selection's monotonic wall duration separately from ordinary command
frames. Keep nondeterministic measurements outside decision info, descriptors
and fingerprints; verify measured/unmeasured execution equality. Isolated timing
uses exposed originals 000/005/010/015/020/025/030/035/040/045/050/055, all profiles,
and ten repetitions each: first invocation, two warmups, seven measured repeats.
Rotate profile order deterministically. First invocation is not an OS-cold-start
claim. Record raw selection/ordinary-frame times, median/p95/max, forecasts and
whole-episode process CPU separately. No developer gate/report work runs during
timed collection. Record host/affinity/load context; no real-time deadline is
inferred from the 60 Hz command rate. Charge all repetitions and warmups.

## Allocation and delivery

Create-only root: `outputs/timing-reference/loop-20261024T185303Z`.
The baseline binds head `2a87566`, 934 Git-visible file hashes and exact foreign
status. Outer deadline is Unix 1792878783 (three hours). Limits: one billion
charged native/model/forecast/neutral-replay steps, 1,200 full captures, 12 GiB
new artifacts, one native worker, 40 GiB free space and zero fitting/RL updates.
Build/software checks are separate. Reserve proof work before captures.

Use frozen source/library/input ledgers, create-only outputs and an independent
read-only schedule/predicate/prefix/proof/accounting audit. Preserve old evidence
and unrelated work. Commit completed slices locally. Deliver the established
rich HTML mission tree, complete comparison inventory, incoming-contact evidence,
cost distributions and honest acceptance/stop result. Use the corrected focused
previews and actual-image decode/paint browser checks. New report publication is
authorized; no report-service lifecycle change, push or default promotion.

The [execution record](timing_reference_execution.md) owns exact releases and
outcomes. The statistical interpretation follows
[exact binomial limits](https://itl.nist.gov/div898/software/dataplot/refman2/auxillar/exacbici.htm).
