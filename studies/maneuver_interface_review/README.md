# Planner and controller interface review checks

[Execution design](../../docs/maneuver_execution_design.md) · [Review record](../../docs/planner_controller_design_review.md)

This offline checker explores an abstract landing-request lifecycle, independent
planner/controller clocks and elapsed-time return accounting. It imports no
native simulator, controller, policy or learner, and executes no flights or
optimizer updates. Passing it establishes consistency of the bounded model,
not correctness of a production adapter or flight capability.

Run from the repository root:

```sh
rtk proxy python3 -B studies/maneuver_interface_review/check_contract.py
```

The default depth is eight abstract command intervals. Exploration merges
identical model states; it checks every outgoing event from each reachable state
within that bound, rather than listing every redundant history. Seven deliberate
lifecycle faults and two return-accounting faults must produce counterexamples.
Separate scheduling cases include termination between native command boundaries.

The simulated clock and changing command token are specification devices. They
model authority, scheduling and state retention, not vehicle dynamics, controller
guidance or native state continuity. Native integration, replay, source/advice
binding and checkpoint reconstruction require their own acceptance checks.

An optional `--output PATH` writes a create-only JSON receipt to an existing
parent directory. It binds this checker's source hash and retains counterexample
traces for the deliberate faults. Existing receipts are never overwritten.
