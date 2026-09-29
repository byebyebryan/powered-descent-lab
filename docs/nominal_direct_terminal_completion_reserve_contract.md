# One-update nominal direct terminal-completion reserve V1

## Status, question and authority

Design-only checkpoint at `b6a235a73501a5ea20e168af44e66c9e685fcb16`,
2026-09-28. The [implementation protocol](nominal_direct_terminal_completion_reserve_protocol.md)
prescribes a separately authorized future pass. This document introduces no
runtime capability, new flight measurement, accepted continuation witness,
operating tolerance, default change or deployment.

The [strict saved-coverage executor](nominal_direct_operational_execution_results.md)
already closes honest finite execution and nominal completion on 28 missions.
The [contact-phase diagnostic](nominal_direct_contact_phase_results.md) separately
retains 168 safe contacts and 48 coverage-limited observations in each 216-row
run. Its diagnostic final-command hold lands those 48 within one additional
global update and the original deadline. Those exposed vertical-only results
motivate one bounded policy; they do not themselves admit it.

Select one new opt-in policy, `one_update_terminal_completion_v1`: authorize
at most one additional global control update using the exact final saved
command. It is command-coverage completion, not feedback recovery or another
trajectory search. Existing [completion authority](nominal_direct_execution_completion_contract.md),
[architecture](architecture.md) and [guidance ownership](guidance.md) remain
unchanged outside this separately versioned mode.

## Frozen invariants and supported scope

- Keep `FlightProgramV1`, its command-count/context/clock validators, strict
  nominal playback, strict saved-coverage playback/replay and independent
  selected-witness verification unchanged. No appended update is inserted into
  a V1 program and no old `passed` field acquires a weaker meaning.
- Generate from physical inputs alone under the unchanged generation and
  body-aware terminal policies. Preserve full ledgers, skips and accepted-only
  selection. Archived programs are comparison references, never generator seeds.
- Keep ordinary dynamics, mission/contact handling, vehicle geometry, fuel/
  actuator rules, contact thresholds, 120 Hz physics and global held 60 Hz
  control unchanged. Preserve the existing pointwise body/terrain-domain and
  source/descending-target-corridor safety rules; do not claim swept safety.
- Full-mission execution retains the current supported source-pad-rest context.
  There is no arbitrary initial-state, waypoint-handoff or in-flight replanning
  admission. Invalid, Unsupported and finite Unknown decisions execute no flight.
- Core remains policy-neutral; control does not acquire an evaluator dependency.
  No built-in registry, controller trait, planner/default selection, old schema,
  historical protocol/archive or roadmap change is included.

## Complete authorization and integer bounds

For a complete validated nominal program, derive before motion:

```text
I = global control interval in physics steps                  (2)
N = complete saved update count
C = nominal expected contact tick
K = checked N * I                                             (saved coverage)
H = original planned-end tick
J = checked K + I
A = J if K < H, otherwise K                                   (authorized coverage)
R = min(A, H) = min(K + I, H)                                 (execution limit)
```

The original `C/K/H` meanings survive. Core bounded limits use `A` and `H`,
not a shorter action log, claimed contact tick or persisted stop. Scenario
horizon remains an independent ordinary core boundary. Validate rates, finite
canonical commands, all overflow and the original program bindings first.

A new versioned completion wrapper binds the exact nominal program identity,
nominal witness and fixed policy. If `K < H`, its one explicitly derived reserve
update is at pre-step tick `K`, global ordinal `N`, with the final saved command's
exact throttle/attitude bits. Its identity includes the policy, update and
derived bounds. Playback/replay checks ordinal N, absolute tick K, canonical
time K/physics_hz and throttle/attitude bits independently; numeric equality
that conflates +0/-0 is not exact command parity. If `K >= H`, no reserve update
exists or can be invoked; ordinary nominal contact/deadline semantics still
apply. The wrapper is not a
modified `FlightProgramV1`, and structural authorization alone is not safety
proof for every state that might reach K.

Persist C/K/I/A/H/R separately in the completion/lane envelope. Core's existing
`coverage_reached` means the supplied authorized limit A, not original saved
coverage K. Derive flags from the retained final valid absolute tick F:

```text
saved_coverage_reached      = F >= K
authorized_coverage_reached = F >= A
hard_end_reached            = F >= H
execution_limit_reached     = F >= R
```

