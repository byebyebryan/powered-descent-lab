# Poly Landing implementation and evaluation execution

[Documentation home](README.md) · [Approved design](poly_landing_controller_plan.md)

October 22, 2026. The user authorized a goal loop after confirming immediate
ignition, planner-owned coast and an explicit planned touchdown/reference-end
time. This record owns the new implementation/evaluation allocation; it does
not reuse the completed standalone-controller campaign's allocation.

**Complete: the checked adapter lands 600/600 fresh flat, prepared, explicitly
timed entries with zero native burn-shape violations. The one-sided 95% exact
lower bound is 99.502%, passing the declared 99% criterion for this conditional
task.** The original interrupted development epoch remains a technical failure.
Mission-generated handoffs, disturbance tolerance, general airborne states and
RL training remain separate work.

## Scope and limits

Execute checkpoints A–F in order, advancing only through the design's gates.
The goal is a reviewed, preserved implementation and an honest bounded
qualification result, including a negative result when a gate fails. Do not
tune the controller or redefine a population after seeing its outcomes.

- Loop root: `outputs/poly-landing/loop-20261022T085313Z/`, create-only evidence.
- Starting source: `f36a4b4f6486cb8017be47afb868acebc45fbb61`; the loop root
  contains the full dirty/untracked workspace inventory and selected source
  bytes before implementation. Existing unrelated edits remain owned separately.
- Wall ceiling: eight hours from 08:53:13 UTC, with the final hour reserved for
  closeout. Stop measured work by 15:53:13 UTC and close by 16:53:13 UTC.
- One native execution worker. Compilation and documentation checks may overlap
  source inspection, but no overlapping flight campaigns or replays.
- New output ceiling: 20 GiB; additional build/source material: 10 GiB; maintain
  at least 50 GiB free. Existing retained evidence is never deleted to fit.
- Software checks and one baseline plus one post-extraction retained 44-case
  parity gate are allocated initially. Further checks require a changed source
  or a diagnosed failure; record each invocation and cost.
- Prefix recovery and 38 original-prefix adapter comparisons are a separate
  regression allocation, with their native steps and verification work counted.
- Pilot ceiling: 12 prospective development originals. Freeze recipe, schedule,
  proposal limits and per-attempt bounds before any pilot flight.
- Structured ceilings remain 256 primary originals and 96 diagnostic originals;
  final confirmation is at most one fixed 600-original sample. Neither phase is
  allocated until its prerequisite gates and measured pilot cost pass. Declare
  exact counts, transformations, proof work and cost limits before release.
- No RL training, default promotion, push, publication or server operation.
  Local report generation uses the established common mission tree and details.

## Checkpoints

| Checkpoint | Status | Evidence |
| --- | --- | --- |
| A — Contract and inventory | Pass | V2 timing/observation contract, recovered historical entries/durations, corrected numerical baseline and numeric flat recipe bound |
| B — Shared extraction | Pass | All 20 maintained checks pass, including exact 44/44 October 7 parity; three math/native-inverse tests pass |
| C — Timed adapter | Pass | 38/38 October 5 burns match complete command bits, samples, incoming contact and final state, with independent proof |
| D — Flat recipe and pilot | Pass, including declared technical repeat | Same 12 physical inputs; 12/12 verified landings and zero shape failures in both epochs |
| E — Structured characterization | Pass after one declared technical repair | 256/256 primary and 48/48 transformed landings/shape pass; all 96 diagnostic dispositions valid. Original technical stop and 141 unstarted slots retained separately |
| F — Fresh confirmation | Pass for the frozen conditional task | One fixed 600/600 fresh run, zero shape failures, one-sided 95% lower bound 99.502% |

## Work ledger

Baseline source/capture inspection is read-only. Native baseline/parity,
source-prefix recovery, adapter execution, evaluation/replay/reconstruction,
software tests and report reconstruction are recorded separately under the loop
root. A completed flight is not a landing claim without the complete native
physical/mission/integrity/replay tuple. Preserve interrupted/failed slots.

