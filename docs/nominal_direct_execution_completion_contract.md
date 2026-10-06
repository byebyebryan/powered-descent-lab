# Nominal direct operational completion and saved-command coverage V1

> Historical record: superseded executable commands are retired. Use the
> [current documentation](../README.md#docs) for current tooling. Links to retired source
> below point to verified pre-retirement Git revisions, not live modules.

## Status and decision

Reviewed contract, based on `abc8bca7597c0f812cfac716527a20eb23140ba7`
on 2026-09-28. The separately authorized
[implementation protocol](nominal_direct_operational_execution_protocol.md)
owns the current execution pass. The design itself introduced no flight
measurement, accepted program, continuation policy or default change.

Select **strict saved-command coverage** for the first opt-in operational
executor. Stop on actual authoritative contact, or stop the laboratory run at
the first finite execution boundary. Do not supply an idle command, repeat the
last command at a missing update, append commands, or extend the planned end.
Keep the existing exact nominal executor and witness verifier strict.

The [contact-phase results](nominal_direct_contact_phase_results.md) motivate
this distinction: saved programs have 168 safe contacts and 48 coverage-limited
observations per run, whereas the separate diagnostic hold has 216 safe
contacts. Those exposed, vertical-only counterfactuals do not authorize a hold
policy or certify an operating tolerance. The new contract resolves completion
and accounting; it does not recover the 48 later-contact observations.

Stopping a simulation while airborne is a computation/authority boundary, not
a physical emergency maneuver or a claim that a real vehicle has been saved.

## Authority and frozen invariants

The [nominal integration protocol](nominal_direct_flight_integration_protocol.md),
[body-aware policy](waypoint_direct_body_aware_terminal_protocol.md),
[architecture](architecture.md) and [guidance ownership](guidance.md) retain
their existing authority. In particular:

- `FlightProgramV1` keeps structural/context/clock validation;
  `run_flight_program` keeps exact playback, update and expected-step
  postconditions; `execute_nominal_direct_flight_program` keeps independent
  witness acceptance and command/contact/fuel/action-replay parity. No old
  `passed` field acquires a weaker meaning. Structural program validation is
  not physical acceptance.
- Generator inputs, complete finite ledger and selected-witness verification
  remain unchanged. Stored programs are comparison references, not generator seeds.
- Core dynamics, body geometry, contact thresholds, ordinary mission handling,
  120 Hz physics and globally held 60 Hz commands are unchanged.
- Built-in controllers, their registry, ordinary unbounded execution/replay,
  `pd_plan::plan()`, existing schemas and historical artifact bytes retain
  their behavior. Sharing runner internals requires exact legacy parity.
- New execution still starts from the precisely bound mission/source-pad rest
  context. It does not admit arbitrary initial states, resume at terminal
  entry, inject a disturbance or perform a waypoint handoff.

The operational entry point independently admits an accepted nominal witness
and its exact converted program before motion, just as the current adapter
does. Rehashing modified commands, context or saved verification flags cannot
bypass admission. Invalid, Unsupported and finite Unknown requests run no
flight; an operational execution result is not a new planning decision.

## Keep three result dimensions separate

1. **Admission:** the supplied nominal witness/program is independently verified
   and exactly bound to the request. This is a prerequisite, not proof of every
   possible execution after a disturbance.
2. **Observed execution:** retain the actual core contact/termination, command
   prefix, physical state and pointwise validity evidence. Derive the operational
   result from those facts, never from the expected contact marker alone.
3. **Nominal comparison:** `Match`, `Deviation` or `NotComparable`, with separate
   command/clock, first-contact, tick, fuel and deterministic replay checks.
   `Match` requires the complete existing nominal checks, not merely equal ticks.

`CompletedSafeTarget` requires authoritative stable target contact, a valid
authorized command prefix, valid numeric/clock/domain/actuator and pointwise
clearance evidence, and bounded action-replay parity. Exact nominal timing is
not an additional operational success predicate. An earlier contact may leave
saved commands unused; that is an operationally valid prefix but a nominal
deviation. A later contact can count only if it still lies inside actual saved
coverage and the hard deadline. No commands are authorized beyond that coverage.

| Observed facts | Operational result | Nominal comparison |
| --- | --- | --- |
| Valid safe target contact and all exact nominal checks | `CompletedSafeTarget` | `Match` |
| Valid safe target contact at a changed tick or with unused commands | `CompletedSafeTarget` | `Deviation` |
| Authoritative crash / stable off-target contact with trustworthy evidence | `UnsafeContact` / `OffTargetContact` | `Deviation` |
| Still airborne at the next uncovered update | `CoverageExhausted` | `Deviation` |
| Still airborne at planned end / ordinary scenario timeout | `HardDeadlineReached` / `ScenarioHorizonReached` | `Deviation` |
| A trustworthy body-domain, clearance or actuator-budget violation | `SafetyRejected` | `Deviation` |
| Nonfinite state, binding/clock inconsistency or replay/audit disagreement | `ExecutionInvalid` | `NotComparable` |

Preflight invalidity has no executed-prefix result. Runtime invalidity preserves
its valid prefix and earliest divergent evidence. If a safety violation and
safe contact coincide, retain the core contact as an observation but do not
report `CompletedSafeTarget`. Raw core physical/mission outcomes are not
rewritten to agree with an evaluator rejection.

Nominal source/entry/reference equality remains part of exact witness proof.
It must not be silently relaxed there or confused with runtime finite-state,
body-clearance and actual-contact validity. These checks remain distinct.

With unchanged bound inputs and this deterministic plant, an admitted nominal
run is expected to match exactly. The changed-contact branches are explicit
executor semantics exercised by neutral fixtures, not a claim that V1 admits
off-nominal mission inputs. Unrecorded state mutation cannot earn completion:
independent bounded replay must reproduce the actual prefix or report invalidity.

## Integer clock and finite bounds

For a validated program, let:

```text
I = global control_interval_steps                 (currently 2)
N = number of saved updates
C = expected_contact_physics_step
K = N * I                                        (checked multiplication)
H = planned_end_physics_step
L = min(K, H)
```

Updates occupy pre-step ticks `0, I, ..., K-I`. The final command is ordinarily
held over transitions through step `K`; it does not authorize a callback at
`K` or a transition beyond `K`. The current structural count is
`N = ceil(C/I)`, so `K` can exceed `C` when contact is off the control clock.
For example, `C=3341` yields final update 3340 and `K=3342`; a contact at 3342
can be within saved coverage without an extension. For flat-600, `C=K=3342`
and `H=3346`: contact at 3344 needs a new command and is not authorized by V1.

`C` is a nominal marker, `K` is a command-authority boundary, and `H` is an
independent hard deadline. Preserve all three. Validate overflow, global update
alignment and `C <= H <= ceil(max_time_s * physics_hz)` before simulation.
An operational wrapper must recompute bounds from the complete admitted
program; neither a truncated action log nor an artifact's claimed stop tick
may reduce them. No second four-tick reserve is added.

### Event order

At initialization and after every authorized ordinary transition:

1. Check trustworthy numeric/clock state and retain any first failing invariant.
   Runtime safety checks are sticky; a later good contact cannot erase them.
2. Retain authoritative contact and its incoming state before ordinary stable
   touchdown zeroes velocity/angular rate. Preserve ordinary contact, progress
   and horizon precedence: contact at a reached boundary is not discarded.
3. If core is terminal, stop without another callback. Resolve the final result
   with the validity evidence; preserve actual core events and outcomes.
4. If still running, stop at `H`, then at `K`, before requesting another command
   or advancing physics. When `H=K`, report `HardDeadlineReached` and record both
   reached-boundary flags. An ordinary horizon termination takes precedence over
   these driver stops; its raw manifest retains the real timeout outcome.
5. Only if still authorized, issue the saved command on a globally due tick,
   or keep the already-issued command between due ticks, then advance exactly
   one ordinary transition. Check command/actuator validity before advancing.

This order allows contact on step `L`, including an odd hard deadline. It never
allows step `L+1`. No uncovered controller invocation, fabricated update/action,
idle failure frame, extra sample-producing step or phase-local clock reset is
permitted. A command inconsistency discovered at an otherwise covered callback
is invalid execution, not ordinary coverage exhaustion.

The guard has three explicit stages: initial numeric/domain/source-support
validation; pre-transition validation of the command selected at the due tick
(or genuinely held between ticks), including its next-step fuel budget; and
post-transition validation. Pre-transition checks run on every physics step,
not just control updates. Post-transition airborne clearance runs only for
`ContactClassification::None`. Contact uses the captured incoming state and
unchanged first-contact predicates, not an airborne nonpenetration test or
normalized zero velocity. Initial support is audited separately without
incrementing the archived post-step scan counts.

Core does **not** terminate solely because fuel reaches zero. Keep unpowered
zero-fuel coast legal; do not invent a core fuel-exhaustion end reason. Record
zero fuel separately. Reuse the existing actuator-budget rules: insufficient
fuel for an authorized powered transition or a powered request with no fuel is
`SafetyRejected`, not evidence of a physical crash. A transition consuming its
exact remaining fuel may still end in valid contact. Do not change the core's
fuel consumption or thrust equations to enforce the execution guard.

## Coverage alternatives and disposition

| Choice | Disposition | Reason |
| --- | --- | --- |
| Strict saved coverage | Selected for V1 | Adds honest finite outcomes without new command authority or physical policy. |
| Commands generated and verified through planned end | Deferred | Current witnesses end at contact; commands beyond it need a separately specified generator, verification and branch-validation contract. |
| Bounded repeat of final command | Deferred | Diagnostic success is encouraging but not an admitted program, validated recovery policy or held-out operating envelope. |
| Silent idle, indefinite hold, larger cap or threshold relaxation | Rejected | Invents authority or changes physical policy to hide an incomplete execution. |

There is no continuation switch in V1. Any later extension requires a distinct
versioned, identity-bound policy, explicit commands/hold semantics and finite
caps; newly sealed acceptance evidence; unchanged nominal controls; and its own
authorization. Time reserve alone is never that authorization. Do not extend
this design pass into proving such a policy.

## Additive runner, API and artifact boundary

The implementation must use the ordinary plant/mission transition, not promote
the evaluator study's paired replay loop into a second runtime engine.

- **Core bounded runner:** add explicit command-coverage and hard-step bounds,
  a fallible command decision and an opt-in per-step validity guard. Guard/bound
  stops return typed partial evidence, not only `SimulationError`. The existing
  unbounded APIs keep their signatures and behavior through a shared loop.
- **Authoritative contact capture:** add an opt-in step report/snapshot seam
  sharing the same single `step_physics_and_classify_contact` call and ordinary
  mission handling. Capture incoming classification, tick, pose, velocity,
  attitude/rate, fuel and held command before normalization. Do not integrate a
  second time or copy contact predicates into the runner. Default traces/events
  must remain exact; capture must not force additional ordinary samples.
- **Neutral bounded envelope:** bind complete program identity, execution policy
  identity `strict_saved_coverage_v1`, derived `C/K/H`, typed stop cause and
  boundary flags, actual final-state snapshot, optional incoming contact and
  the ordinary `RunArtifacts` prefix. New persisted DTOs use explicit versioning
  and reject unknown fields; their exact Rust names are implementation details.
- **Control opt-in playback:** a separate operational function returns the
  bounded envelope and controller records. It validates and consumes the saved
  command prefix; it does not call the old exact executor and suppress its
  errors, weaken its postconditions or route failure through `Command::idle()`.
- **Evaluator admission/audit:** a separate adapter owns selected-witness
  admission, unchanged phase-aware pointwise clearance/strict body-domain and
  actuator checks, actual-run audit, nominal comparison, identity, CLI and
  create-only evidence. The validity guard is policy-supplied, so core/control
  acquire no evaluator dependency. Generic neutral playback is not admission.

Bind and validate the validity-guard policy identity and all behavior-bearing
configuration as well as the coverage policy. The admitted adapter uses only
the fixed reviewed guard; an arbitrary caller-supplied permissive guard cannot
earn `CompletedSafeTarget`. Give the new playback mode its own controller
identity. Compare physical/actions/contact parity with the old mode explicitly;
do not demand identical differently versioned wrapper/controller metadata or
describe such bundles as byte-identical old artifacts.

On an airborne driver stop, raw core state remains `Flying / InProgress /
Running`; retain its honest manifest and extrema. The new envelope says that
execution stopped and why. Do not fabricate `Crash`, `MaxTimeReached` or
`MissionEnded` events, change existing `EndReason` variants/schema versions, or
publish that prefix as an ordinary completed-run bundle. Always retain the final
snapshot even when sample decimation omits the stopping tick. Reports must make
the envelope's incomplete execution and evidence scope explicit.

For nonfinite runtime failure, retain the last valid serializable prefix and
snapshot plus the first bad boundary/field and typed NaN/infinity diagnostic.
Do not replace a bad value with zero, publish a fabricated finite final state,
or lose all evidence because serialization of the failed state is impossible.

Reuse existing phase-aware clearance rules, including source-departure support
and the descending target-pad corridor; a blanket positive-clearance requirement
at source rest would reopen the cutaway bug. Airborne clearance and allowed
first-contact penetration remain different predicates. Pointwise scans are not
swept-path certificates. A guard failure stops further execution at its first
detected boundary and preserves that boundary, not a retrospectively safe label.

### Bounded action replay

Add a separate bounded replay entry point using the same complete program,
bound policy and independently recomputed guard decisions. Existing
`replay_simulation` cannot replay an airborne truncated prefix: it demands the
next action until core termination and rejects unused actions afterward.

Replay must reproduce actions, events, samples, final state, incoming contact,
stop cause and audit evidence. It must not trust a saved outcome or stop tick,
derive coverage from the shorter executed log, or discard extra/truncated
actions. Early real contact legitimately uses only the matching saved prefix.
An empty action log is legal only when the independently recomputed initial
guard/bound stops before the first update; otherwise it is invalid truncation.
Observational compute/per-update timings stay outside deterministic identity.
An externally altered state needs an explicit intervention history to be
replayable; V1's operational input has no such intervention mechanism.

Those full-reproduction requirements govern trustworthy completed/bounded
executions, including deterministic safety rejections. Invalid executions
retain their last valid prefix, attempted/failing boundary and replay diagnostic
even when the fault cannot be reproduced. Such a result is `ExecutionInvalid` /
`NotComparable`, never verified completion; evidence writing must not depend on
successful replay of an unrecorded fault.

Use a separate opt-in CLI mode, proposed as `nominal-direct-operational-flight`,
with read-only preflight and create-only output modes. Keep existing nominal
commands, output schemas and report bundles unchanged. A successful evidence
capture or integration gate is not an alias for flight completion: count every
operational outcome and nominal comparison separately, including incomplete and
failed flights. A partial-run report must consume the bounded envelope.

### Current source seams checked by the design

- [Program validation](../pd-core/src/flight_program.rs):
  `validate_updates` fixes the count at `ceil(C/I)` and global pre-step ticks.
- [Ordinary plant/runner](../pd-core/src/sim.rs): `SimulationState::step`
  classifies contact before progress/horizon; `run_simulation` issues the next
  callback only after a nonterminal transition. Neither its command callback
  nor `replay_simulation` currently supports a truthful driver stop.
- [Core contact handling](../pd-core/src/eval.rs): `apply_landing_goal`
  normalizes stable-contact velocity/rate; `apply_max_time` is a real mission
  timeout, not a substitute for driver exhaustion.
- [Controller runner](../pd-control/src/lib.rs): `Controller::update` returns a
  frame, not a stop decision. [Strict playback](https://github.com/byebyebryan/powered-descent-lab/blob/d7b7075f6c6a3e3ee3603fce3bb88114be7e648c/pd-control/src/flight_program.rs)
  records failure and returns idle at a missing callback, then reports the error
  after `run_controller`; its exact postconditions intentionally remain strict.
- [Nominal adapter](https://github.com/byebyebryan/powered-descent-lab/blob/d5517c4794046864712695718ac71dd4b93bf55b/pd-eval/src/nominal_direct_flight.rs):
  `execute_nominal_direct_flight_program` independently verifies the witness and
  exact conversion, then requires command/contact/fuel/action-replay parity.

## Implementation sequence and ownership

One writer at a time; the primary reviews shared runner and result semantics.
Implementation is a later, separately authorized pass, not part of this design.

| Slice | Owned files/responsibility | Acceptance before proceeding |
| --- | --- | --- |
| 0: Seal validation inputs | New research manifest and implementation protocol | Four resolved physical inputs, policy, population order and hashes sealed before runtime edits/evaluation; no outcome selection. |
| 1: Neutral stop/capture/replay | `pd-core/src/sim.rs`, exports and a new neutral execution DTO module if useful | Boundary, truthful-prefix, incoming-contact, bounded-replay and unchanged legacy-run parity tests. Existing program and manifest schemas unchanged. |
| 2: Operational saved playback | `pd-control/src/flight_program.rs` and exports | Exact consumed prefixes, no uncovered callback/idle action, unchanged strict executor and built-in controller behavior. |
| 3: Opt-in admitted execution | New `pd-eval` operational adapter/tests, exports and opt-in CLI wiring | Full witness admission, actual-prefix audit, separate outcomes/nominal checks, retained failures and create-only bundles. |
| 4: Integrated gate and closure | Focused gate/protocol/results/status docs | All predeclared controls, fresh inputs and negative cases recorded; source/archive provenance, deterministic repeats and repository gates pass. |

No `pd-plan` dependency or default controller registration is added. Do not
optimize generator cost or refactor unrelated guidance in these slices.

## Validation contract for the next implementation pass

Artifact-free fixtures cover executor semantics, not physical robustness. Early
and late contacts can use structurally valid neutral fixtures with different
nominal markers; they are not accepted generator witnesses. The admitted
end-to-end adapter must reject altered context/program/witness combinations.

| Fixture or gate | Required observation |
| --- | --- |
| Exact nominal contact | All old nominal checks remain true and operational result is completed. |
| Safe contact earlier, including before an update | Stop at actual contact; unused saved suffix is allowed only operationally; no later callback. |
| Safe contact later but still at/before `min(K,H)` | Complete with nominal deviation; zero extra command authority. |
| No contact at `K < H` | Coverage exhausted at `K`; callback/action counts unchanged after final saved update; no step `K+1`. |
| Odd/even `H <= K`, and `H=K` tie | Deadline stop and consistent reached flags; held command cannot cause an extra transition. |
| Contact at `K`, at `H`, or at scenario horizon | Preserve core contact precedence, incoming state and unchanged mission events. |
| Genuine unsafe or off-target contact | Typed physical failure; cannot be relabeled as an expected-tick mismatch alone. |
| Domain/clearance failure before or coincident with safe contact | Sticky safety rejection; preserve raw core observation; never completed-safe. |
| Zero-fuel idle coast / underfunded powered command / exact final burn | Separate fuel observation from actuator rejection; valid exact-burn contact stays possible. |
| Missing, extra, reordered, off-clock, nonfinite or self-rehashed commands | Admission rejection, or retained runtime-invalid evidence; no silent clamping/fallback acceptance. |
| Partial replay with forged stop, truncated or extra actions | Recompute bounds/guards from full contract; reject disagreement rather than shorten replay. |
| Initial guard stop and off-sample stop | Truthful zero/partial-step artifacts, final snapshot and no fabricated mission events. |

Use all 24 exposed nominal controls for exact selected-witness, command,
contact-tick/fuel and legacy ordinary replay parity. They remain regression
controls, not held-out inputs. Existing contact-phase archives stay immutable;
do not recapture the 216-row matrix to approve an operational hold.

For the next pass, seal four new uncut, obstacle-free missions in this order:
685 m flat, 845 m flat, 845 m uphill +75 m, 845 m downhill -75 m. Reuse the
supported vehicle/policies, 36 m flat shelves, 160 m terrain-domain padding,
continuous shelf-to-shelf terrain, source-pad upright rest, 9.81 m/s2 gravity,
90 s horizon and unchanged 120/60 Hz clock. Resolve and hash full physical
inputs before implementation or generation; the numbers here are a prescribed
population, not an already sealed artifact or a claim of unseen success.

Generate these missions from inputs, retain every decision and run only admitted
Direct programs. Do not replace Unknown/Unsupported cases or tune to rescue
them. Require exact nominal and operational parity for every admitted case and
deterministic repeats. Fresh terrain coverage targets all four cases as Direct,
safe and nominal Match. Any Unknown remains a recorded generator-coverage gap,
not an executor defect or infeasibility; report the unmet fresh coverage gate
rather than substitute a new population or tune the generator. The
fresh gate validates this contract on new nominal missions, not disturbances,
continuation, an operating envelope or general planner coverage.

Run focused tests, artifact-free workspace tests, strict all-target workspace
Clippy, formatting and diff checks after runtime changes. Run the fixed 24-case
and four-fresh-case integration gates separately with create-only outputs;
retain failures and compare deterministic repeats excluding only observational
timings. Bind current source/protocol/inputs and verify historical archive bytes
before/after. Do not repeat full suites without a changed source or unresolved
verification risk.

## Design closure and stop rule

This pass closes when the policy, outcome axes, integer bounds, event order,
truthful partial artifacts, bounded replay, ownership and validation gate are
reviewed against current source. Its validation is documentation/source-seam
review and formatting/link/diff checks, not newly executed physics evidence.

The next authorized implementation can follow the four runtime/integration
slices after the input seal. After that gate, stop and review whether direct-first
planner integration is justified. Genuine airborne waypoint composition remains
separate: this rest-pad-only generator cannot yet certify arbitrary incoming
waypoint states. Finite planning Unknown or execution coverage exhaustion still
does not establish that an obstacle requires a waypoint.

No new obstacle/perturbation sweep, feedback recovery, source fitting, terminal
optimizer, threshold/cadence change, swept-path proof, continuation approval,
production/default promotion, roadmap expansion, commit, push or deployment is
included. There is no unresolved coverage-policy choice in V1: **stop at strict
saved coverage; preserve exact nominal proof; report incompleteness honestly**.