The authorized/hard flags must agree with core evidence even when contact or a
guard failure wins the stop cause. Contact at K can have saved=true,
authorized=false and zero reserve actions. Rejection after the transition to K
still has saved=true; a nonfinite transition rolled back to K-1 does not, though
its distinct first-bad-boundary evidence may name K. At odd H between K and A,
saved/hard are true and authorized is false. Do not infer these flags from the
stop enum or number of recorded actions.

Record `reserve_update_issued_count` (0 or 1 accepted action at ordinal N) and
`reserve_physics_steps_advanced` (max(0,F-K), at most min(I,max(0,H-K))) separately. An
accepted callback at K followed by a pre-transition fuel rejection may issue one
action but advance zero reserve steps; it cannot earn completed-safe. In the
strict lane, A is K and no reserve action exists; the same boundary fields retain
their explicit saved-versus-authorized meanings.

Reserve activation requires all of:

1. The full nominal witness/program was independently admitted before motion.
2. The original saved prefix was consumed on its exact global command clock,
   without an earlier sticky safety/numeric/clock failure.
3. The ordinary state remains airborne/running at K, after processing any
   contact, mission or horizon event at that boundary.
4. K is a globally due update before H, the final saved phase is
   `terminal_bridge`, and the fixed policy authorizes the derived reserve.
5. The new command passes the same pre-transition actuator/fuel checks before
   each subsequent physics transition.

There is at most one callback at K, never another at K+I. A deadline at K+1
clips the interval after one physics step. No second update, new four-tick
reserve, indefinite hold, automatic idle, feedback correction, changed cadence,
reference recomputation or optimized tail is authorized. The reserve's display
phase/provenance is separate; its geometry checks use the unchanged terminal
phase rules rather than inventing a looser clearance phase.

### Boundary precedence and outcomes

Use the same single ordinary transition and incoming-contact capture seam.
Numeric evidence and sticky guard failure retain priority for validity; ordinary
contact/progress/horizon handling precedes driver stops or another callback.
Contact on K, R or H is retained. A valid contact at K consumes no reserve.
When still running, H wins an A=H tie and both reached flags are recorded.
No transition at R+1 or callback at a reached limit occurs.

Keep nominal admission, observed operational completion and nominal comparison
separate. `CompletedSafeTarget` requires authoritative safe target contact,
valid authorized actions, fixed guard evidence and independent replay parity.
Unsafe/off-target contact and laboratory stopping retain their actual raw core
outcomes. An airborne coverage/deadline stop is not a physical emergency maneuver.

| Observation | Operational interpretation | Nominal comparison |
| --- | --- | --- |
| All unchanged exact nominal checks pass | Completed-safe; reserve unused | Match |
| Safe earlier/later contact with trustworthy authorized evidence | Completed-safe | Deviation |
| Reserve used but still airborne at A before H | Coverage exhausted in completion-reserve scope | Deviation |
| Still airborne at H or ordinary horizon | Hard deadline / actual scenario horizon | Deviation |
| Unsafe/off-target contact or sticky safety rejection | Retain raw contact and rejection separately; no completed-safe | Deviation |
| Binding, clock, numeric or replay disagreement | Execution invalid; retain finite prefix and first bad boundary | NotComparable |

Reaching C alone never earns completion. A reserve use can never earn nominal
Match. Equal contact ticks alone also do not establish Match; all original
command/clock, incoming contact, fuel, source/entry, clearance and replay checks
remain necessary. All new evidence DTOs are versioned and reject unknown fields.

## Two explicitly different execution kinds

### Source-rest nominal mission

The full-mission adapter independently admits the unchanged nominal witness
and exact program conversion, constructs the fixed completion guard and runs
the shared bounded engine with A/H. Regenerate full decisions and retain
preflight/admission failures without starting a prefix. Under unchanged inputs
and this deterministic plant, these flights should match nominally and consume
no reserve. This adapter does not silently inject or admit disturbances.

### Terminal-entry vertical-offset counterfactual

This research execution is separately typed as
`terminal_entry_vertical_offset_counterfactual_v1`, not a flight whose launch
physically produced the entry error. Its allowed intervention is only the
prescribed finite `position_m.y += delta_y_m` at actual terminal entry E,
after the reconstructed source/coast prefix and before the due update at E.

