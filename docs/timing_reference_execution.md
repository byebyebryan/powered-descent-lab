# Timing reference execution

This pass follows the [frozen qualification plan](timing_reference_plan.md).
Its create-only root is `outputs/timing-reference/loop-20261024T185303Z`.
The baseline binds initial head `2a87566`, 934 Git-visible file hashes and all
foreign changes. The outer deadline is Unix 1792878783; limits are one billion
charged steps, 1,200 captures, 12 GiB new artifacts, one native worker and a
40 GiB free-space floor. Fitting and RL updates remain zero.

## Initial declaration

The 63/132-pair comparison, 1% reference-selection rule, 300 independent primary
draws, 60 boundary cases, retained handoff regressions and isolated measurement
protocol precede all new campaign outcomes. No qualification inputs are generated
until the reference is frozen. Controller behavior, clocks, tolerances and old
outcomes remain fixed.

## Pilot and campaign release

Implementation commit `eac8470` adds the named 132-pair opt-in profile and
measurement sidecars. Five targeted Rust tests, strict targeted Clippy and nine
native Python tests pass, including full proof and measured/unmeasured identity.
The compiled source ledger points into the retained `source-epoch-a` copy.

The fixed pilot completed 12 Summary executions and four expanded captures with
eight reconstructions: 3,630,876 charged steps, 16,217,782 logical bytes and
2.875 s measured campaign time. All four captures verify target landing.
Maximum pilot forecast work is 14,172 / 112,016 / 200,584 steps for legacy,
63-pair and expanded selection respectively. This pilot establishes cost only.

`campaign-release.json` freezes phase caps before further outcomes: development
90 million, isolated benchmark 70 million, primary 500 million, boundary
100 million and retained regressions 100 million steps. Including the pilot's
25-million cap, reservations total 885 million within the one-billion outer
limit. Capture reservations total at most 1,165. The original three-hour,
12 GiB and free-space limits remain. Primary allocation is 1.667 million steps
per original, covering Summary, Captured, both reconstructions and neutral proof
for all three selectors. No primary inputs have been generated at release.

Development also reproduces both old profiles on all 180 old programs, rather
than checking only the 60 direct originals. This preservation check does not
expand the reference-selection or qualification denominator.

### Development reservation stop

The 90-million development phase stopped at 88,044,297 charged steps because the
next first frame requires a two-million-step reservation. It retained 407 complete
Summary rows and 58 captures with 116 reconstructions. The aborted attempt was
original 058's 63-pair Summary: its selection spent 90,894 forecast steps and
its first command advanced two native ticks before the next frame's reservation
failed. These partial costs remain in the fault receipt; this attempt makes no
landing claim. The pilot contained direct inputs and underestimated
the cost of reproducing the 120 old arrival programs twice.

`development/fault.json` remains unchanged. A separate create-only completion
phase reserves 20 million of the unused outer reserve for the exact 13 remaining
Summary slots and two expanded captures. Total reserved work becomes 905 million,
still within one billion. No completed slot is rerun, no input or selector is
changed, and no qualification inputs are opened before reference freeze.

The exact remaining schedule completed in its separate root. Both old selectors
reproduce all 180 retained programs exactly (360 complete executions).
All 60 expanded development captures verify target landing. Mean direct fuel
falls from 589.9276 to 570.1138 kg, a 3.3587% saving; the predeclared 1% rule
therefore selects the 132-pair reference. `reference-freeze.json` binds this
decision, source/library hashes and the input generator before any qualification
inputs are generated. The legacy default remains unchanged.

## Isolated measurements

The maintained developer gate passes all 19 checks. Isolated collection then
completed all 360 sequential benchmark invocations, charging 35,057,610 steps.
Every repeat preserves its profile's decision fingerprint. Each profile has
84 measured selections after its first invocation and two warmups per original.

Median selection wall time is 4.459 ms for legacy, 82.083 ms for 63 pairs and
109.920 ms for 132 pairs. Ordinary controller-frame medians are approximately
0.6 microseconds, excluding actuator allocation, native plant stepping and
Python/persistence overhead. The search is a planning cost and exceeds a 60 Hz
frame budget; these measurements do not establish real-time scheduling.
Raw durations, planning-frame durations, whole-episode process CPU, work counts,
affinity and host/load context are retained separately from deterministic info.