### Baseline correction before extraction

The first background launch ended before producing a gate result; no surviving
worker or child remained. Its incomplete log is retained and it is not counted
as a pass. The supervised full gate then passed all ordinary checks, but the
plan's October 5 opt-in parity target failed on its first case. The first
reported difference was an airborne clearance of
`0.0005533556252954597` versus `0.0005533556252945715` m.

Investigation found that the maintained developer workflow explicitly names the
October 7 early-exit capture. A separate, diagnosed baseline invocation of only
the retained numerical test passes all 44 cases against that source (19.21 s
test duration). This is the extraction comparator. The historical October 5
capture remains unchanged and separately bound for its original 38 burns.

The saved read-only comparison proves identical scenario bytes for all 44
inputs, identical segment ledgers for 17 and ordinary flights for 22; 43
outcome/proof tuples match. Thus the difference is broader than the first tiny
clearance discrepancy. Current planning is not claimed exact against October 5.
The loop root retains `retained-inventory.json` and
`retained-capture-comparison.json`. Legacy planner regeneration includes internal
native candidate work whose exact step count is not exposed by this old gate;
its full invocation and wall cost are separate from new local-flight counts.

### Timed adapter and historical-preservation allocation

The integrated implementation gate passed all 20 checks in 263.48 seconds,
including all workspace tests, strict Clippy, format/docs/dependency checks and
exact retained 44-case numerical parity. Full log and receipt are retained as
`implementation-gate-v1.log` and `implementation-gate-v1.json` under the loop root.

V2 has a separate timed request and current-observation callback; V1 target-only
requests and default selection are unchanged. Constructor input binds the
original first command and deadline ticks, allowing ground-launch scenarios to
admit the actual later airborne observation. Construction performs one pure
coefficient solve and first-pair inverse, with no native forecast. Later
actuation rejection and reference exhaustion retain an honest native prefix.
The original four-tick residual allowance is preserved.

The new composition uses the existing native Episode, neutral sealing/replay,
source binding and common rich detail renderer. It reconstructs controller
admission/commands from recorded requests and planner requests independently
from the frozen program. Command/trace wire comparisons retain signed zero.

Before checkpoint D, allocate exactly 38 historical October 5 burn comparisons,
one native worker, a 40-minute stage ceiling, 60 seconds per child and at most
1 GiB retained stage output. Each has one original-prefix recovery, at most
36,000 pure reference queries (actually bounded by its original remaining
deadline), one complete native adapter episode and at most seven complete
neutral/reconstruction proof passes. Record each physical prefix and all
constructor/inverse query counts separately. Frozen native inputs, all 44 roles
and historical source/capture identities remain unchanged.

Duration recovery is a separately named regression-only parameter recovery:
require a unique bit-exact first-command match from the live original entry,
then exact complete command/state/contact/sample parity and independent proof.
It does not reproduce old planner selection or introduce a duration selector
inside Poly Landing. Any failed or ambiguous recovery blocks new characterization
until its preservation issue is understood; do not relax equality or substitute
the newer capture quietly.

All 38 comparisons passed in 9.83 seconds, retaining 172.6 MB. Their original
scenario/flight bytes and executable remained unchanged. Recovery used 109,500
native prefix steps and 127,574 pure reference queries; the 38 original adapter
episodes used 178,902 physics steps. Preparation and explicit verification add
114 neutral replay passes (536,706 steps) and 152 controller/planner
reconstructions (715,608 steps). No candidate native forecasts occurred.

