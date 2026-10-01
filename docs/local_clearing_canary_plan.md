# Local clearing canary plan

## Decision and scope

The next phase should prove one dynamically generated clearing maneuver and
replanning from its actual handoff. Keep the canonical direct generator fixed.
Choose a maneuver for local clearance and progress, not for whether it can land
directly afterward. Another waypoint may be necessary.

This is a proposed implementation plan, not a sealed protocol or measured
clearing result. The foundation is committed at `d5dac57`; its review passed
842 workspace tests, one existing ignore, strict all-target Clippy, formatting,
and whitespace checks. This additional design review rechecks the retained
traces and source hashes, and performs analytical arithmetic only. It does not
rerun flights or workspace tests. Production V1 and the parked completion
reserve remain unchanged.

Use one already exposed physical pair: the 900 m flat control and its existing
315 m late/broad obstruction. Do not change its geometry to obtain success.
The [canonical results](canonical_initial_direct_canary_results.md) establish
the intended distinction: the fixed 232.373 m-peak nominal transfer is blocked,
although the old terrain-aware generator can fly a higher direct route.

Stop after this local capability and handoff compatibility. Repeated waypoints,
production wiring, arbitrary incoming states, disturbances, swept collision,
and real-time planning are not part of this phase. This planning pass runs no
new flight and implements no maneuver.

## Why intervention must be measured first

On the obstruction, canonical source handoff is tick 1992, or 16.6 s, at
approximately (-551.077, 221.582) m with velocity (66.462, 14.591) m/s. The
first actual reserve violation occurs at tick 2354, or 19.617 s, during powered
terminal flight. Only about three seconds separate source handoff and conflict.
The earlier triangular twins have still shorter source-handoff-to-conflict
intervals. These observations do not prove that a redirect is impossible:
braking can also change the time available. They do rule out assuming that an
existing near-apex capture is an adequate intervention state.

Detect blockage while planning, before executing the obstructed part. Retain
the first-conflict tick and position as diagnostic/search evidence, never as
an executable restored state or an automatic intervention boundary.

The existing airborne generator admits idle-throttle coast, not powered incoming
states. A new clearing primitive may accept a running powered state while
preserving its real attitude, angular rate and held command. Do not relax the
existing airborne admission contract. Arrange an idle-coast handoff and test
regeneration there as a separate gate; idle throttle alone does not prove that
its new position, velocity and attitude fit the existing finite family.

## Findings from the additional design review

The retained flat-control source replay contains the following ordinary
observations at the proposed earlier intervention boundaries. The obstructed
mission must independently reproduce its own actual prefix states during the
canary; these stored observations are not executable initial states.

| Source bridge fraction | Tick | Time s | Position x and y m | Velocity x and y m per s |
| --- | ---: | ---: | --- | --- |
| 25 percent | 552 | 4.6 | -895.397, 39.932 | 3.742, 11.462 |
| 50 percent | 1032 | 8.6 | -858.570, 94.447 | 16.075, 15.318 |
| 75 percent | 1512 | 12.6 | -755.228, 158.748 | 36.982, 16.361 |

The initial plan underestimated the distance remaining at these boundaries.
The conservative source acceleration cap is 13.32 m/s squared. At the lower
0.75 level, upright net ascent acceleration is only 0.18 m/s squared; at 30
degrees that level cannot sustain altitude. Earlier entry alone therefore does
not imply that a short maneuver reaches a useful handoff.

A continuous kinematic screen of the original 168-row grid found no row that
both satisfied the proposed progress boundary and maintained the body reserve
through its 2 s continuation. The screen assumes instantaneous commanded
attitude and constant acceleration, omits throttle/fuel dynamics, and samples
the existing conservative rotated-body envelope against the heightfield. It is
not a plant flight, a formal reachability bound or proof that every physical
execution fails. It identifies a design risk before sealing the policy:
the fixed 0.5 s coast ends many otherwise useful lofts before they reach the
obstructed station.

Using the observed 75 percent state, upright maximum-cap examples make the
alternative concrete. Continuous arithmetic gives the following handoffs at
the first even tick satisfying the progress condition:

