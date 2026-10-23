# Airborne arrival and Poly Landing

Status: authorized implementation and characterization, 2026-10-23. This pass
ends before RL training or promotion of a default planner/controller.

The planner selects an optional intermediate arrival goal. The controller owns
the commands needed to reach it and the subsequent landing. Reuse the existing
waypoint guidance kernel; do not create another guidance law or silently use
terminal PDG in place of Poly Landing.

## Contracts

An arrival request specifies a world position, an outbound velocity envelope,
position/cross-track tolerances and an absolute operation deadline. Resolve its
anchor from the actual airborne observation once. Admission accepts an attempt,
not a reachability guarantee. Matching duplicate requests are idempotent;
retargeting while active is busy. Stale observations, invalid envelopes, terrain,
authored routes and unsupported clocks are rejected explicitly.

Reuse waypoint capture-window and handoff predicates, retained horizon search,
actuator prediction and the existing tilt limit. Report admission, window entry,
contract completion, plane miss and expiry separately. A finite forecast that
has not found the event is not a proof that the goal is unreachable.

The new composition has at most one arrival, followed by one deterministic
landing selection. It keeps the original native episode, fuel and clocks.
The landing selector may coast and prepare attitude, then activate the existing
checked Poly Landing adapter with an explicit absolute reference endpoint.
That adapter's immediate activation and finite-reference semantics stay intact.
Every new selector is named and recorded; no claim of old-planner parity applies
to the new selector. Failed arrivals still continue from their actual state;
arrival fulfillment and physical landing remain distinct outcomes.

## Evidence gates

1. Preserve authored-route behavior and test live arrival admission, clocks,
   capture predicates and command ownership against the shared kernel.
2. Characterize arrival execution separately from deterministic continuation on
   flat airborne missions spanning angle, energy, reflection and trajectory miss.
3. Compare direct landing and a finite, input-defined arrival-goal bank. Report
   hindsight selection as a diagnostic, separately from any frozen input rule.
4. Bind captures to original inputs and runtime source. Require physical,
   mission, integrity and complete replay evidence for verified landing claims.
   Check controller reconstruction and Summary/Captured parity.
5. Use the maintained rich mission tree and flight details. Preserve existing
   reports, selectors, server lifecycle and historical evidence.

Freeze cohort generation, choice bank, selector grids, selection rules and cost
allocation in the execution record before measuring the corresponding cohort.
Use a small cost pilot to set the full allocation. Do not outcome-filter inputs
or add a mandatory intermediate maneuver to manufacture decision headroom.
Zero useful headroom is an acceptable research result.

## Boundaries

No terrain/launch/transfer qualification, continuously retargeted goals, new
learned controller, angle-limit relaxation, reward tuning, optimizer steps or
push. Local commits are authorized. Preserve unrelated working tree files.
Stop at three hours, 150 million native/model steps, 6,000 originals, 16 GiB of
new artifacts or 40 GiB remaining free space, whichever comes first. Account
forecast, reconstruction and proof work as well as visible flights.

See [execution and results](airborne_arrival_execution.md).