Recovered entries span 158.47–420.20 m COM clearance above touchdown height,
21.56–470.95 m target distance, 7.27–66.99 m/s forward speed, −53.95 to −4.72 m/s
vertical speed, 5,478.30–5,823.62 kg fuel and 7.417–26.617 s reference duration.
These are exposed historical observations, not a new sampler's bounds. All
five flat core cases and the separate narrow-pad diagnostic preserve exactly.
The loop root's `historical-entry-and-resource-summary.json` owns the exact
inventory. `source-epoch-bc-v1/` preserves the 124 participating source files and
release executable before task-preparation work changes the source epoch.

### Flat task freeze before pilot

The [prospective recipe](poly_landing_flat_recipe.md) fixes the six coupled raw
axes, conservative input-only authority/fuel/servo predicate, native 120 Hz
shape observations and `1e-9 m/s` velocity sign tolerance. Source/controller
outputs do not participate in proposal admission. Development begins with all
64 raw endpoint corners and 12 one-axis boundary proposals, followed by its
separate continuous stream. The pilot and confirmation use uniform streams.

Declare streams before pilot outcomes: pilot cohort `poly-flat-v1-pilot`, seed
`0x504f4c590001`; development `poly-flat-v1-development`, `0x504f4c590002`;
confirmation `poly-flat-v1-confirmation`, `0x504f4c590003`. Exact physical exposure
overlap with prior bound plans invalidates preparation; it is not quietly
resampled. Fresh release also binds the eligible development receipt and source.

V2 uses the existing supervised-child lifetime and timeout implementation.
Continuation may reference complete byte-bound prior results without new flight
or replay; interrupted partial originals are retained and cannot be silently
rerun. The local HTML generator uses the established common tree, twelve-item
mission pages and complete rich detail/navigation templates, with no replay or
report publication. A telemetry label was corrected from “applied” to “mapped”
thrust: actual applied fractions are independently measured from native samples,
including fuel-limited diagnostic transitions. This changes no emitted command.

The complete task-tooling source passes all 20 maintained checks, including the
new bounded CLI recovery/storage panels and exact 44-case numerical parity,
in 296.61 s (`task-tooling-gate-v2.log`/`.json`). Before the pilot, allocate one
final-source repeat of the same 38 historical burn comparisons under a new
create-only regression root. This is source-epoch preservation after adding
evaluation/report instrumentation, not another local population or a new
reliability sample. The same per-case/stage bounds apply; total regression
storage remains below the original 1 GiB stage ceiling.

The final-source repeat passes all 38 historical burns in 9.84 s. Native source
checkpoint `9f1f7c1` and its 129 participating files/release executable are frozen
under `source-epoch-df-v1/` for the remaining measurements. The pilot's declared
12 uniform proposals all admit, with four entries in each duration band. All
12 episodes land with complete proof and zero native burn-shape violations.
They use 22,444 original physics steps, 36 neutral replays (67,332 steps) and 48
controller/planner reconstructions (89,776 steps). Pilot observations make no
fresh reliability claim. Maximum child wall is 0.161 s; maximum bundle is
5,017,848 bytes.

The cost-only development release fixes 256 primary originals, 48 transformations
(16 reflection, translation and relabelling occurrences each), and 96 diagnostics
(12 each of eight named extensions). Its 264 raw proposals retain eight
input-only authority exclusions; no flight result selected inputs. The schedule
has no physical overlap with the bound pilot plan. New ceilings: one native
worker, 45 minutes, 60 seconds per child, 4 GiB stage output, 16 MiB per bundle
and 50 GiB free. No fresh input has been generated or allocated yet.

The pilot HTML confirms the intended descending continuous burn and uses the
established tree/detail templates. UI review found missing neighbour-navigation
data, overly technical contact text and misleading generic terrain-group
captions. These are presentation follow-ups, not flight/proof failures. Keep
the measured source frozen through the fixed confirmation; correct/report-review
the presentation separately before closeout. Raw incoming contact remains the
authoritative contact witness; generic post-contact display values are not
substituted for it.

### Development technical stop and bounded repair declaration