| Powered lift s | Coast to handoff s | Approximate handoff x and y m | Approximate handoff vx and vy m per s | Height after another 2 s m |
| ---: | ---: | --- | --- | ---: |
| 6 | 5.283 | -337.953, 380.887 | 36.982, -14.408 | 332.450 |
| 8 | 3.283 | -337.953, 494.995 | 36.982, 12.232 | 499.838 |

These are hypotheses, not accepted waypoint states or command seeds. For an
upright body over the 315 m plateau, COM height must remain at least 325 m to
provide the 5 m reserve. Both examples warrant a physical test, but the 6 s
example has much less continuation slack. Neither proves that the unchanged
airborne family can generate a nominal transfer from its resulting energy state.

The illustrative calculation uses `x_b = x_0 + vx_0 * B`,
`y_b = y_0 + vy_0 * B + (13.32 - 9.81) * B^2 / 2`, then
`y_h = y_b + vy_b * C - 9.81 * C^2 / 2` during coast. Choose the first even
coast tick where `x_h >= -338.057443`; that threshold is the retained first
conflict x plus the unchanged 12.806248 m conservative body diameter.
The physical canary must instead use actual rate-limited attitude, held-pair
throttle and the original semi-implicit plant. Analytical screening must not
prune its sealed physical rows or substitute for replay.

The revision is to derive handoff timing from a bounded coast trace. Keep the
powered template grid, but allow up to 6 s of coast and select an actual eligible
state. Do not prescribe either analytical example as the expected winning row.

## Proposed local maneuver family

Start with forward-simulated control templates rather than guessing a waypoint
position and independently assigning its arrival velocity. Each template has a
bounded powered loft segment followed by at most 6 s of explicitly commanded
idle coast toward upright attitude. Propagate the existing plant, including
actual mass, fuel burn, throttle mapping and rate-limited rotation. Derive
handoff position, velocity, attitude and tick from an eligible coast state.

The proposed grid uses target attitudes -30, 0 and +30 degrees, two acceleration
levels at 0.75 and 1 times the existing worst-mass derated source acceleration
bound, and powered durations 0.5, 1, 2, 3, 4, 6 and 8 s. Negative attitude allows
braking while climbing. Map desired magnitudes through the existing held-pair
throttle conversion; retain saturation and feasibility failures, rather than
clamping an unrealizable desired trajectory into an accepted one.

The acceleration bound remains
`thrust_derate * (1 - declared_robustness_margin) * max_thrust / (dry_mass + max_fuel)`.
Both the coast and subsequent continuation command zero throttle and
upright target attitude at every global control update; actual attitude still
evolves under the unchanged rotation model.

Evaluate four intervention boundaries: the first complete coast hold and the
75, 50 and 25 percent boundaries of the canonical source bridge, rounded down
to global even ticks. These are 42 powered templates at each of four states,
at most 168 control rows. Every state must be obtained by fresh ordinary
execution of the generated canonical prefix on the actual obstructed terrain,
stopping before its scheduled command at that boundary. Reject unsafe or
unsupported entry states honestly. Source-bridge states are not a retrospective
rewind of an already flown mission.

For source handoff tick `S`, the source-bridge boundaries are
`72 + 2 * floor(fraction * (S - 72) / 2)` and the coast boundary is `S + 2`.
The 72-tick launch and every preceding command are unchanged.

After each powered segment, examine the 360 globally aligned boundaries at
2, 4, through 720 coast ticks. The first possible handoff follows one complete
idle control hold, so its held command is actually idle. For a boundary to be
eligible, its prefix must be safe and it must satisfy progress, admission and
the separate 240-tick continuation check below. Pick the earliest eligible
boundary for that row. There are at most 60,480 bounded handoff checks, not
60,480 independent powered simulations.

Propagate one coast trace per row through at most 6 s plus the 2 s continuation
window. Reuse that trace for window queries; do not run another 2 s simulation
for every handoff candidate. Stop tracing on contact, a nonfinite state, the
absolute deadline or the first reserve violation. An earlier handoff whose
entire certificate completed before a later conflict remains eligible: a
collision beyond its finite certificate is not retroactive local failure.
Retain row counts, tested boundary decisions and trace-stop causes honestly.

