# Body-aware nominal direct terminal prototype

This is a new, opt-in evaluator policy, not a change to `pd_plan::plan()`,
controllers, core physics/contact rules, the V1/V2/F6 decisions or defaults.
The September 27 terminal diagnostic is a preserved causal checkpoint, not a
generator input. Its original 34 accepted / 42 rejected ledger stays unchanged.

## Question and stopping gate

Can the unchanged input-driven launch/source policy support a generic terminal
reference that is body-safe at actual first core contact and executable with
commands held on the intended global 60 Hz clock? Test the smallest added
reference freedom first: quadratic horizontal and affine vertical net
acceleration, with zero final horizontal acceleration. Finite failure is
`unknown`, not a physical-impossibility or waypoint-necessity result. A staged
capture tail, larger search/solver budget or full optimizer is a separate
decision; do not add one automatically to rescue this pass.

The primary owns the reference/executor, policy, physical acceptance and final
review. Luna owns the bounded runner/preflight/create-only/CLI surfaces and
focused runner tests. No commit, push or deployment is authorized here.

## Pre-implementation seal

The complete ten-case manifest was written before prototype implementation or
any candidate/physics evaluation on those cases:
`fixtures/research/waypoint_direct_body_aware_terminal_fresh_inputs_v1.json`.
Raw SHA-256:
`f7b982724cfc8e4df7fd312519a11cfcb1cea25348c7e9451269c8a3e175ddb7`.

Order: span 750 m, then 950 m; each flat, uphill +100 m, downhill -100 m,
high obstacle and late/broad obstacle. The high trapezoid is centered at 60%
source-to-target progress, with plateau width 10% span, ramp width 5% span on
each side and height 55% span. The late/broad trapezoid is centered at 72%,
with plateau width 20%, two 10% ramps and height 25%. Flat shelves are 36 m
wide, terrain domain padding is 160 m, and uphill/downhill terrain connects
the shelves continuously. No cutaway or authored waypoint is present.

All cases retain the supported vehicle, 120 Hz physics, held 60 Hz commands,
9.81 m/s2 gravity, 90 s horizon, 10 s reserve and source-pad upright rest.
Case labels are identities only. The terminal policy below is identical for
all development and held-out requests, and validation rejects tuned fields.

## Generic finite policy

Reuse `evaluate_waypoint_direct_nominal_direct_generation` from scenario
inputs only. It retains the four V2 duration bases and five source-duration
offsets; no historical summary supplies commands, seeds or decisions. Its
original full family is a separately identified comparison, not new-policy
acceptance. Preserve every skipped row and its source-stage reason.

For every generated source schedule, keep all launch/source command payloads
unchanged. Reconstruct and independently replay that prefix. Coast remains
zero thrust; its desired attitude is now the first terminal paired-mean thrust
direction. At most one additional idle physics tick can extend coast so the
powered tail starts at a real global 60 Hz update boundary. Record that tick,
its physical translation/attitude transition and time cost; never reset clocks
or pretend a command can update off-clock. Reject an insufficient physical
attitude join rather than instantaneously assigning a body pose.

Starting at the actual replayed coast endpoint, construct exact discrete
accelerations under the unchanged velocity-first plant integration:

- Horizontal acceleration is quadratic in normalized tick index. Three
  coefficients satisfy final target-center position, zero horizontal velocity
  and zero final horizontal acceleration.
- Vertical acceleration remains affine. Its endpoint is target surface plus
  body base offset minus 0.005 m, and downward speed is half the existing
  safe normal-speed limit. The small undershoot deliberately crosses contact;
  it is not permission to ignore an earlier contact or modify core geometry.
- Terminal-duration offsets are exactly `[0,60,120,180,240,300,360]` physics
  ticks from the original seed tail: seven choices, 0 through +3 s at 0.5 s
  intervals. Mission reserve and horizon still apply to the complete program.
- At most four additional physics ticks after the nominal reference end may
  continue upright terminal thrust to obtain first contact. Include these
  ticks in the planned time bound and audit them. No contact means rejection.

Before physical tail execution, audit each reference's actual first-contact
pose with the unchanged neutral core classifier and predicate mirror. The
reference-pose lane is conditional geometric evidence, not a physical flight.
It must pass safe stable target contact, strict terrain domain, reference
slew, derated thrust and minimum-throttle screens. Scan the conservative
core-rotated body AABB with exact heightfield queries at every airborne tick.
Outside the wholly-contained descending target-pad corridor require the
existing 5 m reserve plus a fixed 0.1 m reference construction buffer. Inside
that corridor require nonnegative airborne clearance. Physical full-program
verification retains the existing 5 m requirement, without weakening it.

