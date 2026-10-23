# Coast ignition timing and efficiency execution

[Documentation home](README.md) · [Timing and efficiency plan](rl_coast_ignite_efficiency_plan.md) · [Frozen stopping result](rl_coast_ignite_stopping_execution.md)

Next controller-first pass: [airborne arrival and Poly Landing](airborne_arrival_plan.md)
with its separate [allocation and evidence](airborne_arrival_execution.md).

October 22, 2026. The owner authorized implementation and execution of the
bounded timing characterization, rich HTML delivery and scoped local commits.
The supplied reference-end times, frozen models, controller/plant behavior,
physical task and earlier results remain unchanged. No training or push is
allocated.

The create-only evidence root is
`outputs/coast-ignite-timing/loop-20261022T225427Z/`. The retained workspace
baseline binds the initial head, Git-visible file hashes and foreign changes.
The outer deadline is two hours after the goal start at Unix time 1792709628.

The allocation is all 128 retained development inputs plus 30 separately
labelled diagnostics from the exposed confirmation: the 28-arc primary G/M miss
union and fuel outliers 488/552. Each gets 81 ignition requests on the original
0–8 s, 10 Hz clock plus never: 12,956 Summary executions. The first four
development maps are the pilot, not additional cases. Up to 256 selected full
captures include eight frozen-policy preservation controls, 16 fixed pilot
controls, one best successful point per arc, 60 diagnostic M/G choices and 14
neighbors for best points on the seven original M misses. Duplicate physical
executions can serve multiple capture roles; unused slots remain unused.

Limits are one native worker, 100 million native steps including all proof
passes, 14,000 originals, 8 GiB of new artifacts and at least 16 GiB initially
free. The pilot has five minutes, five million native steps and 1 GiB. Its
measured capture/report costs must release the full phase within those limits.
The runtime allocation binds exact inputs, old ledgers and model hashes before
execution; a copied source epoch binds the compiled native artifact and scripts.

Summary map outcomes remain provisional native observations. Selected full
captures require physical replay, controller reconstruction and the actual
fixed-slot scheduler or frozen learned-policy reconstruction. A capture must
match its Summary result exactly. Source/software faults retain receipts and
stop rather than becoming physical misses or triggering a different timing.

Report delivery extends the established rich mission tree with per-arc timing,
fuel/time and contact-margin plots. Main and diagnostic cohorts and native
Summary versus verified capture evidence remain distinct. Existing report
selectors, old evidence and the running server's lifecycle remain untouched.

## Completed scan and proof

The retained source epoch builds successfully and passes all 22 Python tests
before the campaign. All eight G/M preservation controls match their original
native decisions, feedback, clocks, fuel and diagnostics. The pilot completes
328 Summary occurrences and 24 full captures in 4.462 seconds. Display-only
calibration measures a maximum rich bundle of 2,312,768 bytes. The retained
pilot receipt includes the pre-existing source/environment bytes; its 1 GiB
local limit applies to newly created pilot artifacts.

The measured full-phase reservation is 67,296,000 native steps and
2,289,679,999 additional bytes within the original caps. There is one source
epoch, no campaign fault, no outcome retry and no optimizer update. All 12,956
Summary occurrences finish. The 256 capture roles resolve to 254 distinct
captured executions because two occurrences serve multiple roles. Every capture
passes three neutral physical replays, controller reconstruction, and either
the actual fixed-slot scheduler or frozen learned-policy reconstruction.
All 158 best observed successful grid points have verified target landings.

The independent read-only audit checks all input and old-ledger hashes,
models, all 887 retained source files, complete maps and their projections,
selected best points, capture/decision/sidecar bindings and accounting. It
executes no native step. The campaign uses 13,210 original executions and
21,459,649 native steps: 19,233,059 original, 1,335,954 neutral replay and
445,318 in each of the two reconstruction families. Pure reference diagnostics
record 19,623,385 grid queries separately. Source/unit tests remain separate
from these measured campaign counters.

## Observed timing landscape

Every one of the 128 development arcs and 30 separately selected diagnostic
arcs has a successful tested powered timing. Each observed success set is one
connected run of 10 Hz points; this does not certify continuous intervals or a
new online policy. Development has a median 38 successful points, spanning
3.7 seconds, with a range of 16–69 points. These are already exposed inputs.

| Retained development result | M | G |
| --- | --- | --- |
| Native successes at the existing choice | 128/128 | 125/128 |
| Misses with another successful tested timing | 0 | 3 |
| Median fuel above the best tested successful point | 0.771052 kg | 1.001823 kg |
| Median excess as a fraction of best-point fuel | 0.195999% | 0.248176% |