Before any nonzero offset, regenerate/admit and prove every available nominal
control's source/coast and complete zero-offset execution. The prefix must be
contact-free and end exactly at E on the global clock. Its full state must match
the independently verified witness entry, including held command and fuel.
If any baseline/lineage check fails, retain evidence and stop the experiment.

For each lane/offset, reconstruct the prefix rather than trust an archived or
persisted entry. A fixed evaluator capture guard clones the actual in-memory
`SimulationState` at E after its successful post-transition audit; a control
prefix pause stops before the due callback at E. Its captured state must agree
with the prefix's evidence snapshot and independently verified nominal entry.
This is a read-only capture through the existing guard seam, not a mutable
physics/state intervention hook or a new public core checkpoint API.

Clone that complete ephemeral state, change only position.y, and
record complete before/after snapshots, delta, tick, program/context/policy and
prefix identities. Time, fuel, velocity, attitude/rate, held command, mission
state, cumulative extrema and waypoint bookkeeping remain unchanged. Validate
the shifted entry itself without physics, in a distinct clearance/domain audit
with explicit phase `terminal_bridge`. Preserve the existing nominal post-step
phase accounting: `phase_at(E,...)` labels the prefix's E transition
`ballistic_coast`; do not change it to make the shifted-entry audit convenient.
Do not reset the global clock or rewrite the nominal witness/context to match.

The fixed matrix is ordered `0, -1, +1, -5, +5, -10, +10, -20, +20 mm`.
An intervention is input to independent counterfactual replay, not an unlogged
mutation or ordinary action. Replay freshly reconstructs the prefix and applies
that same declared intervention before consuming segment actions. Ordinary
action-only replay of an intervened trajectory is not expected to match and
must not be reported as its verification.

A nonzero intervention is always nominal Deviation even if contact tick happens
to equal C. Zero-offset Match requires recombined physical evidence and all
original nominal checks, not comparison of new wrapper metadata. A trustworthy
counterfactual completed-safe result describes that discrete branch only;
it does not widen full-mission admission or certify an operating tolerance.

## Narrow shared segment seam

Add a policy-neutral bounded-segment entry point backed by the same private
ordinary bounded loop as `run_simulation_bounded`, not by copying the diagnostic
paired replay loop or introducing another integrator. The existing source-start
APIs retain their signatures and deterministic artifacts. Exact Rust names are
implementation details; these observable semantics are required:

- The segment start is an ephemeral, non-serialized `SimulationState` produced
  by fresh evaluator prefix reconstruction/capture, not a deserialized snapshot
  or an alternative ScenarioSpec initial state. `SimulationStateSnapshotV1`
  remains evidence/query-only; do not use its `to_simulation_state()` method to
  create executable starts. This design only needs the declared, aligned
  terminal-entry boundary E. Validate finite state, exact absolute clock/tick
  consistency, running outcomes and command validity before callbacks. Derive
  global action ordinal E/I, never accept a log's local index as that ordinal.
  General arbitrary-tick resume, mutable intervention hooks and public
  disturbance scheduling are out of scope.
- Core can validate neutral structure, not witness lineage. The evaluator alone
  authorizes this start by fresh admitted-prefix reconstruction and the exact
  declared offset; a caller-provided snapshot/permissive guard is not proof.
- The first segment callback is the original saved update at E, global ordinal
  E/I. Prefix callbacks cover ticks 0 through E-I. All later callbacks/actions
  retain absolute ticks/times and global ordinals, including ordinal N at K.
- Keep the full snapshot's inherited cumulative state metrics; do not reset
  historical extrema to manufacture segment margins. A new segment envelope
  records start/end absolute ticks and `physics_steps_advanced = end - start`
  separately. Actions/events/samples are explicitly segment-local records with
  absolute clocks. No legacy manifest is presented as a whole source-rest run
  or falsely claimed to contain the omitted prefix.
- The prefix owns any periodic sample at E. The segment records its start
  snapshot independently and periodic samples only after E, using the original
  absolute sampling phase. Recombination has neither a duplicate seam sample nor
  an invented sample/event/time step. The intervention's before/after snapshots
  remain separately visible even when E is off the sample clock. The prefix's
  sample at E is pre-intervention lineage; nonzero-offset segments cannot be
  concatenated and labeled as an ordinary whole-flight trace. Any combined
  research display marks the explicit counterfactual seam, not a flown jump.
