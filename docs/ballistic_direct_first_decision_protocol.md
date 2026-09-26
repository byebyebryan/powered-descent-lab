# Ballistic-direct-first planning decision protocol

## Status and purpose

The analytical boundary result and original selected-profile execution stop
below remain historical evidence. The later
[complete-flat acceptance checkpoint](waypoint_direct_complete_flat_acceptance_protocol.md)
accepts new launch-aware nominal wrappers on the flat control; it does not
reopen this controller slice or certify the original selected trajectories.

This is a predeclared, opt-in development gate. It does not change the V1
planner, controller, simulator, F6, default mission selection, or any frozen
research artifact. Its question is whether the existing bounded V2 ballistic
family can make the intended setup-time topology decision on uncut terrain:
try a landable ballistic source-to-target transfer first, then search for a
waypoint route only when that direct family has no certificate.

`Direct` here means a source powered bridge, one exact ballistic coast, and a
terminal powered bridge with no operational waypoint handoff. The coast must
clear terrain and meet a descending terminal bridge that passes the shared
thrust, slew, fuel, time, clearance, touchdown, and robustness screens. An
endpoint-shaped pad-to-pad chord is not a direct-flight feasibility test.

The decision is made before departure. A committed nominal arc blocked after
launch is a separate local-recovery problem: a higher direct arc available
before launch must be considered before calling a waypoint necessary. A
failure of this finite V2 family means `not_certified_under_policy`, not
physical impossibility or failure of every possible ballistic transfer.

The V2 certificate is controller-independent and is not a command stream.
Observed `transfer_pdg` flights, V2 certificates, route projection, and
authoritative simulator contact remain separate evidence. In particular, a
simulator landing on a zero-waypoint route does not prove it tracked the V2
reference, and a V2 certificate does not prove a controller landing.

## Frozen inputs and terrain family

Reuse the existing `waypoint-direct-primitive-analytical` continuous-flat
800 m probe, vehicle, 120 Hz gravity/plant model, 90 s effective policy, and
four duration multipliers without modifying them. Rebuild and bind the
existing flat, uphill, and downhill analytical baseline before evaluating new
terrain. Its accepted semantic identity is `fnv1a64:d3fa6b24336f7c05`;
the prior 24-cell sweep identity is `fnv1a64:1bcd5a3bd6c6da01`. Treat an
identity drift as a failed input gate, not as a new baseline. The baseline
must retain at least one certified direct candidate in each of the three
uncut cases.

The overflight positive control is the existing centered, wide 160 m obstacle:
center fraction `0.50`, base-width fraction `0.25`, height fraction `0.20`.
It must remain `analytical_direct` under the unchanged V2 policy.

The new boundary grid changes terrain only. Keep the original sweep's
trapezoid construction: base edges at center +/- half base width, top edges
at center +/- one quarter base width, and top height above the flat baseline.
Evaluate cells in center, then height order:

| Parameter | Frozen values relative to the 800 m span |
| --- | --- |
| Center fraction | `0.35`, `0.50`, `0.65` |
| Base-width fraction | `0.25` |
| Height fraction | `0.45`, `0.50`, `0.55`, `0.60` |

This is twelve cells: a natural extension above the previous `0.40` tier,
not a height chosen from a particular certified apex or controller outcome.
No cell-specific policy, vehicle, controller, or duration tuning is permitted.

Add one deliberately out-of-envelope diagnostic control at center `0.50`,
base width `0.50`, and height `2.00` of the span. Its purpose is to exercise
the `unknown`/no-certificate path, not to model a plausible gameplay mission.
If it unexpectedly certifies a route or is invalid, record that result and
stop for review instead of changing its geometry.

## Ordered analytical decision and first gate

For every valid cell, evaluate all four V2 direct candidates. If any certifies,
classify `analytical_direct` and select the shortest total-time certificate,
then stable candidate identity. Do not invoke waypoint search for that cell.

Only after zero direct certificates, generate the same four terrain-derived
waypoint proposals as the previous sweep. With `L` and `R` the obstacle base
edges, `T` its top, and `dx,dy` the shared V2 hull-envelope spacing, they are
`(L-dx,T+dy)`, `(L-2dx,T+2dy)`, `(R+dx,T+dy)`, and `(R+2dx,T+2dy)`; sort by
position without clipping. Search the unchanged finite one-waypoint duration
pairs and stop at the first certified witness. Classify a certificate as
`analytical_one_waypoint`; valid exhaustion as `unknown`; setup failure as
`invalid_setup`. Record every direct rejection reason and waypoint-search
count. An analytical waypoint result is a bounded policy result, not proof
that all physically possible direct arcs are blocked.

The first gate passes only if:

1. The three uncut baseline rows and the overflight control remain direct.
2. At least one of the twelve boundary cells is `analytical_one_waypoint`.
3. For the first such cell in frozen order, at least one duration multiplier
   that certified on the flat twin becomes rejected with the V2
   `terrain_clearance` reason or a nonzero terminal-environment rejection
   count. Preserve all other rejection reasons; source-only or invalid-input
   failures alone do not qualify. The V2 model does not independently prove
   closed-loop recovery after an obstacle, so label that question unresolved
   rather than inferring it from
   a terminal-bridge failure.
4. The out-of-envelope control is valid and `unknown`.

