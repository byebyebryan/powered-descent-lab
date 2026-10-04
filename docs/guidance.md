# Guidance Architecture

This document is the stable boundary between terminal guidance, direct
transfer, waypoint guidance, and waypoint planning. It describes ownership and
compatibility rather than controller tuning. The V1 planner contract and
current V2 evaluator boundary are summarized in
[Waypoint Planning](waypoint_planning.md).

## Ownership

Terminal guidance owns braking, lateral cleanup, descent-rate control,
attitude, and touchdown after a terminal entry has been accepted. It may apply
local terrain-clearance constraints while selecting a terminal candidate, but
it does not plan a route or interpret waypoint profiles.

Direct transfer owns source-pad clearance, boost, coast, route-local corridor
constraints, and the decision to hand the craft to terminal guidance. It may
reacquire source clearance after a premature direct terminal entry. It does not
evaluate waypoint contracts.

Waypoint guidance owns the currently active preplanned leg, capture-window
lifecycle, handoff contract, continuation viability, and final-waypoint entry
into terminal guidance. It is terrain-blind: waypoint placement and arrival
envelopes must already encode a terrain-valid route.

The implemented waypoint-planning layer is upstream of guidance. It owns
terrain-valid waypoint placement, leg ordering, and arrival-envelope
construction. V1 plans once from static setup-time context and emits a direct
route or at most two preplanned waypoints. It does not select a controller, run
inside the controller update loop, or move terminal and waypoint contract logic
into route-label-specific branches.

A planner success is a bounded geometric and conservative-authority result, not
a promise that guidance will complete the mission. Existing handoff, ordered
sequence, terminal-recoverability, terrain-clearance, and landing evidence
remain the authority for flown behavior.

The V1 descriptions above cover `pd_plan::plan` and the controller guidance
path. The accepted native V2 workflow is separate: `pd-eval` constructs a
terrain-blind nominal flight, audits the fixed program against actual terrain,
applies local correction, and replans from the actual handoff state. This
evaluator-owned outer loop is not a `pd-control` controller or a per-tick
runtime planner. Its policy-3 batch and report contract are recorded in the
[activation results](waypoint_v2_eval_activation_results.md).

## Lifecycle Contract

The following lifecycle describes `pd-control` guidance over the resolved or
authored route; it is not the V2 evaluator's multi-segment flight loop.

- terminal guidance starts only after terminal spatial ownership and entry
  policy pass
- direct transfer progresses through source clearance, boost, coast, and
  terminal ownership
- waypoint guidance keeps the active leg until its window contract passes or
  the waypoint-plane deadline resolves it
- a recoverable final waypoint enters terminal guidance directly
- a final waypoint without recoverability evidence retains the transfer
  fallback instead of assuming terminal safety
- route-contract reports stop at contract completion; landing reports continue
  to physical touchdown

## Compatibility Surface

The following controller and guidance contracts are persisted or consumed
across crates and must remain stable during behavior-preserving refactors. The
native V2 evaluator keeps a separate batch schema rather than converting its
records to the controller `BatchReport`; see the
[V2 activation results](waypoint_v2_eval_activation_results.md).

- `ControllerSpec` JSON shape and built-in controller aliases
- canonical controller IDs
- terminal and transfer configuration field names and defaults
- controller phase strings
- telemetry metric keys and marker IDs
- batch schema `34` waypoint and terminal-recovery fields plus schema `36`'s
  optional planner provenance, diagnostics, and identity-neutral per-solve
  monotonic wall-time evidence
- deterministic mission outcomes, handoff evidence, and landing summaries

Internal Rust types and module paths are not compatibility surfaces. They may
be reorganized to make ownership explicit as long as the persisted contracts
above remain unchanged.

## Implementation Layout

The current `pd-control` layout follows those ownership boundaries:

- `controllers.rs` owns the controller registry plus the legacy baseline and
  staged controllers
- `guidance.rs` owns shared state-target acceleration and command allocation
- `terminal/mod.rs` owns the terminal update loop and guidance-plan lifecycle;
  `terminal/config.rs` preserves the public serialized configuration,
  `terminal/state.rs` owns internal command and entry DTOs,
  `terminal/planning.rs` owns deterministic candidate ordering and ballistic
  helpers, and `terminal/terrain.rs` isolates local candidate clearance
- `transfer/mod.rs` owns the direct-transfer and waypoint update loop;
  `transfer/config.rs` preserves the public serialized configuration,
  `transfer/state.rs` owns lifecycle state and internal DTOs, and
  `transfer/math.rs` owns shared ballistic and command-conversion helpers
- `transfer/telemetry.rs` owns transfer and waypoint metric emission plus
  waypoint handoff-marker assembly; it receives already-computed guidance
  products and does not recompute control decisions
