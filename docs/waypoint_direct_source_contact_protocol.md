# Direct-leg source-contact diagnostic protocol

This opt-in research pass asks why the frozen nominal direct profiles stop at
the source pad, and whether their later discrete-plant trajectory would be
representable *if contact terminalization were omitted for diagnosis*. The
second question is counterfactual: it cannot establish a safe launch, route,
clearance, or landing. The unchanged simulator's mission outcome remains the
authoritative outcome for the original runs. No planner, controller, contact
rule, F6 path, or default changes are authorized by this checkpoint.

## Sealed inputs and roles

Bind the primitive baseline `fnv1a64:d3fa6b24336f7c05`, topology sweep
`fnv1a64:1bcd5a3bd6c6da01`, nominal-to-plant input manifest
`fnv1a64:9e32aa0c1a52b28b`, and prior nominal-to-plant artifact
`fnv1a64:9e8cbc902ca11fbc`. Rebuild and verify the same six probes,
scenarios, selected candidate identities, and materialized bridge/coast
profiles before any new physics step. Preserve both the native V2 and
research-shortest-certified roles; shared identities produce one candidate
profile, not a new selection. The result has six cases, nine unique profiles,
and two command cadences per profile. Do not rewrite the sealed source artifacts.

## Three separate observations

1. **Source-contact shadow audit.** For every source-bridge tick, evaluate the
   declared V2 clearance mode against the nominal post-step state, including
   its source-pad center-height allowance when the complete touchdown
   footprint is over the pad. Separately record the exact simulator post-step
   contact classification and actual touchdown/hull clearance, attitude,
   angular rate, and fuel. Preserve the first mismatch and extrema, not just
   a crash count. Contact includes stable touchdown as well as crash; a
   source-pad touchdown is not a successful departure.
2. **Counterfactual free-body continuation.** Starting from the unchanged
   scenario state, issue the identical command rule and 120 Hz per-tick or
   60 Hz two-tick-held schedule from the prior nominal-to-plant pass. Use
   `SimulationState::step_physics_and_classify_contact` through exactly the
   nominal profile end, recording every contact classification without
   applying mission terminalization. Record first state error above
   `1e-6 m` or `1e-6 m/s`, maximum errors, source/coast/terminal handoff
   errors, command saturation, and fuel. This is explicitly **not** a
   simulator mission rollout; omit landing/success claims and the prior
   post-profile idle fallback.
3. **Upright launch control.** For each frozen scenario, run one predeclared
   0.5 s (60 physics ticks) full-throttle, zero-attitude command in the
   unchanged simulator, or stop earlier if it terminates. Record contact,
   clearance, velocity, fuel, and the first sampled tick at which the static
   rotated footprint for each frozen first-powered attitude would be clear.
   This is a geometric/plant positive control, not a prelude grafted onto a
   certified candidate and not a per-case tuned launch policy.

The diagnostic must preserve the previous command conversion, candidate
order, initial state, terrain, vehicle, physics/controller cadence, and
nominal profile samples. The physics-only lane may pass through states that
the simulator would have terminated; label all such evidence counterfactual.
The upright control uses a different command by design and must remain in a
separate lane. No arbitrary airborne re-seed is permitted: V2 probes require
a source-pad rest state, and a prelude changes position, velocity, and fuel.

## Output, acceptance, and decision gate

Write an additive, identity-bound evaluator artifact with the source
identities, case/role/candidate bindings, exact diagnostic protocol, all
candidate/cadence rows, and separate upright-control rows. Exclude paths and
wall-clock timing from its semantic identity. Invalid source identity or
profile materialization stops before any simulation. A completed pass has no
silent omissions, focused tests for contact-mode accounting and cadence,
two byte-identical runs, and the integrated repository validation gate.

The pass succeeds as a *measurement* even if every candidate has later
counterfactual divergence. If a retained profile has good 120 Hz parity
beyond the source contact gap, the next design target is an explicit,
contact-safe launch phase **within** a source-pad-rest certificate, with
fuel/time/velocity/attitude included before re-solving the bridge. If later
120 Hz command/profile errors dominate, investigate those before designing
the launch primitive. The 60 Hz result remains a measured hold-cadence gap,
not an optimal-control search. Neither branch promotes a waypoint, alters a
default, or treats counterfactual clearance as authoritative safety.

