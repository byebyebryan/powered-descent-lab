# Arrival actuation execution

This pass follows the [calibration and qualification plan](arrival_actuation_plan.md).
The create-only evidence root is
`outputs/arrival-actuation/loop-20261024T021508Z`.

The retained workspace baseline binds initial head `d9f2d1b`, all 903
Git-visible file hashes and pre-existing foreign changes. The outer deadline
is Unix 1792815109. Limits are 100 million measured native/model steps, 1,000
original occurrences, 64 full captures, 4 GiB artifacts, one native worker,
zero optimizer steps and at least 40 GiB free space.

The frozen phases and their completed results are recorded below.

## Frozen campaign declaration

The retained comparison has all 198 programs from the previous full and fresh
panels. Their mission bytes, requested goals and legacy Summary hashes remain
unchanged. The former fresh panel is now exposed regression data. Pilot inputs
are core cases 100, 110, 125 and 135 with `p35-v70`, in both inspected legacy
and opt-in native-pair modes. Two fixed corrected pilot captures are 100/125.
The pilot receives ten million steps and five minutes after binary readiness.

The main comparison receives 70 million steps and 16 fixed corrected captures:
four angles × low/high energy × clean/+75 m miss, all with `p35-v70`.
Direct programs are preservation controls, not a new timing policy.

Witness construction uses 21 declared recipes: vertical motion at 15/30/45 m/s
sink, plus lateral, braking and attitude-preparation motion at those sinks in
both reflected directions. Reflections are paired diagnostics. Every recipe
runs six seconds; preparation recipes coast/point for the first half second.
Native endpoints define goals with the existing 30 m and velocity tolerances.
Construction must satisfy the unchanged complete fly-through predicate before
being admitted. Exclusions are retained without replacement. Witness, legacy
controller and corrected controller each execute every admitted goal. Capture
the independent witness and corrected controller for each, regardless of
outcome: at most 42 witness-phase captures and 20 million steps.

The exact declaration and input hashes precede any measured execution.
Maximum capture roles across phases are 60. Native calibration covers minimum
fractions 0/0.25/1, six fuel states, seven acceleration demands, aligned/rotating
attitude and both profiles: 504 command pairs and 1,008 native steps.

## Implementation and calibration

`ArrivalActuationProfile::NativePairV2` is an explicit opt-in constructor path.
The ordinary arrival constructor, stored transfer configuration and existing
Poly Landing behavior retain legacy allocation. The correction is shared by
arrival command execution and its candidate forecasts; it does not replace
the 60 Hz approximate forecast with the complete 120 Hz native plant.

One frozen runtime epoch, `source-epoch-a`, is built from `0dd9823`. Its native
library SHA-256 is
`ad21db3f19ecd6e8afc1b43fb62d384302b1408f4b998f92850ddb1d7fbabdd3`.
The runtime ledger binds all native and campaign Python source bytes. Later
audit, analysis and presentation edits are read-only consumers of that epoch.
They do not alter captured executions or the campaign inputs.

All 504 calibration pairs pass. Maximum aligned forward-model error is
1.004e-13 m/s²; maximum aligned realizable-demand error is 1.191e-13 m/s²,
against the declared 1e-8 bound. Rotating cases verify native rate-limited
motion without claiming instantaneous vector alignment. Fuel-edge and
minimum-thrust cases retain explicit allocation regimes and residual error.
Across corrected witness/main executions, maximum observed aligned tracking
error is 1.055e-12 m/s². The largest recorded engine-switch count is two.

The four pilot legacy results reproduce the old compact summaries and command/
state fingerprints exactly. All 69 main direct Poly controls also reproduce
their legacy results exactly. No old input, goal, outcome or fixture is changed.

## Independently witnessed local arrivals

All 21 declared recipes demonstrate the unchanged arrival predicate under native
physics; there are no construction exclusions or replacement cases. Every
independent witness and corrected controller has a full selected capture.

| Family | Independent witness | Legacy controller | Corrected controller |
| --- | ---: | ---: | ---: |
| Vertical | 3/3 | 1/3 | 3/3 |
| Lateral | 6/6 | 4/6 | 6/6 |
| Braking | 6/6 | 6/6 | 6/6 |
| Attitude preparation | 6/6 | 2/6 | 6/6 |

This establishes bounded local instruction tracking on six-second native
trajectories, with the original 30 m/velocity/heading tolerances. It is not
generalized state control, independent statistical confirmation or a composed
landing result. Reflected recipes remain paired diagnostics. Landing is
disabled on this cohort; every run continues engine-off to its original native
end, so later tail crashes are separate from local goal fulfillment.

## Retained comparison and regressions

The main phase completes all 198 programs. These include 129 arrival programs
and 69 direct controls; multiple goals on one original state are correlated.
The former fresh cases are exposed regression inputs in this pass.

| Population | Arrival programs | Legacy reached | Corrected reached | Legacy native landing observations | Corrected native landing observations |
| --- | ---: | ---: | ---: | ---: | ---: |
| Core | 96 | 21 | 32 | 77 | 70 |
| Paired reflections | 9 | 1 | 1 | 7 | 7 |
| Exposed regression | 24 | 4 | 6 | 19 | 18 |

Arrival fulfillment improves from 26/129 to 39/129: fourteen gained and one
lost. Reached-and-native-success improves from 24 to 37. Total native landing
observations on arrival programs decline from 103 to 95: nine lost and one
gained. Summary observations remain distinct from complete captured proof.
Direct native success stays 63/69 with exact execution parity.