- `transfer/waypoint.rs` owns pure waypoint geometry, capture prediction, and
  handoff kinematics
- `transfer/experimental.rs` owns the frozen boost-scoring mode gates and
  weights retained for diagnostic reproducibility
- `transfer/tests.rs` owns the transfer and waypoint controller tests without
  changing their access to module-private fixtures

The controller evaluator follows the same separation. Persisted batch/report DTOs live in
`pd-eval/src/model.rs`; pack validation and expansion live in `resolution.rs`;
execution, artifact/cache support, comparison, and review derivation live in
their named modules. The batch report shell delegates overview, diagnostics,
review-tree, and comparison rendering to `pd-eval/src/report/` modules. Public
crate exports and persisted schema paths remain unchanged.

The V1 planner boundary places neutral route-planning contracts and clearance
queries in `pd-core`, with the deterministic `pd_plan::plan` algorithm
depending only on `pd-core`. `pd-eval` calls that planner during V1 scenario
resolution, persists algorithm/policy/plan identity, and passes the resulting
`TransferRouteSpec` to guidance. `pd-control` is not a planner dependency, and
planner policy does not read controller configuration defaults.

The native V2 evaluator uses a separate policy-3 flight loop and native batch
schema; it does not change `pd_plan::plan`, the ordinary `pd-cli run`
controller path, or add a V2 controller. The common batch shell and rich detail
renderer are shared through `pd-report`; V2-specific aggregation and handoff
annotations remain in its evaluator/report adapter. See the
[common-template results](planner_v2_common_report_templates_results.md).

This split is internal. Public controller exports still resolve through
`pd-control`, and persisted controller, phase, telemetry, and artifact contracts
remain unchanged.

## Native V2 Acceptance Contract (2026-10-03)

The approved reliability pass adds a versioned, evaluator-owned acceptance
check for the frozen `planner_v2_lab_suite`; implementation and final-source
validation are in progress. This is not a controller comparison policy or a
new terrain experiment. The contract is settled before the measured runs:

- Bind the complete 44-case input identity to expansion of the tracked pack,
  not merely to mutually consistent hashes supplied by a capture. Require
  policy 3, completed capture status, and unchanged source/input provenance
  during capture. An older capture need not match today's Git commit.
- Require all 36 ordinary cases to have target touchdown, mission success,
  landed planning stop, integrity, and source-replay evidence. The 11 clear
  controls must be terrain-unblocked and use zero corrections. The 25 terrain
  cases must have a blocked initial nominal and at least one correction.
- Keep the eight diagnostics separate. The six supported diagnostics may land
  or end at an honest finite planning stop, but may not crash, land off-target,
  time out in the core, or hide an implementation/unverified-execution failure.
  The two expected unsupported inputs must remain preflight-only, with no
  invented physical outcome, mission outcome, or replay claim. Previously
  difficult diagnostics are allowed to improve.
- Require integrity for all 44 cases and recorded source replay for all 42
  supported cases. Do not fix the total number of handoffs or impose a new
  machine-dependent timing threshold.
- Saved-capture checking is read-only artifact/identity/outcome validation. It
  checks recorded replay evidence; it does not run a fresh physical replay or
  establish arbitrary-terrain or real-time acceptance.
- A failed capture keeps its evidence and rich report, but cannot replace the
  current accepted site. The checked batch workflow returns failure after
  recording the result; collection-only operation distinguishes completion
  from acceptance explicitly.

The final validation allowance is one fresh 44-case batch and, only after it
passes, one repeat: at most 88 measured case attempts. Unit tests and read-only
artifact comparisons are separate checks, not additional measured batches.
Freeze source before these captures; retain every miss, and do not alter cases,
flight policy, or acceptance thresholds in response to results. Local
checkpoint commits are authorized; pushes, server restarts, runtime-default
changes, new mission corpora, and planner tuning are not part of this pass.

## Optional V2 Runtime Consumer: Next-Phase Design

This is a design-only follow-up, not an implemented adapter or a requirement
for usable offline lab evaluation. A concrete runtime consumer should be named
before implementation; the ordinary `pd-cli run` and controller defaults stay
unchanged in the reliability pass.

The existing seam is `pd-eval::FlightLoop` in `waypoint_v2.rs`. It owns one
`OrdinaryLive` state, one absolute deadline, the consumed command ledger, and
the correction count. Queries clone that live state; executing to a handoff
advances the original state. The controller trait, by contrast, returns a
`ControllerFrame` and has no typed planning-stop result. A snapshot's
`to_simulation_state` is query-only and explicitly must not be stepped. Fixed
flight-program playback is not a dynamic replanning interface.