If any condition fails, stop the pass and publish the complete finite result.
Do not shift an obstacle, retune the V2 policy, or replace the miss with the
older committed-nominal ridge canary. If a witness exists, retain the first
one by frozen order and use it as a development case, not held-out evidence.

## Conditional execution and promotion boundary

Only after the analytical gate passes, investigate a versioned opt-in
decision-to-route-to-controller slice for the first witness and the direct
controls. Before an execution claim, resolve the source-transition route
validation mismatch and the core/V2 footprint-angle convention explicitly;
neither should be waived by altering terrain or the V1 validator. Declare
whether the controller is tracking the ballistic reference or merely flying
the selected zero-/one-waypoint route, and label outcomes accordingly.

Require the unchanged controller and authoritative simulator to pass source
departure, any waypoint handoff, en-route clearance, terminal entry, and
stable target contact on the retained case. Preserve V1's focused baseline
and frozen F6/research identities. A 60 Hz exact source-handoff diagnostic is
not a topology gate unless the chosen executor actually relies on that
feedforward schedule.

This pass cannot promote a default planner. A later promotion would need
fresh held-out terrain and explicit direct/waypoint/no-certified-route
outcomes, with the same policy and no post-result tuning.

## Recorded boundary result and execution stop (2026-09-25)

The frozen-input gate passed with the accepted baseline and prior-sweep
identities above. All three uncut flat/uphill/downhill controls and the 160 m
overflight control retained `analytical_direct`; the overflight probe and
classification also matched the corresponding prior-sweep cell. Of the twelve
predeclared boundary cells, seven were `analytical_direct` and five were
`analytical_one_waypoint`. The deliberately out-of-envelope control was valid
and `unknown`.

The first witness in frozen order was `center_035_width_025_height_055`: an
800 m flat-pad span with a 200 m-wide, 440 m-high trapezoidal obstacle centered
280 m from the source. None of its four direct V2 duration candidates
certified; the unchanged waypoint search found a certificate after 48
candidates (`fnv1a64:a2061e6781764a52`). The `1.0` and `1.25` duration
multipliers certified on the flat twin but rejected here with
`terrain_clearance` among their recorded reasons and nonzero terminal-
environment rejection counts. The first gate therefore passed. This is a
finite-policy waypoint-needed result, not a proof that every possible direct
ballistic arc or real controller recovery fails.

Two fresh-path summaries under
`outputs/research/ballistic_direct_first_decision_20260925/run_a/` and
`run_b/` were byte-identical (SHA-256
`65ad8538d901998ce6473af6d4248de521ef81901382d20d03bca3993247da5d`,
semantic identity `fnv1a64:884f773f4009c303`). The integrated workspace gate
passed 670 tests; strict Clippy, formatting, and diff checks passed. No
production/default planner, controller, simulator, F6 selection, or frozen
artifact was changed.

The controller slice stopped before execution. The public F6 projection is
bound to its ridge canary: supplying this witness as its generic input would
cause F6 to derive a different mesa rather than preserve the witness's exact
terrain. Reusing that run would not test this boundary decision. An opt-in
candidate-to-runtime adapter must preserve the selected candidate, raw
terrain, and stable identities; produce a zero-/one-waypoint route and
handoff from those exact inputs; and fail closed on changed joins. Its
ballistic-specific path contract must state why the ordinary endpoint-shaped
`validate_route` source-taper rejection does not assess the certified arc,
without weakening that validator. The V2/core tilted-footprint sign mismatch
must be reconciled or conservatively checked for these exact trajectories
before treating a V2 clearance certificate as simulator-compatible. Only
then should the unchanged controller be run, with route-following observations
kept distinct from V2-reference tracking. No controller outcome or landing
claim is made by this pass.

## Exact-execution preflight stop (2026-09-25)

The next pass stopped at its first, certificate-to-simulator compatibility
gate. The accepted boundary artifact (`fnv1a64:884f773f4009c303`) still binds
the frozen primitive baseline (`fnv1a64:d3fa6b24336f7c05`). Its
`continuous_flat_r00` direct control selects the shortest certified candidate
`fnv1a64:4e6c0b23f9eb1b8f`. The earlier identity-bound source-contact audit
(`fnv1a64:81b3fe5b31349524`) included the same baseline's native and
shortest-certified roles and reported that every original profile passed the
V2 source-clearance screen but was classified as a simulator crash on physics
step one. The current boundary evaluator and that audit use the same
shortest-time-then-identity rule. Thus the flat direct control cannot pass a
gate requiring its *selected V2 trajectory* to depart safely in the unchanged
simulator. This is an identity/provenance link to sealed prior physics
evidence, not a new controller or physics run in this pass.

The subsequent launch-feasibility study obtained contact-free departures by
adding a fixed launch and re-solving the remaining bridge. Those wrappers
have new identities and mixed analytical/terminal results; they cannot be
substituted for this selected candidate while claiming the old certificate.
No exact-terrain runtime adapter or controller lane was built or run after the
failed preflight. A zero-waypoint route flown by the unchanged controller
could still depart differently; this stop does not predict its outcome. It
only blocks the proposed certificate-to-simulator compatibility claim.

Reopening this execution slice requires a versioned candidate whose source
departure is contact-safe under core geometry from the pad-rest state, with
the changed launch, fuel, time, attitude, and handoff included in the new
certificate. The core/V2 footprint convention, terminal first-contact
screen, and controller cadence remain separate gates; neither the floor nor
the V1 validator should be changed to hide the source failure.