The first development run stops at diagnostic slot 258 after 259 original
episodes, in 43.35 s with 926.6 MB retained. All 256 primary originals land with
zero shape violations. The attitude/rate diagnostic reaches the exact reference
end plus four ticks while still airborne. Its physical prefix replays and both
controller/planner reconstructions pass, but the composition labels reference
expiry as `ExecutionInvalid`; complete native replay correctly refuses that
software-failure disposition. This is an evidence/operation-boundary defect,
not a physical replay disagreement. The failed receipt and all 141 unstarted
successors remain unchanged. No confirmation input has been generated.

Allocate one bounded technical repair, with no controller tuning or sampler
change. Preserve the original V2 protocol and source epoch. Add an explicitly
versioned finite command-coverage endpoint under the unchanged original hard
deadline, so the native Episode can close ordinary reference exhaustion as a
valid bounded prefix. A checked adapter admission scans the entire declared
paired-actuation/fuel window without any native forecast; it never changes
duration or the polynomial/throttle arithmetic. Unexpected software errors
remain proof faults. Regression tests must reproduce the failed mechanism and
prove no additional command, state/fuel/clock reset, or false landing.

After source review and maintained checks, allow one create-only replacement
pilot (same 12 physical inputs) and development pass (same 400 scheduled physical
inputs, roles and transformations) under a new source/protocol epoch. These
are explicit technical repeats, not new independent samples or replacements of
historical outcomes. Retain source/input equality receipts and count all work.
Repeat the 38 historical preservation checks once under that final source.
The existing per-phase wall/worker/bundle/free-space bounds apply, with a fresh
4 GiB replacement-development cap; total new loop artifacts still cap at 20 GiB.
Fresh confirmation remains unallocated until the repaired gates pass.

The repair explicitly selects timed program/config/trace schema 3 and backend
`polynomial_landing_v2`. Schema 2/backend `polynomial_landing_v1` retains its
prior wire bytes and execution semantics. Both endpoints are bound by companion
verification. Full-window pure actuation checks have separate original and
reconstruction counters; they are not native forecast steps. Tests reproduce
the tick-964 airborne diagnostic, prove complete bounded replay with no command
at tick 964, and compare admitted-burn native commands/samples/contact/final state
exactly against the original adapter.

Because a replacement source epoch is already necessary, finish the known
presentation fixes before freezing it: navigation data, human raw contact text,
post-contact labels, engine-setting labels, mission-group captions and an opt-in
saved-reference overlay in the maintained rich template. Existing callers retain
the default idealized display; report generation performs no replay or flight.

The repaired source passes all 20 maintained checks, including exact current
44-case planner parity, in 301.09 s (`technical-repair-gate-v2.log`/`.json`).
The earlier repair-gate attempt stopped at compilation because one existing
report caller needed the new optional overlay set to `None`; no campaign ran.
Browser preflight confirms the established tree, saved-reference legend, raw
incoming contact, restored detail navigation and engine-setting labels.

Repair checkpoint `7b8f277` is frozen with all 129 participating source files and
its release executable under `source-epoch-df-v2/`. The third historical stage
passes all 38 exact burns in 9.84 s. Replacement pilot: 12/12 verified landings,
zero shape failures, 1.71 s. Replacement development: all 400 slots complete,
256/256 primary and 48/48 transformed landings with zero shape failures; 96
diagnostics include 15 landings and 12 unsupported vehicle cases. Its 388 native
episodes retain complete proof, including ordinary airborne reference expiry.
It takes 65.69 s and 1.245 GB. Both input-equality receipts prove identical
scenario bytes, roles, lineage, raw proposal admission and previous timing;
only explicit protocol/backend and finite command coverage differ.