For entry tick `E`, powered duration `B` ticks and coast index `k` from 1 to
360, test `H = E + B + 2*k` with continuation endpoint `H + 240`. Checked
clock arithmetic and the original deadline apply before stepping. Keep the
coast endpoint tests separate from the 168 powered-row count.

These constants and formulas are proposed choices, not evidence of coverage.
Before flights, review the grid and seal its exact policy, inputs, ranking,
limits and source closure. After measurement, no per-case template expansion,
earlier unsealed intervention, geometry change or margin relaxation is allowed.
Finite exhaustion means LocalUnknown, not physical impossibility.

## Local acceptance contract

Accept a local candidate only when all of the following hold:

- Its entire actual trajectory is contact-free, inside the terrain domain and
  above the existing 5 m rotated-body reserve. The unchanged source launch
  exceptions apply only to the preceding canonical prefix; admit the clearing
  entry only after full airborne clearance is established.
- At handoff it remains Flying, InProgress and Running, with forward velocity,
  positive fuel, a globally aligned clock and idle held throttle, and before
  the target center. Require the unchanged airborne admission predicate, but
  not a successful direct proposal. Preserve all other live state; do not
  manufacture an upright or zero-rate snapshot.
- Handoff center x is beyond both the entry x and the first nominal conflict x
  by at least one conservative body diameter. This is a proposed anti-stalling
  progress threshold, not the obstruction's far edge. A handoff above the
  plateau is legal even when that plateau continues ahead.
- An explicit additional 2 s idle-coast command schedule from this state is
  contact-free and satisfies the same body reserve and original absolute budget.
  Every aligned boundary in that continuation remains admissible to the
  existing airborne generator, including forward motion, positive fuel and
  position before the target. Record the entire schedule, trajectory and
  remaining resources. Do not accept a short collision-free coast that silently
  carries the craft past the supported replanning domain.

Compute the conservative body diameter as twice the largest COM-to-hull-corner
or COM-to-foot distance from the unchanged vehicle geometry. Use
`handoff_x >= max(entry_x, first_conflict_x) + diameter`; do not tune a separate
distance for the obstacle.

The 2 s continuation is a finite nominal safety certificate. It is neither a
proof of eventual landing nor a worst-case compute-time or robust viability
bound. In this phase, planning still pauses the offline simulator. Test the
continuation on a separate execution branch; do not append it as missing-command
padding or silently consume it and then plan from a stale handoff state.

Admission and a collision-free certificate do not guarantee recoverable energy.
That remaining question is tested by the separate nominal regeneration gate,
not hidden in a requirement to land after the waypoint. No second maneuver or
general viability/abort guarantee is claimed when this finite certificate ends.

Within the sealed grid, prefer the latest intervention admitting a local
candidate, then earliest handoff, least fuel burn and stable row identity.
This diagnoses a coarse supported intervention window, not the physically latest
safe trigger. No landing result or subsequent direct-audit result participates
in local acceptance or ranking. Select once, then test composition separately.

## Execution and replay without checkpoint success

Keep the original route-free LandingOnPad mission. A clearing handoff is a
planning boundary, not a touchdown or mission checkpoint. Preserve the real
in-memory state and the original absolute deadline through that boundary.
Serializable snapshots are evidence only.

Do not package the local segment as a `FlightProgramV1` with an invented expected
contact. That existing type describes a complete source-to-target landing
program. Use a separately identity-bound local proposal with entry, powered-end,
handoff and continuation-end ticks, supplied updates, full query snapshots and
the original deadline. The canonical program is used only for the actual prefix.

Use the existing `run_simulation_bounded` and `replay_simulation_bounded` for
independent whole-source proof. Set command coverage to the exact handoff or
continuation endpoint, with the original absolute deadline retained separately.
The expected stop is coverage exhaustion while the state remains flying and
the mission remains in progress, not fabricated success or a new hard deadline.
Do not change core simulation behavior or the legacy replay path.