- The fixed segment guard validates the proved lineage and shifted initial pose
  without imposing a source-pad-support test at E. It carries freshly computed
  prefix evidence forward and reuses the same per-transition fuel, airborne
  clearance, incoming-contact/domain and actuator audits. Do not weaken the
  existing source-rest guard or accept caller-chosen validity policies.
- Initial/command/pre-transition/post-transition failures and zero-action stops
  preserve finite start/prefix evidence. Actual terminal/contact/horizon and
  driver-limit ordering matches full bounded execution, including odd H and
  boundary contact. No transition or command is invented to finish serialization.
  If no finite valid start exists, reject before motion and retain typed input
  rejection/available prefix evidence; do not fabricate a finite segment start
  or serialize NaN as a normal state.

For a contact-free prefix, use an explicit setup-time pause at E before its
callback: the prefix driver's coverage is E and its hard limit remains original
H. Record purpose `terminal_entry_capture`, not completion or exhausted original
saved coverage K. This laboratory pause is derived from the admitted program,
not from a truncated action log. The resumed segment has original H and the
lane's independently derived coverage limit; it does not inherit a pause cap.

Segment replay starts at the independently reconstructed start and global ordinal,
recomputes bounds from the full program/completion contract, and validates every
recorded action against the expected saved or single reserve update. Reject
truncated, extra, off-clock, altered, nonfinite or wrong-origin logs. Compare all
deterministic segment state/contact/actions/events/samples/stop/guard evidence.
Unused *planned* suffix after contact is legal; unused *recorded actions* are not.
Invalid evidence retains its first divergent boundary and cannot earn success.

This seam is a bounded state-start adapter and explicitly labeled segment
evidence, not a general checkpoint restore API. If it cannot share the existing
loop without broadening that scope, stop for a specific design decision.

## Implementation ownership and order

1. Primary freezes this contract, physical input seal, replay seam and protocol
   before runtime edits or generation. All behavior-bearing identities/bounds
   are specified; no state mutation is hidden in a guard.
2. A bounded core/control slice shares the ordinary loop, adds neutral segment
   artifacts/replay and separate completion-wrapper playback. Its artifact-free
   tests establish clock/sample/action/contact/bound/failure semantics and exact
   legacy parity. No evaluator policy or built-in registration enters core/control.
3. Primary implements independent admission/lineage, fixed full/segment guards,
   counterfactual reconstruction, result classification and create-only evidence.
   Initial source support and shifted entry validation remain distinct stages.
4. Add opt-in input-only preflight and integrated-gate CLI/reporting. Baseline
   nominal proof gates all perturbations. Preserve every case's decision, finite
   ledger, failure, lane and comparison result without replacement.
5. Perform overall source-bound review, artifact-free validation and the sealed
   repeated integration gates in the protocol. Keep commits/push/deployment
   separately authorized; a passed research matrix does not promote defaults.

## Acceptance and stop rule

The protocol owns exact controls, fresh manifest hash and negative fixtures.
The complete 32-case nominal gate precedes any counterfactual execution.
All 28 exposed nominal cases must preserve exact generation/program and physical
results; all four newly sealed nominal cases target Direct/completed-safe/Match.
The proposed reserve targets valid safe target contact across all nine discrete
offsets of every accepted case, before or at R, with independent replay parity.
The original 48 exposed late rows must use no more than one extra update and
retain their historical incoming-contact/tick/fuel semantics. Strict coverage
remains its separately observed lane, not a secretly extended baseline.

Fresh Unknown/Unsupported/Invalid is recorded as an unmet coverage/readiness
gate, not rescued by replacement or tuning. A baseline, lineage, guard, unsafe
contact or replay failure stops at its earliest demonstrated cause. Preserve all
completed and failed evidence. No second update, changed cap/threshold, source
fit, feedback lane, obstacle sweep or extra disturbance axis is added to rescue
the gate. Success closes only this explicitly authorized one-update policy
on the measured finite branches; continuous, combined-disturbance and full-flight
robustness, swept safety and physical deployment remain unproven.

The design pass closes when the minimal seam and validation are unambiguous,
the four inputs are input-only sealed, documentation/fixture gates pass and
runtime source/historical archives remain unchanged. Runtime implementation,
new accepted continuation evidence, planner/default promotion, arbitrary
waypoint-state composition, setup optimization, commit, push and deployment
are not authorized by this design closure.