Relative to M's actual consumption, the median available fuel saving is
0.195614%. Fourteen development arcs offer at least 1% lower consumption at
another tested timing. All fourteen minima lie at a timing edge without two
successful adjacent requests, and all contact more than 0.1 seconds before the
supplied reference end. Only 25/128 absolute minima have successful requests
on both adjacent slots.

A post-hoc discrete-interior screen considers candidates whose preceding and
following 0.1-second requests also succeed. Its median available saving against
M is 0.124159%; no development arc reaches 1% under that screen. The 30-case
diagnostic pack likewise has zero such 1% interior savings. This is observed
timing sensitivity, not robustness to sensor, actuator or state perturbations,
and it does not retrospectively invalidate any native-successful early contact.

All seven retained M misses have successful requests elsewhere in their maps:

| Original fresh arc | M request | Successful tested powered interval | Relation |
| --- | --- | --- | --- |
| 90 | 2.7 s | 3.8–4.9 s | Too early |
| 132 | 4.0 s | 4.1–6.5 s | Too early |
| 303 | 0.5 s | 1.0–2.2 s | Too early |
| 357 | 1.6 s | 1.8–3.7 s | Too early |
| 628 | 4.1 s | 4.2–6.4 s | Too early |
| 793 | 2.9 s | 0.2–2.8 s | Too late |
| 1061 | 3.8 s | 4.8–6.2 s | Too early |

The six rejected M requests precede their observed successful interval. The
accepted-crash request follows its interval. A universal earlier/later offset
is unsupported. The successful timing union is a hindsight capability map of
the fixed backend, not a deployable decision rule or general reliability claim.

## Interpretation and delivery

The next recommendation is reliable handoff timing before fuel learning.
Fixed-end fuel improvement is small on the typical input; the larger savings
concentrate at early-contact timing edges. A follow-up design can investigate
what current-state feasibility/contact information the controller should expose
to the planner. It must preserve immediate activation and distinguish a pure
query from a committed burn. No such interface, mask or admission change is
implemented here. Joint ignition/reference-end selection remains a separate
possible experiment; the original fresh 99% reliability criterion remains unmet.

The [development HTML tree](http://starship:8000/coast-ignite-timing/loop-20261022T225427Z/campaign/report-final/report.html)
contains all 128 development maps and their best-point rich flights. It links
the [30-case diagnostic tree](http://starship:8000/coast-ignite-timing/loop-20261022T225427Z/campaign/report-diagnostics-final/report.html)
and [96 additional captured probes](http://starship:8000/coast-ignite-timing/loop-20261022T225427Z/campaign/report-probes-final/report.html).
Every map shows 81 requests plus never, native admission/contact data,
fuel by ignition or actual flight time, policy markers and links to captured
points. Summary-only cells remain explicit. Each of the 254 full capture
occurrences has one final rich payload copy; the retained 24-view pilot byte
calibration is separate. Common tree/browser code and rich templates are
unchanged. Immutable interpretation indexes retain the original generated
indexes and every rich file.

Final delivery directories link the immutable rich payloads to their original
inodes and add their own indexes/receipts; they do not regenerate or edit those
payloads. This keeps back/next navigation and cross-report links within the
accepted final edition, while retaining one physical rich copy. Logical byte
accounting counts both names conservatively. Display-only finalization improves
plot-label contrast for the common light theme and adds a native request
selector for phones. The final 390 px and desktop checks cover all 254 captured
occurrences, unique pagination, dense approach slots, all 82 choices, both fuel
axes, Summary/capture distinction, rich plots, intact terrain and final-edition
back/next navigation, with zero browser errors. Failed QA-driver attempts are
retained separately; they do not change any flight or map result.

The final read-only presentation check reproduces all three indexes exactly,
verifies 1,025 original report files unchanged and checks 1,022 immutable
payload/navigation aliases. Nine requests to the visible `starship:8000` URLs
match the local bytes exactly, covering all three indexes, navigation data,
a rich flight, its evidence and proof. Browser acceptance and source copies
are retained under `browser-qa/` and `presentation-source/`.

The maintained developer gate passes all 19 checks. Final workspace Python
validation passes 23 tests; focused documentation checks pass 13 Node tests,
287 documents and 2,219 local links, with clean diff whitespace. Retained
44-case numerical parity remains opt-in and was not run. Final resource,
workspace-preservation, local commit and owned-browser cleanup receipts are in
`closure.json` under the run root. There is no push or default promotion.