Compare the full final state, incoming contact option, actions, events and
samples against independent ordinary live stepping. Include a proof ending at
handoff and a separate proof ending after the 2 s continuation. Every consumed
60 Hz update must be supplied exactly once on the original 120 Hz clock.
Record genuine contacts before normalization on failure. The older airborne
stitched-replay helper expects terminal contact, so it cannot substitute for
this bounded flying-state proof.

For selected entry tick `E` and handoff `H`, the handoff action list is the
canonical updates strictly before `E` plus local updates from `E` through
`H - 2`. Its coverage endpoint is `H`; the hard endpoint stays at the original
absolute deadline. The continuation branch adds its explicit commands from
`H` through `H + 240 - 2` and stops at `H + 240`. Its expected contact option
is None. Do not duplicate the pending canonical command at `E`, issue an extra
command at either endpoint, or reset controller-update ordinals at the switch.

Use the existing sample cadence exactly. A flying handoff need not coincide
with a 10 Hz sample, so its full bounded final snapshot is authoritative even
when no sampled record exists at `H`. Do not insert a terminal-looking sample
or alter cadence merely to make replay arrays agree. Phase attribution for a
post-step state uses the command actually consumed before that tick, not the
next pending update at the same boundary.

The whole-source initial guard must retain the canonical source-pad launch
exception. Applying an airborne 5 m guard at source-rest tick zero would
reintroduce an artificial startup rejection. The local-entry guard requires
full clearance; it must not carry that launch exception into the clearing leg.

From the selected actual handoff, call the unchanged terrain-blind airborne
generator without a baseline suffix, local reference, expected row or terrain
seed. Then independently audit its single selected nominal proposal:

- Direct permits full suffix execution and ordinary landing replay, as extra
  evidence rather than a local candidate requirement.
- TerrainBlocked is a valid replanning outcome when backed by an actual terrain
  conflict and intact binding/parity. Retain it; do not execute the unsafe suffix
  or rerank to a taller direct arc. Another clearing maneuver belongs to a later
  phase.
- Unknown or unsupported incoming state is a composition coverage failure.
  Retain any successful local-clearing proof separately. Do not retrospectively
  reject/rerank the local candidate until a suffix lands.
- Invalid bindings, missing commands or replay mismatch are execution/evidence
  failures, never terrain blockage.

For a Direct suffix, whole-source landing replay must stitch the canonical
updates before `E`, the actually executed clearing updates from `E` before `H`,
and the newly generated suffix from `H`. The existing airborne stitched helper
extracts its prefix from an original complete landing program; passing the
unchanged canonical program there would replay the obsolete commands between
`E` and `H`. Reuse the lower-level full-source action replay with the real
combined update list instead. Keep full final state and incoming contact parity,
not merely agreement with the suffix's compact state.

## Implementation order and file responsibilities

First write and review the canary protocol and input-selection manifest. Pin
the existing two physical requests and their exposure history; seal the revised
grid, 6 s coast bound, 2 s certificate, progress formula and ranking before any
physical measurement. Analytical examples remain review context, never expected
answers embedded in the harness.

Add `pd-plan/src/local_clearing.rs` for pure policy validation, template
enumeration, stable row identity and local goal/proposal contracts. The pure
enumerator receives supported state/vehicle scalars and the audited local goal,
not an archived route, old winning row or target-landing reference.
Full live-state and snapshot envelopes remain in `pd-eval`; do not introduce
an evaluator-type dependency into `pd-plan`.

Add `pd-eval/src/local_clearing.rs` for real prefix collection, held-pair plant
propagation, strict terrain audit, bounded coast-window selection and selected
live-state execution. Reuse existing throttle/body-query math with minimal
visibility changes; do not fork those equations or change their semantics.
Keep proposal math and actual-terrain validation visibly separate.

Add `pd-eval/src/local_clearing_canary.rs` for create-only evidence, gates and
repeatability. Register an additive `local-clearing-canary --output-dir NEW_ROOT`
command with no promotion flag or per-case tuning switches. Reuse the core
bounded executor/replay, not a new simulation API. Finish the integrated harness
and seal its source closure before the first new physical measurement.