The smallest staged path is:

1. Extract an internal V2 session/iteration seam while keeping the existing
   evaluator driver and all numerical selection, query, admission, contact,
   certificate, and replay checks intact. Prove command/state parity before
   changing crate placement or adding a consumer. Do not assume every replay
   is removable post-processing: witnesses and prefix proofs also participate
   in admission.
2. Define a segment-or-stop contract bound to the original run context, actual
   full live state, incoming contact, global command clock, original deadline,
   consumed prefix, source-phase ownership, and completed correction count.
   A segment carries globally scheduled commands through its planned handoff;
   the next planning call receives the actual state after execution. There is
   no simulator restart, fuel refill, fresh deadline, or reconstructed live
   state. Keep predictions distinct from executed handoffs.
3. Add an optional V2-specific driver only after that seam passes parity. It
   must stop advancing on a typed finite stop and retain partial evidence,
   rather than issue idle commands until timeout. `NoClearing` means the local
   search found no admitted proposal; `CorrectionLimit` means the next nominal
   remains blocked after the correction allowance was reached. A final direct
   attempt remains permitted after the last allowed correction. Ordinary
   controller APIs need not be generalized for this
   first optional consumer.

Keep the evaluator's final source replay, batch policy, and reporting outside
the consumer. Decide shared module/crate placement only if a second consumer
actually needs it. Do not introduce a `pd-control` dependency on `pd-eval`, or
put simulation/report orchestration into the neutral `pd-plan` crate.

The future implementation gate should cover direct and repeated-correction
flights, finite pre-departure stops, correction-limit/deadline boundaries,
unsupported preflight, and no commands after stop. Verify actual E/H state,
monotonic clock, held command, incoming contact, fuel continuity, unchanged
ordinary CLI defaults, and parity with the existing evaluator's frozen inputs.
New measured captures need their own approved allowance. Retain the current
vehicle/Earth/120-60 Hz domain and local-clear-then-replan behavior; neither a
landing suffix nor feature-far-edge clearance becomes a new requirement.
Hard real-time or asynchronous planning is a separate measured design question,
not a claim derived from the offline planning timings.

## Maintained Gates

The counts below are retained outcomes for the maintained controller/guidance
corpora, with freshness and source scope defined by their capture records; they
are not a recapture at current HEAD and do not represent the separate native V2
44-case batch. See the [V2 activation results](waypoint_v2_eval_activation_results.md)
for that active evaluator baseline.

The guidance regression set is:

- terminal bot-lab and trajectory-error smoke packs
- `transfer_route_angle_radius_suite`
- paired waypoint turn and ordered smoke landing/contract packs
- paired full-seed nominal waypoint landing/contract packs
- paired all-radius waypoint landing/contract packs

Current schema-34 primary evidence is:

- clean terminal: `171 / 189` physical successes, with `9` scored failures and
  `9` analytic invalidations
- trajectory-error terminal: `694 / 756` physical successes, with `26` scored
  failures and `36` analytic invalidations
- direct transfer: `297 / 297`
- all-radius turn landing and contract: `405 / 405` for both
- all-radius ordered landing and contract: `135 / 135` for both
- focused generated-route landing: `54 / 54` with zero invalidations
- focused generated-route handoff/ordered contract: `36 / 36` with zero
  invalidations

Supporting full-seed nominal waypoint evidence remains `540 / 540` for turn
landing/contracts and `180 / 180` for ordered landing/contracts. Bounded final
authority-recovery search closes the former
`single_gentle_bend_v1/full/r-30/short/seed 02` landing residual without
changing waypoint contracts or adding route/profile branches.

The report catalog also retains the direct-transfer solved-region and focused
`r+80` full-seed captures as supporting schema-32 evidence (`1080 / 1080` and
`108 / 108`). They remain valid outcome history but do not carry every
schema-34 waypoint/terminal-recovery field; recapture them only when current
schema evidence is needed.

## Experimental Boundary

Pathwise boost scoring, recoverability-weighted boost scoring, and the
no-terrain terminal alias remain reproducible diagnostics. They are not
maintained guidance modes and must not influence default controller behavior.
Their code is isolated from the maintained path, and their comparison packs use
the `diagnostic` expectation tier plus `experimental` tags. Reopen them only
when a new hypothesis justifies another experiment.

## Consolidation Rule

The 2026-07-13 guidance consolidation was behavior-preserving at the
then-current schema `33`: it did not change thresholds, candidate ordering,
route geometry, phase transition conditions, or artifact contracts. Schema
`34` was added later for explicit terminal-recovery evidence. Future structural
cleanup follows the same rule; behavioral changes require a separate controller
change with fresh evaluation evidence.
