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

## Confirmation and conditional decision value

The unchanged input rule constructs all 120 arrival goals on 60 fresh states,
with zero construction exclusions. Every corrected arrival fulfills the original
instruction and then verifies target landing. All 60 paired direct controls also
verify target landing. Every one of these 180 executions has complete native
capture, mission success, integrity, physical replay, controller reconstruction,
input-goal reconstruction and full Summary equality. The declared engineering
gate passes without a guidance correction or confirmation-driven retuning.

| Confirmation choice | Arrival fulfilled and verified landing | Mean fuel kg |
| --- | ---: | ---: |
| Direct, selected as best constant on development | 60/60 direct landings | 741.591670 |
| Support short | 60/60 | 777.912935 |
| Brake long | 60/60 | 798.952165 |

The input-only envelope is height 725–875 m, horizontal position zero or
310–390 m from the pad, lateral speed zero/22–28/37–43 m/s, sink 13–47 m/s,
and initial attitude within about ±0.29 rad. Vehicle and initial fuel are fixed.
The first actual goal resolution, rather than its recipe endpoint, starts the
unchanged continuation selector. The goal construction horizons do not prescribe
arrival time: arrival still has an absolute 30 s deadline inside the original
90 s mission. This is one optional arrival, not continuously retargeted control.

Direct remains the best fixed choice. Hindsight best captured choice is direct
on 32/60 states, support on 21 and braking on seven. All 28 non-direct wins
exceed the predeclared 1 kg descriptive margin. The hindsight mean is
704.806455 kg: 36.785215 kg, or 4.9603%, below always-direct; maximum saving is
228.102064 kg. This is conditional decision headroom against the fixed
first-feasible coast/duration selector, not a learned policy or a fuel optimum.
A stronger direct timing baseline could reduce that advantage. No new planner
policy is selected from the confirmation outcomes.

The 120 goal programs share 60 original states; neither those paired goals nor
the retained reflected witnesses are additional independent state samples.
The zero observed failures pass the declared bounded engineering gate, without
claiming 99% population reliability or arbitrary airborne/vehicle qualification.

## Retained failures and controller decision

All fourteen diagnostic programs reproduce their complete old decision info and
fingerprints, with full new proof. They remain outside the confirmed envelope:

| Retained diagnostic role | Programs | Outcome |
| --- | ---: | --- |
| Newly lost native landing observations from the actuation comparison | 9 | Arrival plane miss; finite Poly search finds no plan. Eight use only gap-off and zero fuel; one has mixed allocation. |
| Lost local arrival | 1 | Plane miss followed by verified landing. |
| Arrival fulfilled without continuation | 2 | Local goal reached; finite Poly search finds no plan. |
| Absolute operation expiration | 2 | Arrival expires, followed by verified landing. |

The zero-thrust mechanism remains the preserved geometry projection combined
with the corrected minimum-thrust mapping. The expiration/reference-retention
questions and high-energy shallow-entry failures remain unresolved. Finite
no-plan does not prove physical impossibility. The composed witness and fresh
results show that these broad failures do not require a speculative correction
to establish this smaller working envelope. No angle limit, reference/recovery
rule, tolerance, shared allocator or Poly burn arithmetic is changed.

The next supported experiment is a bounded three-choice planner: direct landing,
support goal or braking goal, using this frozen corrected controller. It needs
new training, validation and test inputs; the current confirmation is now
exposed. The qualified interface supplies goals rather than native witness
commands. Input IDs/family labels, hindsight choices and witness tapes must not
be privileged policy inputs. Efficiency claims must state the fixed selector
baseline and distinguish them from optimized direct-timing comparisons.

## Evidence, checks and HTML delivery

There are 244 complete captures and 488 successful reconstructions. All pass
complete physical replay, integrity, goal reconstruction and Summary equality.
233 captures verify target landing; the other eleven retain diagnostic crashes.
All 44 scheduled retained-prefix proofs match old native observations through
the first resolution and command frames before it. The read-only independent
audit recomputes arrival outcomes/ticks from native samples, verifies actual
handoff equality, source/input identities, all selected proofs, fresh/development
physical-state separation and exact measured work. See `evidence-audit.json`.