## Recorded result (2026-09-23)

The no-physics input gate accepted the exact sealed sources and prior nominal
artifact under gate identity `fnv1a64:768ec21b127a1da6`. Its six cases bind
nine unique candidate profiles, twelve role labels, and eighteen cadence
continuations. Two fresh-path runs produced byte-identical `summary.json`
files (SHA-256 `11ae8040eab78943b3980a1c6b8aa8530ceac615599462668c2547c0edfdc764`),
under `/tmp/pd-waypoint-direct-source-contact-run-20260923-a` and its `-b`
repeat, with artifact identity `fnv1a64:81b3fe5b31349524`.

Every one of the nine first source ticks passes the mirrored V2 clearance
screen and is nevertheless classified as a simulator crash on physics step 1.
Across all source-bridge samples, the mirror records zero V2 clearance fails.
For the flat/native first tick, the V2 source-pad center-height margin is
`+0.000223360 m`, but actual hull and touchdown clearance are
`-0.037338819 m`; the vehicle reaches its requested `-0.009446454 rad`
attitude in that step, with `-1.1335745 rad/s` angular rate. Thus this example
is not a slew-saturation artifact. The research-shortest downhill first tick
is separately slew-limited.

In the explicitly counterfactual 120 Hz continuation, eight of nine profiles
stay within the `1e-6 m` / `1e-6 m/s` state-parity thresholds through their
full nominal profiles. Flat/native runs 4,230 ticks with maximum position and
velocity errors of about `9.3e-12 m` and `2.3e-13 m/s`. The sole exception is
the known research-shortest downhill first-tick slew mismatch; its maximum
full-profile errors are `0.0793 m` and `0.00223 m/s`. All nine 60 Hz two-tick
held-command continuations diverge at step 1 or 2, with maximum observed
position and velocity errors across profiles of `1.3285 m` and
`0.0740 m/s`. These numbers quantify representability after suppressing
terminalization; none is a flown route or landing result.

All six separate upright controls run 60/60 real simulator ticks without
contact or termination. The flat control rises about `1.014 m` in `0.5 s`,
reaches `3.992 m/s` vertical speed, and retains `6275.25 kg` fuel. Static
rotated-hull clearance for the frozen first-powered attitudes first becomes
strictly positive between physics ticks 9 and 21, depending on candidate.
Those ticks are geometric observations, not selected release times or a
certified launch schedule.

The architecture verdict is narrower than a landing claim: for these frozen
profiles, source-pad departure/contact modeling is the immediate direct-leg
blocker, not a broad inability of the per-tick plant to represent the bridge.
The next design pass should make a contact-safe launch transition part of the
source-pad-rest certificate, accounting for real attitude, rate, rotated
footprint, time, fuel, and handoff state before re-solving the remainder of
the route. The 60 Hz hold error and the downhill slew case need their own
tracking/constraint checks before any controller-facing or default route
claim. Do not splice the upright control onto an old certificate.

Reproduce the no-physics gate, then the additive artifact, with the opt-in
`waypoint-direct-source-contact` command and these inputs:

```text
cargo run -p pd-eval -- waypoint-direct-source-contact \
  --baseline-summary /tmp/pd-waypoint-direct-primitive-analytical/summary.json \
  --sweep-summary /tmp/pd-waypoint-direct-topology-sweep/summary.json \
  --nominal-summary /tmp/pd-waypoint-direct-nominal-plant-final-check-20260923/summary.json \
  --output-dir /tmp/pd-waypoint-direct-source-contact-repro \
  --preflight-only

cargo run -p pd-eval -- waypoint-direct-source-contact \
  --baseline-summary /tmp/pd-waypoint-direct-primitive-analytical/summary.json \
  --sweep-summary /tmp/pd-waypoint-direct-topology-sweep/summary.json \
  --nominal-summary /tmp/pd-waypoint-direct-nominal-plant-final-check-20260923/summary.json \
  --output-dir /tmp/pd-waypoint-direct-source-contact-repro
```
