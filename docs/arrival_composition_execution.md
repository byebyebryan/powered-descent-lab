# Arrival composition execution

This pass follows the [arrival and landing qualification plan](arrival_composition_plan.md).
Its create-only root is `outputs/arrival-composition/loop-20261024T051904Z`.
The baseline binds initial head `746d9a9`, all Git-visible file hashes and foreign
changes. The three-hour deadline is Unix 1792829944; other limits are 150 million
measured steps, 256 captures, 4 GiB retained artifacts, one native worker, zero
optimizer steps and at least 40 GiB free space.

Exact phase declarations precede measured outcomes. Old captures and their
interpretation remain unchanged; new native suffixes receive new evidence.

## Development release

The retained-tick seam and native witness composition are committed at `00981f6`.
Runtime epoch A freezes that source and uses native SHA-256
`a97523d1c4a18585cb2f1732d3848144ff72e1ae15119fdd211348e100332d68`.
The generator is separately frozen at `input-source-a/composition_inputs.py`
from `1196e2d`; its bytes and anchor are bound before generation.

The pilot runs three fixed witness goals in witness/corrected modes with two
fixed captures. All six composed executions land after fulfilling arrival;
both retained-prefix proofs pass. The 21-goal phase then completes all 42
witness/corrected composed captures with exact old/new native prefix parity:
21/21 local-plus-landing for both. Legacy has 13/21 local-plus-landing and
21/21 native landing observations. No guidance correction is warranted for
this restricted development task. All fourteen exposed diagnostic programs
retain their complete old decision info and fingerprints.

The original development declaration's shared Python roster dictionaries were
corrected before any execution in `development-declaration-v2.json`; the earlier
declaration remains retained. Inputs and capture roles did not change.

Twelve new development states span four families and three sink strata, with
continuous position, velocity, attitude and horizon jitter. Two independent
native goal recipes are `support_short` (5.5–6.5 s, gravity support) and
`brake_long` (7–8 s, gravity support plus 35% nominal lateral-velocity reduction).
Attitude-preparation recipes coast/point for 0.5 s first. Native endpoints
define goals using unchanged tolerances. There are no outcome exclusions.
All 24 corrected composites and 24 independent witness composites land after
fulfillment; all twelve direct controls land. Six fixed development captures
complete full proof. The generator is unchanged after these outcomes.

## Fresh confirmation freeze

`confirmation-release.json` freezes the existing corrected controller and input
rule before confirmation generation or execution. The new hash-stream cohort
uses seed 2310240723 and five independent jittered originals per family/sink
cell: 60 states, 120 arrival programs and 60 paired direct controls. Every one
of the 180 executions is a scheduled complete capture, including misses.
No sampling replacements, tolerance changes or confirmation-driven retuning.

The engineering gate requires arrival fulfillment and verified target landing
for all 120 arrival programs, without treating correlated goal programs as 120
independent states or claiming 99% population reliability. The baseline constant
is selected on development mean fuel, before confirmation outcomes. Hindsight
best-choice headroom is separate, with a 1 kg descriptive win margin. Fuel
comparisons remain conditional on the unchanged first-feasible landing selector.
The confirmation receives at most 100 million measured steps within the outer
150-million-step cap. Existing phases use 45,845,383 steps. Total scheduled full
captures are 244, within the 256 allowance.