For an admissible reference, pair adjacent thrust vectors by arithmetic mean.
Hold one desired attitude and throttle request across both physics ticks.
Invert scalar throttle against both post-consumption masses, using exactly
48 bisection iterations. Keep minimum throttle, thrust, fuel and physical slew
limits unchanged; saturation is a rejection, never a rescue. No source refit,
feedback controller tuning, terminal shooting correction or case-specific
coefficient adjustment belongs to this policy.

Retain every attempted duration and its earliest failure. Stop that row's
terminal search after its first independently accepted complete witness;
record remaining offsets as not attempted after acceptance. Rank accepted
rows by complete planned mission time then stable witness identity. Do not
rank by collision time or accept generator-provided pass flags.

## Independent complete-program verification

Replay stored command updates from scenario rest in both ordinary and neutral
core states, using the real global command clock. Verify command coverage,
source-prefix payload identity, supported departure, launch/source boundary,
strict 1e-6 m / 1e-6 m/s source handoff, actual descending clear terminal entry,
pointwise rotated-body clearance and first authoritative safe stable target
contact. Preserve incoming contact velocity/rate; ordinary stable-touchdown
zeroing is an explicit parity exception, not a zero-impact-speed claim.

Recompute mass/fuel, actuator/slew limits and complete actual/planned budgets.
Bind the request, original generator, new terminal policy/reference, unchanged
source prefix and exact new command program with distinct identities. Preserve
skips/rejections, contact margins, minimum clearance and deterministic setup
compute counters. A command program that is tampered, truncated, reordered,
off-clock or inconsistent with the reference must fail verification.

Reload verification regenerates the original input-driven family once per
case and reconstructs the entire new finite first-success ledger from those
inputs. Every row, attempted rejection, post-acceptance skip, accepted witness
and ranked selection must reproduce exactly; omitting a valid row or claiming
a later acceptance cannot be hidden behind a self-rehashed artifact.
Each accepted witness must match that family's exact launch/source
payload, source handoff, coast length/alignment and allowed terminal duration;
a self-rehashed saved prefix is not sufficient. Saved reference/contact
evidence must match independent recomputation. Rejected attempt diagnostics
remain reproducible generator evidence, not independently accepted flights.

## Gated evaluation

1. Input-only preflight validates all manifests/scenarios/policies and pinned
   historical comparison digests before simulation, candidate solving or
   output writes. Existing output roots and source/protocol/input drift refuse.
   Historical full-trace files are streamed through SHA-256 rather than
   parsed/re-serialized in preflight. Exact regenerated original-summary byte
   equality later binds their complete contents without loading old commands
   or duplicated metadata as generator inputs.
2. Development evaluates all six previously exposed uncut 600/1,000 m cases
   and all eight exposed obstacle 700/900 m cases from inputs alone. Compare
   the regenerated original-policy summary bytes to the historical copies;
   the new tail must keep every previously Direct case Direct and obtain a
   new complete witness for each of the two high cases. No old ledger changes.
3. The primary reviews and accepts development evidence, then creates a
   source/protocol/manifest freeze bound to that passing development summary.
   If development fails, publish the bounded failure and stop before held-out
   physics. Correct integrity/extraction defects without policy tuning; any
   policy redesign needs its own recorded boundary.
4. Under that freeze, evaluate all ten sealed cases, checking source identity
   between cases. A passing fresh capability gate requires a complete new
   direct witness for every case. Publish all outcomes, including valid
   finite unknowns, and stop. Do not retune inspected held-out cases.
5. Repeat accepted gates only into independent create-only paths for byte
   parity. Verify input and historical-output digests after evaluation.

## Validation and scope limits

Test exact polynomial endpoints and acceleration constraints, finite-policy
refusal, entry/global-clock handling, two-substep inverse, physical slew,
first-contact geometry, explicit crossing, missing contact, domain/corridor
clearance, command tampering, stable ordinary-zeroing parity, accepted-only
ranking, create-only outputs, source freeze and held-out ordering/refusal.
Finish with workspace tests, formatting, strict all-target Clippy and diff
integrity checks. Avoid repeated full-suite runs without a change or open risk.

These are discrete nominal simulator witnesses. No swept collision proof,
perturbation robustness, arbitrary incoming waypoint-state support, waypoint
composition, real-time planning or production/default authority is claimed.
The existing diagnostic is preserved but its all-source runtime binding is
historical once the new evaluator source is added; old result bytes and claims
must not be relabeled as a newly frozen run.
