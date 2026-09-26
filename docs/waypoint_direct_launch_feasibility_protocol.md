# Direct-leg launch-feasibility canary protocol

This opt-in pass tests one deterministic launch-aware direct-profile rule in
the unchanged simulator. It is a feasibility canary, not a new V2
certificate, planner selection, controller policy, or claim that a direct
route is generally possible or impossible. The sealed V2 baseline, topology
sweep, nominal-plant result, and their candidate identities remain inputs,
not objects to rewrite. The source-contact diagnostic motivates this pass but
its upright control was a separate positive control, not an already selected
launch.

## Frozen construction, before outcomes

Revalidate the frozen six-case baseline/sweep/nominal input gate before any
physics. Preserve the native V2 and research-shortest-certified role for each
case; identical candidate identities share one profile. Test flat terrain
first. For each selected flat profile, and only then any conditional cases,
construct the following from its source-pad-rest scenario:

1. Apply full throttle at zero target attitude for exactly 60 physics ticks
   (0.5 s at 120 Hz), stopping if the real simulator terminates or reports
   any post-step contact.
2. Apply full throttle for exactly 12 physics ticks while targeting that
   frozen profile's first powered source-bridge command attitude, defined by
   the first nonzero source thrust sample as
   `atan2(thrust_acceleration.x, thrust_acceleration.y)`. This is a
   predeclared tilt command, not a search over tilt times or angles. Stop on
   any real contact or terminal state. Record actual position, velocity,
   attitude, angular rate, remaining fuel, time, clearance, and commanded
   versus achieved attitude at the end. The 60+12 tick boundary is even so
   the 60 Hz held-command lane has an unambiguous transition.
3. From that *achieved* position and velocity, call the opt-in exact discrete
   source-bridge solve to the frozen candidate's source handoff state, using
   the frozen source-bridge tick count. Preserve its source-handoff arc step,
   coast, terminal bridge, and candidate role. Do not reuse the old source
   bridge or silently subtract launch time from its tick count. This adds 72
   physics ticks to the old total; account for them explicitly in fuel and
   mission time.
4. Use the existing nominal command inversion with the actual pre-step mass
   to replay the reseeded bridge, coast, and terminal bridge through the
   ordinary simulator. Report its exact per-step contact, actual clearance,
   state error, command saturation, phase handoffs, and final mission outcome.
   The launch/tilt and all subsequent steps must remain contact-free until
   the target touchdown to support a flown-direct claim.

The source bridge's own thrust, minimum-throttle, endpoint, and powered-slew
classification is recorded, along with its identity. Also record the
conservative fuel margin from actual launch/tilt burn plus re-solved source
and frozen terminal bridge burn, and the total-time margin against the
policy's mission budget. Neither bridge classification nor those aggregate
margins replace the private V2 terrain/source-attitude screens or the real
simulator contact result. Specifically, never label this wrapper `Certified`
under V2 semantics. Record the actual attitude and angular rate at the first
re-solved source tick, including the launch-end-attitude to first new thrust
direction slew gap that the bridge's within-profile check does not cover.
Even a materialized bridge with failing analytical margins is replayed
diagnostically, with command saturation and analytical eligibility separate
from simulator outcome. Any commandability failure is a measured failure of
this rule, not a reason to alter simulator contact behavior.

Run separate 120 Hz per-physics-tick and 60 Hz two-tick-held command lanes
from the same supported source-rest state. The 120 Hz lane diagnoses whether
the newly materialized profile can be followed; the 60 Hz lane measures the
controller cadence that matters for later integration. Do not infer 60 Hz
success from 120 Hz success. No per-case timing, angle, handoff, bridge-step,
or candidate adjustment is allowed after seeing outcomes.

## Stop and evidence gates

The first gate is the flat case. Construct and execute both frozen flat roles
through the source handoff, recording a row even when a bridge is not
analytically feasible or the simulator terminates early. If neither role is
contact-free through the launch, tilt, and source handoff at 120 Hz, stop the
pass there and report this fixed-handoff rule as inconclusive for general
flat direct flight. If at least one role passes that gate, finish flat
mission rollouts and then run uphill/downhill. Only if the uncut cases show
a useful contact-free handoff should the three frozen obstacle cases be run.
For every attempted case, record full-mission status, not just handoff status.
No case is dropped because it failed.

Produce an additive semantic-identity artifact binding every frozen input,
case/role/candidate, exact 60+12 protocol, achieved state, re-solved bridge,
margin, cadence, and rollout outcome. Exclude paths and wall time from its
identity. Preflight must reject changed source/profile bindings before any
simulation. Focused tests cover the protocol and identity boundaries, then
two fresh-path runs must be byte-identical. Run the integrated workspace
format, tests, and strict lint gate after the focused gate. A negative result
does not authorize a lowered-floor workaround, a contact-rule change, a
different launch search, or default/F6 promotion; those require a separate
decision.