After reference freeze, input-only generation produced 300 independent primary
draws and 60 endpoint cases without native outcome admission or replacement.
All 360 physical initial states are unique and separate from exposed inputs.
The primary cells have 18–37 draws rather than fixed quotas; the engineering
panel has five per cell. All qualification profiles and retained regression
slots are bound to immutable allocations before execution.

## Qualification results

The frozen reference verifies all 300 independent primary originals. The
one-sided 95% exact lower bound is 99.0064%, passing the predeclared 99% gate for
this fixed mixture and plant. Legacy and 63-pair controls also verify 300/300.
All three profiles verify all 60 boundary cases. The 21 retained arrival
instructions are reached and followed by verified landing; all 21 prefixes
preserve native samples/commands, clocks, fuel and actual handoff state.

| Primary profile | Verified originals | Mean mission fuel kg |
| --- | ---: | ---: |
| Legacy first feasible | 300/300 | 739.6625 |
| Fuel-ranked 63 pairs | 300/300 | 589.4604 |
| Frozen 132-pair reference | 300/300 | 567.4842 |

All pairs succeed, so the efficiency comparison needs no success conditioning.
The reference saves 172.1783 kg (23.2780%) against legacy, with paired
original-level bootstrap 95% interval 22.2279–24.3249%. Against 63 pairs it saves
21.9762 kg (3.7282%; interval 3.2925–4.1674%). Bootstrap uses 10,000 paired
resamples and seed 2310241907. A separate boundary/synthetic check confirms that
failed pairs cannot earn low-fuel crash credit in the analysis.

Selected primary coast is 2–7 s, below the expanded 10 s boundary. The new 14 s
burn duration wins on 167/300 originals. Coast and duration matter jointly;
neither later-is-always-better nor continuous/global optimality is established.

Incoming reference contact speed is 1.5 m/s, with maximum absolute lateral speed
below 1.3e-13 m/s, attitude 0.000620 rad and pad offset 0.000207 m. Legacy maxima
are 2.493 m/s contact speed and 9.438 m pad offset. These measurements precede
the native final-velocity reset; original contact limits remain unchanged.

The read-only evidence audit passes exact schedules, all source/library/input
bindings, native first-arrival predicates, finite-bank rankings, selected
forecast/actual equality, physical/mission/integrity/complete replay, both
reconstructions and accounting. It includes the interrupted development attempt.
Totals: 1,165 new verified captures, 2,330 reconstructions, 360 exact retained
profile reproductions, 21 prefix proofs and 577,894,322 charged steps. Fitting
and RL updates remain zero. No default is promoted and nothing is pushed.

These results qualify a bounded offline reference. They do not establish 99%
reliability for each family or endpoint, arbitrary approach shaping, repeated
instructions, uncertain dynamics or other vehicles. Full search costs remain
planning work; an online planner would need an explicit scheduling or amortization
contract before reusing it frequently.

## Delivery and closure

The [complete established mission tree](http://starship:8000/timing-reference-report-20261024T185303Z/report.html)
contains 1,285 rich occurrences: 1,165 new captures plus 120 retained direct
controls completing the exposed three-profile comparison. Four pilot repeats
are labelled separately. The report keeps native contact/source links, rich
plots, focused equal-metre previews and the maintained bounded tree browser.

An immutable [findings snapshot](http://starship:8787/reports/4a36215061f1dda50bd47e71aeaa0193602971a2798e330521b64721a55dd4cc/)
links the complete tree and provides the same checked analysis, figure and
360-original comparison picker. Both canonical tree and published snapshot pass
desktop 1440×1000 and mobile 390×844 browser checks. All 1,285 occurrences are
browsed at each size; nine representative rich details per viewport exercise
plot modes and next/back navigation. Visible lazy previews are decoded and
painted before screenshots, and the published tree link reaches the full report.

Hostname HTTP checks verify 6,437 files/URLs (8,089,690,180 bytes): all 5,147
canonical report files, all 1,285 linked raw native captures and all five
publication files match local SHA-256 and size. The existing report service is
reused at PID 37689; the static server remains PID 37637. No server lifecycle
change is performed. The source epoch, independent proof/presentation sources,
input/reference freezes, allocations, budget fault, audits, raw timings,
screenshots and delivery receipts remain under the create-only root.

Validation includes all 19 maintained developer checks, nine native Python
tests, five targeted Rust composition tests, 13 relevant docs/gate Node tests,
statistical boundary/failed-pair checks and whitespace/docs-link checks. The
final closure receipt binds local commits, source evidence, total artifacts
and preservation of every unrelated baseline file and Git status entry.