Release exactly one 600-original confirmation on the previously declared
`poly-flat-v1-confirmation` stream and `0x504f4c590003` seed. Freeze the unchanged
99% reliability criterion with one-sided 95% exact-binomial lower confidence
bound, zero native shape failures and all 600 slots complete. Allocate one
worker, 90 minutes, 60 s per child, 6 GiB stage output, 16 MiB per bundle and
50 GiB free, within the 20 GiB loop ceiling. Development maximum child is
0.201 s and maximum complete bundle is 5,245,026 bytes. Bind both old and
replacement pilot/development exposure plans and the eligible current-source
development receipt. Source, sampler and criterion remain frozen through this
single fresh run; no tuning or restart follows its outcomes.

### Final confirmation, report and limits

The single fresh stream retains 602 raw proposals: two input-only authority
exclusions and exactly 600 admitted independent originals. All four prior
pilot/development exposure plans are bound and have no physical overlap with
the fresh schedule. Its 135 / 231 / 234 entries occupy the 8–12 / 12–18 / 18–24 s
duration bands. All 600 land with complete integrity, physical replay and
controller/planner reconstruction, with zero native shape violations. The
one-sided 95% exact-binomial lower bound is `0.9950195566196385`; the declared
99% conditional reliability criterion passes. No second fresh run exists.

Fresh execution takes 118.86 s and 2.140 GB. It records 1,170,110 original native
steps, 1,800 neutral replay passes (3,510,330 steps) and 2,400 controller/planner
reconstructions (4,680,440 steps). Admission uses 586,255 original and 2,345,020
reconstruction pure actuation checks, with zero native forecast steps. The
complete loop audit counts the original failed epoch, both pilots, replacement
development and all three historical preservation stages separately. Software
gate/test work remains in its invocation logs rather than being silently
included in, or inferred from, campaign counters.

The [HTML mission tree](http://starship:8000/poly-landing/loop-20261022T085313Z/reports/final-v1/report.html)
is the user-facing report: 1,468 scheduled occurrences, including unstarted and
unsupported entries, and 1,309 full rich flight views. Rendering takes 60.05 s
with zero new flight/replay work. It preserves the common tree, twelve-item
paging, previews, complete detail controls and static fallbacks. Desktop and
390 px phone browser review pass: page-two/back context, neighbour links,
saved-reference overlay and raw contact labels work, without document overflow.
The repaired ordinary airborne miss and original incomplete-proof failure are
both visible. No report selector, publication or server operation was performed.
The audit records 7.28 GB of loop artifacts, below the 20 GiB ceiling; native
source and release executable remain byte-identical to `source-epoch-df-v2/`.

The diagnostics define the practical limit of the claim:

| Extension | Verified landings / scheduled | Other disposition |
| --- | --- | --- |
| Unprepared attitude/rate | 1/12 | Eleven native bounded misses |
| Receding horizontal motion | 0/12 | Rejected references or native misses |
| Near contact | 0/12 | Rejected references or native misses |
| Five kilograms of fuel | 0/12 | Rejected reference; planner coast remains explicit |
| Two-second reference | 0/12 | Rejected references; bounded planner coast |
| Narrow 8.1 m pad | 12/12 | Diagnostic, outside the fresh denominator |
| Unsupported dry mass | 0/12 | Twelve pre-native unsupported inputs |
| Ascending entry | 2/12 | Ten rejected references or native misses |

All 84 executed diagnostics have complete proof; twelve unsupported inputs
consume no native budget. NoPlan is a statement about this requested reference,
not a physical impossibility claim. The successful conditional task assumes flat
ground, the accepted vehicle, feasible explicit timing and prepared attitude.
It does not establish robustness to state error, disturbances or an arbitrary
mission-generated handoff.

Next specify and evaluate planner-generated flat handoffs: preserve the live
coast/preparation prefix, choose the endpoint time externally, declare admission
and NoPlan handling, and freeze bounded position/velocity/attitude perturbations
with fresh holdouts. Use the existing historical flat controls as exposed
regressions. Retain physical mission continuity and independent replay. This is
a next-pass recommendation, not a new campaign or RL allocation.