## Recorded result (2026-09-23)

The no-physics input gate accepted the sealed baseline, sweep, nominal
result, and manifest under identity `fnv1a64:a4654d06c9166b14`: six cases,
nine unique selected profiles, and twelve role bindings. A flat-only pilot
stopped at its declared review boundary. Its corrected artifact is
`fnv1a64:7910837eafd17cf4` at
`/tmp/pd-waypoint-direct-launch-feasibility-flat-gate-20260923-02/summary.json`.
Both flat profiles completed the 72 launch ticks without contact and reached
the frozen source handoff at 120 Hz within `1e-6 m` and `1e-6 m/s`.

The conditional-full pass then opened both uncut slopes; their three unique
selected profiles all passed that same 120 Hz handoff gate, so it opened all
three frozen obstacle cases. Thus all six cases and nine profiles ran, with
no conditional omissions. Two fresh-path `summary.json` files at
`/tmp/pd-waypoint-direct-launch-feasibility-conditional-20260923-01` and
`-02` are byte-identical (SHA-256
`deab83d3af4cb96b0c608a1266912fc83c1d96bdba750b7610470e74b0627174`),
with semantic identity `fnv1a64:2c7b965ffdc809d6`. The artifact preserves
the original V2 candidate identities only as bases for newly identified
launch/reseed wrappers.

All nine 120 Hz lanes reached the source handoff contact-free within the
strict state tolerance. All nine 60 Hz two-tick-held lanes missed that
nominal handoff tolerance, despite using the same contact-free launch. Six
of nine profiles landed on target at each cadence; three crashed near the
target (flat research-shortest, uphill shared, and the height-0.20 obstacle
research-shortest). No early source-pad or obstacle contact occurred in
these rollouts. A target touchdown or crash contact tick is recorded
separately from free-flight tracking error.

The flat native-basis wrapper landed on target in both lanes, while its
re-solved bridge is `NotCertified`: the coupled-thrust normalized reserve is
`0.0561`, below the policy's `0.075` required robustness margin. Its 120 Hz
free-flight profile parity is within about `9.5e-12 m` and `2.4e-13 m/s`;
the 60 Hz lane misses source handoff by about `0.214 m` and `0.0293 m/s` yet
later lands. The flat research-shortest re-solved bridge passes its recorded
analytical screens, reaches handoff, and crashes at target contact instead.
From the recorded flat crash state and frozen 4 m touchdown half-span/5 m
base offset, its upper foot is about `0.290 m` above flat terrain when the
lower foot first touches—above the simulator's `0.15 m` stable-touchdown
limit. This is a terminal contact-geometry mismatch, not a source departure
or en-route obstacle strike.

The verdict is constructive but narrow: the unchanged plant can depart a
source pad on flat, uphill, and downhill uncut terrain without a lowered
floor under this fixed opt-in launch rule, and can land a flat direct
profile. The old step-one crash exposed an analytical source-contact modeling
gap, not proof that direct flight needs a cutaway. The rule is **not** ready
for planner or controller selection: four landed profiles fail the declared
analytical robustness screen; three analytically eligible profiles crash
near target; and every 60 Hz lane misses its strict nominal source handoff.
The next research target is a certificate that includes launch state/attitude/rate,
actual first-contact geometry, and 60 Hz command holding, with selection
and terminal behavior validated together. No production planner, contact
rule, controller, F6 integration, or default was changed.

Reproduce the input gate, then the conditional artifact, with the opt-in
command and the frozen source paths:

```text
cargo run -p pd-eval -- waypoint-direct-launch-feasibility \
  --baseline-summary /tmp/pd-waypoint-direct-primitive-analytical/summary.json \
  --sweep-summary /tmp/pd-waypoint-direct-topology-sweep/summary.json \
  --nominal-summary /tmp/pd-waypoint-direct-nominal-plant-final-check-20260923/summary.json \
  --output-dir /tmp/pd-waypoint-direct-launch-feasibility-repro \
  --preflight-only

cargo run -p pd-eval -- waypoint-direct-launch-feasibility \
  --baseline-summary /tmp/pd-waypoint-direct-primitive-analytical/summary.json \
  --sweep-summary /tmp/pd-waypoint-direct-topology-sweep/summary.json \
  --nominal-summary /tmp/pd-waypoint-direct-nominal-plant-final-check-20260923/summary.json \
  --output-dir /tmp/pd-waypoint-direct-launch-feasibility-repro
```

The final tree passed 631 workspace tests, strict workspace Clippy, formatting,
and `git diff --check`. The post-lint-refactor full artifact remained
byte-identical to the two original conditional runs.