| Core entry angle | Legacy arrivals | Corrected arrivals | Legacy to corrected native landing observations |
| --- | ---: | ---: | ---: |
| 0° | 10/24 | 20/24 | 23 to 23 |
| 30° | 11/24 | 12/24 | 22 to 21 |
| 60° | 0/24 | 0/24 | 16 to 14 |
| 80° | 0/24 | 0/24 | 16 to 12 |

All nine newly lost native landings resolve the arrival as a plane miss and
the unchanged finite Poly continuation search returns no plan. Eight use zero
fuel, selecting `gap_off` on every arrival update. Core case 131 `p60-v40` is
the remaining mixed allocation case. The complete nine identifiers and their
regime counts are retained in `analysis.json` and the HTML report.

Source and diagnostics explain the zero-thrust mechanism: the preserved
geometry projection clips negative vertical demand and bounds lateral authority
using `ay.max(0.2)`. Large lateral/downward requests can therefore project to
only 0.1754 m/s². Legacy passes that small nonzero fraction as the engine setting
and receives near-minimum thrust. The corrected inverse instead selects off
as the nearer realizable scalar demand. This removes incidental braking and
changes the state offered to continuation. Calibration alone cannot repair
that guidance interaction or establish whether a different trajectory is
feasible under the fixed tilt bound.

Core 109 `p35-v70` loses local arrival fulfillment while still completing a
native landing. Core 108/115 `p60-v100` reach their arrival but have no selected
Poly continuation, as in the legacy comparison. These are explicit examples
of the difference between local fulfillment and safe composition.

The final diagnostics show 88 plane misses after an earlier ideal reference
pass and two expirations while the last ideal and approximate actuator
predictions still pass. These are model/recovery warning signs, not a causal
isolation. Local horizon, absolute operation deadline and native end remain
different clocks. Tilt, horizon and recovery logic were deliberately frozen.

A post-hoc read-only finite-bank scan recovers no direct miss. Its only
eligible Summary fuel improvement over the first-feasible direct selector is
the previously exposed case 302: 721.684408 kg versus 1,034.574484 kg. This is
neither a new selected policy nor an efficiency optimum. No RL or timing
optimizer has been run, and no new generalization claim follows from it.

## Proof, checks and delivery

All 60 scheduled captures pass complete native physical replay, integrity,
controller reconstruction, input-goal reconstruction and full Summary equality.
Thirteen selected captures verify the complete target-landing tuple; the
remaining captures deliberately retain failed landings or witness tails.
The independent read-only audit additionally recomputes arrival status and
resolution tick from native observations, checks input/source/proof hashes,
and reconciles every charged step. Its result is `evidence-audit.json`.

All 19 maintained development checks pass. Focused control tests pass 197 unit
tests plus ordinary frozen-adapter parity; all five old/new native Python
tests pass. The 44-case retained numerical capture parity remains optional
and was not used as a prerequisite or relabeled as new evidence.

The [HTML report with the established mission tree](http://starship:8000/arrival-actuation/loop-20261024T021508Z/report-delivery/report.html)
contains every selected rich flight, comparisons, allocation calibration and
the failed cases. Shared report templates, old reports, selectors and server
lifecycle are unchanged. Desktop 1440×1000 and phone 390×844 browser checks
visit all 60 occurrences and exercise four rich details each, with no page
errors or horizontal overflow. All 60 complete native captures are checked
over the visible hostname against their sealed hashes.

The [immutable published findings](http://starship:8787/reports/12d8e056c468466ec263ae5491749794acb105a549ba6b28ce9c219d91588263/)
retain the summary, calibration figure and artifact manifest, linking to that
complete canonical tree. Rich report files are 98,947,085 bytes and complete
native captures add 199,444,233 bytes; their combined bundle exceeds the
publisher's 100 MiB limit. The published index deliberately preserves the
canonical raw-capture links instead of serving a partial copy of the tree.
All four published assets pass hostname byte checks. Browser checks also
verify the published entrypoint, tree link, next detail and back navigation
at both viewports.

The first presentation attempt retained its rich output but could not import
Matplotlib in the pinned simulation environment. A new `report-delivery`
directory uses an isolated presentation environment, leaving the pinned
environment unchanged. The first browser QA attempt retained a selector-quote
fault; the corrected QA script passes in `browser-qa/check-b`. Neither fault
executes new native flight work. The owned disposable browser is cleaned up
after delivery; existing report servers and user browsers are preserved.

## Resources and decision

Measured campaign work is 65,416,717 steps: 1,102,610 native plant steps,
60,725,917 local model steps, 3,238,771 selection forecast steps and 349,419
neutral replay steps. Witness construction and reconstruction are included
in native steps. There are 329 original occurrences, 120 reconstructions and
60 full captures, with one native worker and zero optimizer steps. Measured
phase wall times total about 40.18 seconds; implementation, build, review and
presentation time are separate. Per-update controller CPU cost is uninstrumented.
The final `closure.json` binds elapsed time, retained artifact bytes, free
space, local commit heads and baseline preservation checks. No push is performed.

The scalar allocation and narrow witnessed-arrival gates pass. Broad composed
landing qualification does not pass, so the correction stays opt-in and the
legacy default remains selected. The 60°/80° gap and native landing regressions
are retained, without tolerance changes or post-outcome tuning.

The recommended next pass is to define a restricted landing-compatible arrival
envelope: validate both reaching a goal and a native continuation from it,
then investigate allocation-aware projection/recovery on the retained failures.
The planner/controller split remains valid, but targets need a meaningful
choice set with demonstrated downstream viability before an arc-shaping RL
experiment. This does not require solving arbitrary state control or relaxing
the tilt limit now. Further control changes, broader goal discovery and RL
training require their own pass.
