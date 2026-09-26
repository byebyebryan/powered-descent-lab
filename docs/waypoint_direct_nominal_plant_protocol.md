# Direct-leg nominal-to-plant characterization protocol

This opt-in research checkpoint asks two different questions: can each nominal
V2 bridge/coast/bridge profile be executed exactly through the discrete plant,
and what happens when one deterministic feedforward command rule is actually
flown? An exact-profile mismatch is recorded, not treated as a crash or as
proof that direct flight is impossible. The simulator owns contact and mission
outcomes. No planner, existing controller, F6 path, or default changes here.

## Frozen inputs and selection lanes

Use baseline analytical artifact `fnv1a64:d3fa6b24336f7c05` and topology
sweep artifact `fnv1a64:1bcd5a3bd6c6da01`. The six cases are continuous
flat, uphill, downhill, and the centered/wide obstacle at heights 160, 240,
and 320 m (`center_050_width_025_height_020/030/040`). Rebuild and compare
each sealed artifact, probe, result, and candidate identity before any run.
Regenerate bridge samples from the stored analytical inputs because setup
artifacts intentionally omit runtime-only samples.

For every case retain two existing, distinct selection rules: the shortest
certified research choice (then identity) and the V2 evaluator's native
margin-first choice. If they choose the same candidate, keep both roles in
the evidence. Never replace one role after observing an execution outcome.
The previous downhill first-tick regression is part of the inputs, not a
reason to exclude that profile.

## One command rule, two cadences

For a nonzero requested thrust acceleration of magnitude `a`, pre-burn mass
`m`, maximum thrust `F`, maximum fuel burn rate `B`, and physics interval
`dt`, request applied throttle `f = a*m/(F + a*B*dt)`. This accounts for the
plant burning fuel before integrating translation. Convert a realizable `f`
to the simulator's command scale through minimum throttle. Zero requested
thrust commands engine off; nonzero requests below minimum command
`f64::MIN_POSITIVE` (a deterministic on-command that applies minimum
throttle), and requests above maximum clamp to full
throttle. Record every saturation rather than silently treating it as an
exact realization. Powered attitude targets follow the requested thrust
direction. During the ballistic coast, command zero throttle and immediately
turn toward the first terminal-bridge powered direction; otherwise retain
the last attitude target when no future powered direction exists.

At 120 Hz, issue a new command on every physics tick directly to the plant.
This is a diagnostic oracle, not the scenario's controller interface. At
the unchanged 60 Hz controller cadence, compute a command from the first
desired tick in each two-tick update interval and hold it for both physics
ticks. This is one predeclared zero-order-hold baseline, not an optimal
60 Hz command search. Use the scenario's unchanged vehicle, physics,
controller cadence, terrain, pads, initial state, and mission rules. After
the nominal profile ends, command zero throttle and hold the last attitude
target at the next normal command update until the simulator terminates. At
60 Hz, a profile ending between updates retains the prior held command for
the remaining physics tick; record that tail and profile-end state separately
from the idle-fallback outcome. Do not add a terminal rescue or per-case gain.

## Evidence and stop rules

For each case, selection role, and cadence, record the first nominal-state
deviation above `1e-6 m` position or `1e-6 m/s` velocity, plus desired and
actual attitude and throttle at that tick. Continue through nonfatal
deviation. Also record maximum state error, phase-handoff errors, saturation,
fuel and time, profile-end state, authoritative contact/mission outcome, and
minimum **en-route** hull clearance in the existing endpoint-transition
window. Keep global clearance distinct because it includes pad contact.
If contact ends a run early, retain an explicit partial row. Invalid input
identities stop all simulation before any row is flown.

The bounded pass is complete when all frozen rows have an accounted-for
result and repeated runs produce the same semantic summary. It does not
require exact parity, successful feedforward landings, or a particular
topology result. A failed feedforward baseline is not proof that feedback
tracking, another fixed command schedule, or a direct route is impossible;
a safe feedforward run is not by itself a robust-controller certificate.
The remaining 21 sweep cells are outside this checkpoint and are not a blind
holdout, since their earlier unchanged-controller outcomes were inspected.

## Recorded result (2026-09-23)

The input gate accepted all six cases before simulation under manifest identity
`fnv1a64:9e32aa0c1a52b28b`. Twelve selection-role labels resolve to nine
unique candidate profiles; each unique profile was flown at both cadences
(18 simulator rollouts). The final artifact identity is
`fnv1a64:9e8cbc902ca11fbc`. Independent runs in
`/tmp/pd-waypoint-direct-nominal-plant` and
`/tmp/pd-waypoint-direct-nominal-plant-repeat` produced byte-identical
`summary.json` files (SHA-256
`fb25306b8a62714b10d376d8171de9739e6fb457553ca76bbeae16e078998d35`).

All 18 rollouts terminated as simulator crashes on physics step 1, while
moving upward. None reached a phase handoff, the nominal profile end, an
en-route clearance sample, or the idle fallback. In eight of the nine unique
profiles, the first-tick position and velocity remained within the declared
parity tolerance before contact; this is **not** a finding of later-profile
parity. The research-shortest downhill profile alone breached it on tick 1:
requested first thrust attitude `-0.0302615 rad`, reachable attitude
`-0.01308997 rad`, velocity error `0.00185146 m/s`. The 60 Hz and 120 Hz
rollouts have the same first command, so this result does not compare their
later tracking behavior.

The first flat/native step illustrates the contact gate. Its center rose to
`y=5.000223 m`, but the commanded `-0.009446 rad` tilt left the lower
foot/hull about `0.03734 m` below the source pad while the vehicle was still
in the touchdown window. Its angular rate was `1.13357 rad/s`, above the
vehicle's `0.35 rad/s` safe-contact limit. Under the unchanged simulator
contact rule, that is a crash even though the center was ascending. The other
eight profiles likewise rotate faster than the safe-contact limit on the
first step (`0.725` to `1.571 rad/s` in magnitude). This is a
source-pad launch/contact mismatch for this feedforward profile execution,
not an obstacle collision and not a verdict that direct flight is impossible.

The next research gate should specify a physically valid source-pad release
transition and then separately test later nominal-to-plant representability.
Any release state or prelude must be declared and identity-bound before
re-evaluating the analytical profile; it must not silently skip contact in an
authoritative mission run. A physics-only continuation could help diagnose
later actuation, but it would remain a non-authoritative diagnostic. No
tracker, planner selection, or scenario default is changed by this pass.

Generate the sealed summaries as described in
[the direct-primitive research record](waypoint_direct_primitive_research.md),
then reproduce this artifact with:

```text
cargo run -p pd-eval -- waypoint-direct-nominal-plant \
  --baseline-summary /tmp/pd-waypoint-direct-primitive-analytical/summary.json \
  --sweep-summary /tmp/pd-waypoint-direct-topology-sweep/summary.json \
  --output-dir /tmp/pd-waypoint-direct-nominal-plant
```
