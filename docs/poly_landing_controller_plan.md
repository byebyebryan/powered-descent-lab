# Poly Landing controller and flat-ground evaluation plan

[Documentation home](README.md) · [Planner/controller design](maneuver_execution_design.md) · [Previous standalone-controller results](controller_evaluation_results.md) · [Accepted planner evidence](waypoint_v2_session_repair_results.md)

October 22, 2026. **Implemented; the frozen flat, prepared timed-entry task
qualifies at 600/600 fresh landings with zero shape failures and a 99.502%
one-sided 95% lower bound. Mission handoffs and broader robustness remain
unqualified.**
The name is **Poly Landing**, with `PolynomialLandingController` as its Rust
name. The historical adapter uses explicit backend `polynomial_landing_v1`;
the checked finite-coverage adapter uses `polynomial_landing_v2` with timed
program/trace schema 3. The shared burn and timed adapters are implemented;
the [execution record](poly_landing_execution.md) owns the measured qualification
and its conditional scope. The existing standalone facade remains available.
The subsequent [execution declaration](poly_landing_execution.md) owns the
authorized loop, resource limits, gate status and measured allocations.

The subsequent [coast and ignition study](rl_coast_ignite_execution.md) completes
the first hybrid experiment using this fixed burn: 589/600 verified fresh target
landings. Its [plan](rl_coast_ignite_plan.md) keeps coast actuation controller-owned
and supplies the absolute touchdown/reference time from the mission. Generalized
control remains separate from this supplied-arc task.

## 1. Correct the baseline before another evaluation

The intended reference is the accepted planner's finite terminal burn, not
`TerminalPdgController::default()`. The previous interface and characterization
wrapped the latter. Neither existing landing algorithm was rewritten in that
work; the backend selection was different from the intended reference.

| Existing path | Source and behavior | Evidence that belongs to it |
| --- | --- | --- |
| Accepted finite landing burn | [Terminal reference and paired actuation](../pd-planner/src/planner_flight/terminal.rs), executed by [airborne realization](../pd-planner/src/planner_flight/airborne.rs) | Accepted Policy 3 flights and their original command/source-replay receipts |
| Standalone terminal feedback | [Terminal PDG](../pd-control/src/terminal/mod.rs), including receding horizons, ZEM/ZEV requests, nominal allocation, rescue and settled descent | Existing terminal packs, the [standalone facade](../pd-control/src/maneuver/landing.rs) and the completed 598/600 characterization |

The accepted burn uses a fixed-duration, discrete polynomial reference:
quadratic horizontal acceleration and affine vertical acceleration. Horizontal
endpoint position/velocity and zero ending horizontal acceleration are coupled;
the vertical reference targets the existing touchdown-center convention and
downward contact speed. This is more specific than the ZEM/ZEV label used for
the standalone acceleration helper.

The paired throttle inverse accounts for the native positive-setting mapping,
fuel consumption and the masses used in the two 120 Hz steps under one held
60 Hz command. Preserve that arithmetic. Do not copy the standalone allocator
into this backend or add another inverse conversion at the common facade.

The old 598/600 result and failed confidence criterion remain true for their
declared standalone-controller population. They are not Poly Landing results.
The new backend and task receive separate names, schedules, receipts and reports.

## 2. Source review and saved evidence

Review checkpoint: `f36a4b4`. A diff from before maneuver implementation
`83303da` through this checkpoint contains no changes to terminal PDG,
`guidance.rs`, the plant, or the accepted planner's terminal implementation.

