# Poly Landing implementation and evaluation execution

[Documentation home](README.md) · [Approved design](poly_landing_controller_plan.md)

October 22, 2026. The user authorized a goal loop after confirming immediate
ignition, planner-owned coast and an explicit planned touchdown/reference-end
time. This record owns the new implementation/evaluation allocation; it does
not reuse the completed standalone-controller campaign's allocation.

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
| D — Flat recipe and pilot | Native/task/cost gates pass; presentation follow-up queued | 12/12 verified target landings, zero shape violations; 1.69 s stage wall and 41.1 MB output |
| E — Structured characterization | Allocated/in progress | 256 primary originals, 48 named transformations, 96 separate diagnostics; 264 raw proposals with eight input-only exclusions |
| F — Fresh confirmation | Unallocated | Requires frozen source, task and criterion |

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