Primary owns local acceptance, trajectory math, policy, integration and verdict.
If a later worker loop is authorized, a bounded worker may own evidence/CLI and
structural tests after those contracts are settled. `pd-control`, core physics,
production V1, existing canary policies and terminal reserves are out of scope.

## Measurement gates and stop rules

1. Baseline and entries. Require safe flat-control landing, identical complete
   canonical searches on both terrains, and a parity-backed terrain conflict on
   the obstruction. Obtain all four actual entry states from ordinary prefixes.
   Unsupported/unsafe entries are retained exclusions, not restored substitutes.
2. Local generation. Search the sealed family and select by local criteria only.
   Rejecting one row is normal finite search; stop downstream work only when
   no local proposal exists or a binding/execution defect invalidates the gate.
   Retain all row and handoff-boundary decisions.
3. Local physical proof. Execute the selected trajectory from the retained live
   entry and require full-state/contact/action/event/sample parity in independent
   whole-source bounded runs at handoff and certificate endpoint. Require exact
   coverage stops while the original mission remains in progress.
4. Handoff compatibility. Generate a fresh nominal segment from the actual
   handoff. Direct or a genuine TerrainBlocked result passes this gate. Unknown
   or unsupported state stops at a composition coverage failure; keep successful
   local proof without choosing another row for easier landing. If Direct,
   execute and replay its full suffix as an additional result. An execution
   mismatch there remains an evidence failure, not successful composition.
5. Closure and preservation. Repeat in two new roots on unchanged final source.
   Compare complete deterministic payloads, excluding only predeclared timings
   and paths. Preserve generation, commands, full states/contact and replay for
   the eight canonical controls, prior twelve airborne continuations and the
   24-control source-rest population. Legitimately expanded source bindings may
   change envelope identities; they cannot excuse changed physical evidence.
   Run focused negatives, workspace tests, strict Clippy, formatting and
   whitespace checks.

At every decisive gate failure, record its actual cause and mark later gates
not evaluated. Do not retune the sealed family, shrink the obstacle, extend the
certificate after results or reopen the old direct policy to manufacture a pass.

Negative tests must cover a conflict/unsafe starting state, powered-state
rejection by the unchanged airborne generator, insufficient progress, unsafe
or admission-losing continuation, clock or fuel reset, missing/duplicate commands,
altered full state/contact evidence, and an unrelated audit error mislabeled
TerrainBlocked. Test launch-guard versus local-guard scope, first idle hold,
handoff/continuation endpoint off-by-one errors, a later conflict outside a
completed certificate, and attempted fake landing-program wrapping. Also test
that later TerrainBlocked does not invalidate local acceptance or trigger
reranking. Structural tests are not physical multi-obstacle proof.

Artifacts should include the sealed requests/policy/source hashes; unchanged
canonical search and first conflict; entry-state snapshots and prefix updates;
all powered rows with boundary decisions and failure categories; the selected
local updates, handoff and certificate; both bounded whole-source proofs; and
the complete fresh airborne search, fixed audit and optional landing proof.
Keep phase timings separate from physical identities and from real-time claims.
Bind the local policy, actual terrain, original mission budget, entry state and
all supplied commands. Validate these bindings independently before execution;
the nominal direct policy's deliberate terrain-independent identity must not
be mistaken for a terrain-independent local clearing certificate.

## Exit and investment decision

A full pass establishes one trajectory-derived waypoint, an explicit finite
safe continuation and actual-state replanning on an unchanged demonstrated
obstruction. It does not establish a usable full multi-waypoint planner.

If local generation fails, report whether the finite family rejected the late
coast boundary and the earlier powered boundaries, with their actual causes.
If local clearing passes but regeneration fails, keep the local proof and make
that exact handoff-coverage gap the next decision. Do not reopen source launch,
landing physics or the completion reserve without evidence identifying them.

This bounded pass is worth doing: it tests the central missing capability rather
than expanding direct-flight research. Reconsider further investment after its
first decisive gate if success would require a broad powered-state optimizer,
relaxed safety contract or production redesign. No additional maneuver family,
second waypoint, default promotion, commit or push is automatically authorized
by this plan.