Implementation preflight corrected a baseline distinction: the maintained
44-case numerical regression uses `capture-early-exit-20261007-native`, as
documented in [development](development.md#optional-retained-flight-regression).
Unmodified `f36a4b4` matches that capture exactly. The October 5 capture below
owns historical accepted evidence, not exact complete-record parity for current
planner selection. All 44 scenario files are identical between captures, but
only 17 segment ledgers and 22 ordinary flights are identical; one diagnostic's
outcome/proof tuple differs. Preserve each capture and its identity. Do not
relabel October 7 results as a replay of October 5's 38 burns.

Read-only inventory of
`outputs/eval/planner_v2_lab_suite/capture-session-repair-20261005-native/`
found 38 saved target landings: 36 mandatory cases and two diagnostic landings.
Every consumed terminal command in those 38 burns is powered, and their saved
integrity and final-source-replay fields pass. Observed consumed powered windows
range from 7.417 to 26.617 seconds. These are historical records, not new replays
or a reliability estimate for another population.

Two readable examples are:

| Case | Consumed terminal interval | Powered 60 Hz commands |
| --- | --- | --- |
| `v2_clear_685` | 17.850–30.350 s | 750; no engine-off command |
| `fresh_clear_flat_805` | 18.100–32.600 s | 870; no engine-off command |

The input and capture review also found:

- All 38 saved landing cases use 7,200 kg dry mass. The accepted full VehicleSpec
  is fixed by [nominal input validation](../pd-planner/src/planner_flight/input.rs).
  Three-payload coverage from standalone terminal tests does not transfer here.
- Six saved landing scenes are entirely flat at target-pad height: five core
  controls and `v2_diag_narrow_target`. The diagnostic remains separate.
- The accepted coast phase turns toward the first planned burn command before
  ignition. An arbitrary upright or tumbling entry is a different condition.
- Only 10/38 captures contain a saved sample exactly at the first burn command.
  Recover other exact entry states by original-source forward replay; never
  interpolate sparse samples or present a reset snapshot as a mission handoff.
- Accepted ignition is aligned to the 60 Hz command clock, not necessarily the
  example's 10 Hz planner grid. Exact preservation must not round an old burn
  to the nearest 10 Hz request tick.
- A consumed command interval need not equal the reference's selected duration:
  first native contact and the existing post-reference allowance matter.
  Recover and bind the selected duration/reference through the original
  deterministic construction, checking its saved proposal/command identities.
- The current [nominal selection](../pd-planner/src/planner_flight/canonical_initial.rs)
  and [acquisition timing](../pd-planner/src/planner_flight/acquisition/terminal_time.rs)
  jointly consider prefix/coast motion and duration. The reference builder alone
  is not an autonomous target-only landing controller.
- The current maneuver `submit(request, tick)` has no current observation;
  submission happens before `frame(observation)`. A finite construction cannot
  safely use the previous frame's cached observation at activation.
- The existing standalone constructor checks that the scenario's initial state
  is airborne. That rule cannot establish a live handoff after a ground launch;
  Poly Landing must validate the actual activation observation instead.
- Existing admission distinguishes Accepted/Busy/Unsupported/InvalidRequest,
  but has no ordinary finite-construction rejection. A valid requested burn
  that cannot be constructed must not become an implementation error or a
  physical-impossibility claim.

## 3. Confirmed first timing contract

The user confirmed both timing choices on October 22, 2026. The request encoding
and numerical entry limits remain implementation-design deliverables.

**Activation starts the powered burn at the current native command boundary.**
The planner owns preceding coast time. The controller does not insert an
engine-off alignment phase, acquisition burn, route search or hidden retry.
Prepared attitude is part of the first capability's entry contract.

**The planner supplies an explicit absolute planned touchdown time for the first
adapter, expressed as the reference-end time.**
The old planner already selected a terminal duration. Preserve that decision
as a declared input rather than inventing a new duration selector inside a
supposedly behavior-preserving wrapper. At activation, derive the local reference
length from the requested absolute clock and actual entry tick. Native episode
time, fuel and deadline are never reset.

The timing input expresses the finite reference's intended endpoint. Native
first contact remains authoritative; it is not required to coincide with an
invented exact floating-point timestamp. Preserve and explicitly document the
existing complete-command-pair convention, contact handling and maximum four
post-reference physics ticks, bounded by the original mission deadline.
Measure contact-time residual separately from the landing verdict. Do not
infer a new timing tolerance from outcomes after a run.

| Alternative | Consequence | Decision |
| --- | --- | --- |
| Explicit reference-end time | Existing selected burns can be reproduced without changing duration policy; caller's timing decision is visible | Confirmed for the first adapter and realization evaluation |
| Target-only request with internal duration selection | Requires a new candidate family, ranking, feasibility/NoPlan semantics and resource budget | Separate follow-up, not hidden in extraction |
| Internal coast before ignition | Changes activation semantics and competes with the planner's timing decision; also changes attitude preparation | Excluded from the first capability; the planner owns coasting |

This makes the first question precise: can the interface construct and execute
the established finite landing burn for a declared current state, target and
reference-end time? It does not yet answer whether an autonomous duration policy
or learned activation policy chooses useful times.

## 4. Components and ownership

Keep planner/controller as the two flight-decision layers. The constructor and
executor below are internal parts of the Poly Landing backend, not new public
operator/executor interfaces.

| Component | Proposed ownership | Responsibility |
| --- | --- | --- |
| Shared finite-burn implementation | `pd-control/src/poly_landing/{reference.rs,actuation.rs,mod.rs}` | Existing coefficient construction, reference evaluation and paired throttle conversion, using core vectors/state inputs without planner dependencies |
| Accepted planner compatibility adapter | Existing `pd-planner/src/planner_flight/terminal.rs` and its callers | Retain existing duration/coast selection, guards, policy identities, DTO encodings and public exports; call the shared implementation |
| Maneuver lifecycle adapter | `pd-control/src/maneuver/poly_landing.rs` plus scoped contract changes | Bind current observation, timing, target and backend identity; initialize once; emit one held native command per boundary |
| Composition and verification | Existing `examples/maneuver-policy` | Explicit backend selection, deterministic requests, source/config binding, complete original Episode and reconstruction |
| Input preparation and assessment | Existing evaluation modules and neutral scenario interchange | New flat task, roles, lineage, audit, budgets and independent outcomes |
| Presentation | Existing common mission tree and rich details | Complete inventory, previews, timing/command diagnostics and evidence navigation |

`pd-control` currently depends on `pd-core` and `pd-run`; retain that direction.
It must not import `pd-planner`, `pd-eval-cli` or planner-specific `pd-plan` DTOs.
Use small core-based state inputs at the shared burn boundary and conversion
adapters in `pd-planner`. Keep terrain/body-reserve orchestration in the planner;
do not move the entire current `terminal.rs` dependency closure into control.
Neutral runtime/evaluation/report crates remain independent of strategies.

Preserve coefficient operation order, two-tick averaging, 48 inverse iterations,
fuel/mass treatment, current endpoint convention, reference indexing and legacy
serialized field order/identities. Re-export old types or keep thin compatibility
wrappers where necessary. A shortened display name must not rewrite historical
policy identities such as `body_aware_quadratic_terminal_held60_v1`.

This is a fixed-reference executor under the current deterministic plant. It
does not add positional feedback, disturbance rejection, replanning, smoothing,
retuning or a fallback to standalone PDG during the burn.

## 5. Request, lifecycle and evidence changes

Specify and review these changes before writing the adapter:

1. Add a versioned timed-landing request/capability with target and absolute
   reference-end tick. Keep old target-only V1 requests, descriptors and saved
   reconstruction readable and behaviorally unchanged. Unsupported timing goals
   must be explicit for backends that do not implement them.
2. Supply the actual current observation to admission, or use one atomic
   request-plus-observation boundary. Do not advance the backend just to cache
   state, reconstruct an old observation, or build against a stale tick.
   Bind context/hardware at construction and assess prepared airborne entry at
   actual activation. Initial source-pad rest is not an unsupported landing
   entry merely because the original episode began there.
3. Distinguish malformed input, unsupported capability, valid finite
   construction rejection (`NoPlan`), accepted execution, actual native end,
   and reference exhaustion without contact. `NoPlan` means this declared
   construction did not supply an executable burn; it is not proof that no
   controller could land. Adopt explicit new contract versions where wire
   changes would otherwise reinterpret old receipts.
4. Require future, aligned reference timing within the original horizon.
   Construction and any prospective limit checks occur once at admission and
   have bounded work. They must not execute an unaccounted flight inside a
   command callback. Actual physical execution and replay stay with the harness.
5. On successful admission, bind reference identity, entry state/clock, target,
   timing, complete configuration and source. Duplicate requests preserve that
   reference. A busy or rejected request cannot reset the burn or replace clocks.
6. At tick `s`, use local reference index `s - activation_tick`, while preserving
   global physics ticks. Use actual observed mass with the existing paired
   actuation. Do not re-fit position/velocity or extend the arrival clock.
7. Before accepted activation, use the explicitly declared coast owner. On
   `NoPlan`, record the disposition and continue the declared original episode
   without automatic retry. Retry is a new, scheduled planner decision.
8. Native contact/end preempts remaining commands. If the reference and its
   original bounded residual window end without contact, record an operation
   failure and an honest bounded physical prefix. Do not append a settling
   controller, invent a crash, or extend the original deadline.

For original-prefix preservation, the harness needs an explicit prefix command
owner followed by exactly one handoff to Poly Landing at the live burn boundary.
That regression adapter consumes the bound historical prefix for comparison;
it is not a deployed planner or evidence that a new planner chose those commands.
Initialize the controller's lifecycle at that actual command boundary without
recreating the physical Episode or resetting its ledgers. Before handoff, the
prefix owner executes the original attitude preparation; an upright-coast
substitute would invalidate exact preservation. A separately named synthetic
local task may start airborne at tick zero, but must not be labelled a replayed
or integrated original mission.

The preservation driver can submit the bound historical request on its exact
60 Hz burn boundary. Label it as a regression driver, not a new 10 Hz planner's
decision. Keep the ordinary 10 Hz activation programs unchanged and measure
their decision-clock restriction separately if later integrated. Otherwise the
test would change ignition timing to accommodate the harness while claiming an
unchanged burn. A future scheduled-ignition/armed state would be a separate
interface change; it is not implicit in immediate activation.

Bind reference coefficients, requested/actual clocks, emitted engine setting,
applied thrust fraction, controller work and operation disposition in the new
trace. The evaluator independently checks the native physical/mission/integrity/
replay tuple. A successful constructor or matching coefficient hash is not a
landing. New backend/contract identities must select the corresponding
reconstructor; unknown identities fail rather than silently selecting PDG.

## 6. Revised mission populations

The first measured task is **flat, prepared, timed local landing**. Flat ground
is still native terrain for collision/contact; pad surface and ground agree
over the complete footprint. No ridges, backstops, route obstacles or terrain
shaping are part of its primary denominator.

| Population | Contents | Claim and denominator |
| --- | --- | --- |
| Historical behavior preservation | All 44 accepted pack inputs: 36 core landings, two diagnostic landings, four finite stops, two unsupported inputs; exact adapter checks on the 38 existing burns | Preserve original roles, commands and proof tuples; no fresh reliability claim |
| Flat realization core | The accepted 7,200 kg dry-mass vehicle, 120/60 Hz and Earth gravity; valid remaining fuel, flat ground, ordinary pad width, descending forward entries and attitude prepared for the first commanded vector; explicit reference-end time | Qualify execution for this declared local family; no arbitrary-arrival or whole-mission claim |
| Structured development | Interior/boundary and coupled position/velocity/time/fuel cases within that task; label-only and translation invariances retain lineage | Every original and transformed occurrence stays visible; copies are not independent trials |
| Separate frontier/extension diagnostics | Attitude mismatch/rate, reverse or initially receding motion, near contact, low fuel, short timing, nonstandard pad width, other dry masses and ascending entries | Expose rejection and failure mechanisms; not folded into primary reliability |
| Fresh confirmation | Independent draws from the frozen flat-core joint sampler, including declared timing inputs; no access to development outputs during release | One fixed test of that exact population |
| Later planner decision study | Real prefixes plus an explicit coast attitude policy, timing-choice policy and equal-controller comparisons | Deferred until the first entry/timing capability is established |

Define numeric bounds, joint correlations, axes and weights in the new recipe
before any development flights. Recover actual accepted entry/timing data first;
do not reuse the old 5–800 m / three-mass terrain-mixture recipe with the terrain
field merely set to flat. The broader standalone sampler was not the accepted
Poly Landing handoff contract.

Use two distinct sources of flat evidence:

- Actual flat mission prefixes from the accepted inputs establish preservation
  and integration. They are already exposed regression data. Keep the narrow-pad
  diagnostic separate, and replay from the original source to the entry boundary.
- New reference-defined flat tasks may sample declared entry/timing families
  using input-only discrete endpoint/authority constraints. Record that the
  generator is related to the polynomial reference family. Such tasks establish
  realization within that family, not generalization to arbitrary airborne
  states. Timing is an explicit request input, never an undisclosed oracle.

For new tasks, retain every proposal and every input/support disposition.
An input-only admission predicate must be prospectively declared and applied
without executing the controller and keeping its successes. Freeze accepted
draw count, maximum proposals and treatment of underfill. A primary `NoPlan`,
missing proof, timeout or unsupported disposition after scheduling cannot be
removed from its denominator or moved to diagnostics after inspection.

Arbitrary valid flat entry tuples can be studied as diagnostics without a
feasibility guarantee. Keep constructor coverage, target-landing success over
all scheduled primary cases, conditional success among accepted burns and
diagnostic outcomes separate. Do not equate search/construction failure with
physical impossibility.

Synthetic airborne starts are labelled local tasks. A reset scenario created
from an old handoff snapshot does not establish original-mission integration.
The current upright-coast activation programs also do not reproduce the old
attitude-preparing coast. Do not carry their timing-grid conclusions into this
backend or launch another activation study before resolving that difference.

## 7. Acceptance beyond an eventual landing

For each accepted primary burn, independently assess:

- Native target touchdown, mission success, execution validity, integrity,
  complete original-source command replay and component reconstruction.
- One uninterrupted powered interval from burn activation to native contact,
  allowing variable throttle and retaining the existing residual window.
- No downward-to-upward velocity reversal for the declared descending core;
  no inserted hover/settling phase, clock reset, mid-burn reference replacement
  or undeclared command owner.
- Incoming contact speeds, attitude/rate, foot/hull geometry, pad margins,
  contact-time residual, fuel used/remaining and original time remaining.
- Requested versus emitted versus applied thrust, reference tracking residuals,
  native command count and original versus construction/proof work.

Report exact violations and raw magnitudes. Freeze any numerical tolerance
before release; do not relax native landing/contact rules. A shape violation
remains a failure of the declared maneuver even if the craft eventually lands.
Existing planner/body guard protections remain in their own execution path.

Fuel and time are descriptive efficiency measures initially. Preserving an
existing reference is not an optimality claim. Comparisons require the same
physical state, original budget and available request/timing information. The
old standalone target-only controller is not a fair timed-goal comparator merely
because both plots end at the same pad.

## 8. Implementation checkpoints and release gates

| Checkpoint | Concrete output | Required acceptance before proceeding |
| --- | --- | --- |
| A — Contract and inventory | Encode confirmed immediate ignition, planner-owned coast and explicit reference-end time; finalize the timed-request version and bind the 44-case source, all 38 burns, exact entry-recovery method and unmodified source/capture hashes | No ambiguous backend/default alias, stale-observation admission or guessed duration |
| B — Shared burn extraction | Shared coefficient/actuation code, thin accepted-planner adapters and small deterministic numerical tests | Exact reference, command, state/fuel/clock and original accepted-role preservation; no change to ordinary PDG or planner selection |
| C — Timed maneuver adapter | Versioned request/admission, explicit backend factory, native composition and reconstruction support | Boundary/lifecycle tests and full original-prefix-to-burn parity on the 38 accepted burns; NoPlan and exhaustion remain honest ordinary dispositions |
| D — Flat task and cost pilot | Input-only recipe/audit, new immutable populations, at most 12 predeclared development pilot originals with full proof/resource receipts | Frozen numeric entry/timing contract and coverage; measured cost fits prospective limits; no fresh-test access |
| E — Structured characterization | At most 256 primary development originals and 96 separate diagnostic originals, with transforms and exact counts declared before launch | Complete inventory, shapes/proofs/cost reviewed; characterize uncovered cells and valid failures without tuning them away |
| F — Fresh confirmation and report | One eligible fixed-size confirmation, complete report and closeout | Final source, backend, recipe, criterion and limits frozen before release; no extra sampling after results |

The counts in D/E are proposed ceilings, not an allocation or a partially filled
success denominator. Freeze their exact schedule before execution. F is proposed
as 600 independent primary originals with the existing style of **99% landing
reliability and a one-sided 95% exact-binomial lower confidence bound**, plus
zero violations of the declared burn-shape/clock/ownership contract. The
confidence claim applies only to the frozen flat timed-task mixture; no
per-stratum 99% or arbitrary-state claim follows. Decide the final criterion
with the actual declared task before release, not from pilot landing results.

Do not release F while extraction, adapter proof, task integrity or required
behavior is unresolved. A negative or incomplete development result closes
with evidence and a recommendation; it does not automatically trigger more
controller epochs, another sampler or a fresh-test restart.

## 9. Validation and review

The review gate has three separate parts:

1. **Preservation:** compare the old and extracted reference builders and command
   paths using frozen vectors and original saved prefixes. Compare against the
   original independently bound command ledger, not only two callers of newly
   shared code. Preserve historical serialization and all 44 original roles.
2. **Contract:** check current-state admission, odd/stale/future ticks, exact
   command-clock handoff versus planner cadence, duplicate
   and changed IDs, Busy/Unsupported/NoPlan, no warm state from advice, fixed
   reference time, partial last intervals, reset and native closure. Fault tests
   must detect wrong backend, coefficients, duration, command bits and sources.
3. **Assessment:** audit all scheduled slots and proposal dispositions, independent
   original-source replay, raw incoming contact and burn-shape evidence. Review
   complete mission-tree/detail navigation on desktop and phone.

For implementation, run the maintained gate:

```sh
rtk proxy node scripts/check-planner-development.mjs
```

The retained 44-case gate is an explicit implementation-preservation check
against the maintained October 7 numerical baseline:

```sh
rtk proxy node scripts/check-planner-development.mjs \
  --parity-capture outputs/eval/planner_v2_lab_suite/capture-early-exit-20261007-native
```

That opt-in check and any prefix regeneration/replay have an execution cost;
include it in the implementation ledger and keep it distinct from fresh
originals. Ordinary software checks must not require this local capture.
The separately identified October 5 historical-prefix adapter comparisons bind
their original commands and recovered construction; they are an additional
preservation target, not the numerical gate's source.

For October 5, the saved flight ledger does not serialize the selected terminal
duration. The regression driver may recover it by enumerating the bounded even
durations within the original deadline and requiring exactly one bit-identical
first command from the live recovered entry. This is a pure coefficient/inverse
query, not a native forecast, new controller duration selector or reproduction
of the old planner's selection policy. Do not use consumed burn length or final
outcome to select a duration. After unique recovery, require every original
command bit, sample, incoming contact and final state to match, together with
the independent native proof tuple. No exact match or ambiguity stops that
case's preservation gate with its original evidence intact.

The planning pass itself uses read-only source/history/capture inspection and
documentation checks. It performs no native flight, replay or report refresh.

## 10. Accounting, reporting and stopping boundaries

Reuse the existing immutable-input, bounded-attempt, recovery and evidence
machinery. Add explicit backend/task selection; do not fork a parallel harness.
Existing calls and saved V1 reconstructions retain their standalone semantics.
New source ledgers must include the shared Poly Landing files and any required
contract/serialization implementation; an old executable with new sources is
not a valid reconstruction epoch.

Count original flights, source-prefix recovery, candidate/constructor work,
native forecast steps if any, proof/reconstruction passes, test work, report
reconstruction and external repeats separately. Pure reference evaluation is
not a physics replay. Conversely, any native trajectory query must not be
hidden under the word construction. Measure source regeneration separately
from a cheap read of a saved row.

After the bounded pilot, prospectively bind wall time, original/proposal counts,
construction limits, all proof passes, storage, report size and worker count.
No measured phase starts without those limits and create-only output paths.
Do not inherit the old goal loop's authority or assume that an old budget is
valid for this backend. Stop on budget exhaustion, source/config drift, missing
proof, inconsistent geometry or reconstruction disagreement; preserve every
completed and interrupted slot. A technical invalidation requires its own
record and a prospective replacement allowance, never a quiet retry.

Use the established common tree and rich detail templates. The new report leads
with task/backend/request identity, then population → entry/timing axes →
mission → scheduled attempts. Include every evaluated flight and explicit
rejected/unsupported/unscheduled status. Keep core, diagnostics, historical
parity and any later activation comparisons separate. Expose fixed reference
time, ignition/contact, engine-off intervals, re-climb and applied thrust in
the maintained detail views rather than introducing a flat replacement browser.

The subsequent goal-loop request authorizes implementation and evaluation
through these gates under the [new execution declaration](poly_landing_execution.md).
Report publication and push remain separate actions. RL training, obstacle
navigation, autonomous duration selection, other hardware, disturbance recovery
and default promotion are outside this first pass.

## 11. Decisions to carry into implementation

Confirmed timing ownership: the planner owns coasting and supplies the absolute
planned touchdown/reference-end time; accepted activation starts the powered
burn immediately. Poly Landing executes the selected finite reference without
choosing a new duration.

The proposed first evaluation scope remains prepared descending flat entries,
the accepted vehicle, one finite reference, and exact existing realization
before new characterization. Moving duration selection or attitude preparation
inside the controller would require a separate capability design and identity.

Checkpoint A must finalize the timed-request field/version and existing contact
allowance, recover exact accepted entries/durations, and write the quantitative
flat recipe. These are explicit deliverables with source evidence and tests;
they are not permission to guess entry states, derive an oracle from final-test
outcomes, or call a new duration heuristic an unchanged interface.