All 19 maintained development checks pass, including 198 control unit tests
and ordinary adapter parity. All six native Python tests pass. The final
documentation checks and relevant Node tests pass; retained 44-case numerical
capture parity remains optional. The gate wrapper initially extracted the wrong
completion-marker pattern; `developer-gate-final.json` binds the exact successful
19-check marker and log hash while preserving the original receipt.

The [current HTML report with corrected flight previews](http://starship:8000/arrival-composition/loop-20261024T051904Z/report-flight-frame/report.html)
uses the same captures and established mission tree. The
[original HTML edition](http://starship:8000/arrival-composition/loop-20261024T051904Z/report-delivery/report.html)
contains all 244 rich flights, 104 state comparisons, first-handoff diagnostics,
failed cases and the conditional fuel figure. Shared templates, old reports,
selectors and server lifecycle remain unchanged. The initial `report/` edition
is retained; `report-delivery/` clarifies goal-construction time versus arrival
deadline without changing any execution. The original presenter source is
retained in `presentation-source/` and both editions have source-bound receipts.

Desktop 1440×1000 and phone 390×844 browser checks visit every occurrence and
exercise seven rich details each, plot modes, comparisons and next/back
navigation. A second QA pass explicitly scrolls through visible lazy previews
before screenshots; all load successfully. All 244 complete native captures
and every preview SVG pass hostname byte checks. The owned disposable Chrome
is cleaned up afterward; user browsers and report servers are preserved.

The [immutable published findings](http://starship:8787/reports/c1cf57dff31478395e4db1eddf631d43742a6cccd78df4bca2ac16c4ec0b6c61/)
retain the summary, standalone figure and artifact manifest, linking the complete
canonical tree. Its rich report is 676,585,705 bytes and sealed native captures
add 1,325,131,553 bytes, exceeding the shared publisher limit. This index keeps
the canonical raw-capture links intact. All four published assets pass hostname
byte checks; desktop/phone publication checks verify content, figure and the
link to all 244 rich views.

## Resources and closeout

Total measured work is 113,032,287 steps: 3,824,673 native plant steps,
98,546,290 local model steps, 8,404,262 selection forecasts and 2,257,062
neutral replay steps. Native totals include 116,462 input-construction steps
and 451,358 subsequent goal-generation/reconstruction steps. There are 567
original occurrences, one native worker and zero optimizer steps. Phase wall
costs total 94.08 seconds; build, implementation and review time are separate.
Per-update controller CPU timing is uninstrumented. The final `closure.json`
binds elapsed time, artifact/free-space limits, heads and baseline preservation.

The existing corrected profile passes this declared arrival-to-landing menu.
Guidance repair, default promotion, RL training and push are not performed.

The subsequent [timing and goal-selection pass](arrival_timing_selection_plan.md)
separates timing improvement from intermediate-goal value before conditionally
fitting a small selector. Its outcomes belong to its separate execution record.

## Display-only preview framing correction

The [updated mission tree](http://starship:8000/arrival-composition/loop-20261024T051904Z/report-flight-frame/report.html)
corrects thumbnails that previously fitted the entire −4,000 to +4,000 m flat
terrain strip. An 814 m descent occupied only 13 pixels vertically. The new
opt-in flight frame fits the full actual path, pad and requested arrival goal,
clips terrain to the viewport, retains equal metre scales and shares bounds
across each physical mission's choices. All 180 confirmation traces now occupy
at least 68.49 pixels vertically. Requested goals use amber diamonds; a metre
scale bar and start altitude make distances explicit.

The [published before/after comparison](http://starship:8787/reports/1832924a01981773fb7294192e229e982b0c3a6ba3f603ed39ad792fd689e1cb/)
links the full 244-flight edition. Existing terrain-focused preview APIs retain
their output; an old/new unfocused export has seven byte-identical assets.
Navigation/tree data, all 488 displayed evidence/proof files and all 976 sealed
capture files remain byte-identical. No flight/controller execution or proof
regeneration is part of this display correction. Earlier editions are preserved.

The correction's source, asset checks, legacy parity and desktop/phone browser
evidence are retained under `outputs/report-preview-framing/20261024T074448Z`.
Both viewports browse every occurrence, load visible previews and exercise
the 104-state comparison picker, seven rich flights each, plot modes and
next/back navigation. Publication checks verify all four before/after SVGs and
the full-report link. Rich detail/tree templates and server lifecycle are unchanged.
The maintained 19-check development gate and all thirteen focused documentation/
tooling Node tests pass for the correction.
