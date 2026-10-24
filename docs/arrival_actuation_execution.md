# Arrival actuation execution

This pass follows the [calibration and qualification plan](arrival_actuation_plan.md).
The create-only evidence root is
`outputs/arrival-actuation/loop-20261024T021508Z`.

The retained workspace baseline binds initial head `d9f2d1b`, all 903
Git-visible file hashes and pre-existing foreign changes. The outer deadline
is Unix 1792815109. Limits are 100 million measured native/model steps, 1,000
original occurrences, 64 full captures, 4 GiB artifacts, one native worker,
zero optimizer steps and at least 40 GiB free space.

Phase allocations and results will be recorded before advancing each gate.

## Frozen campaign declaration

The retained comparison has all 198 programs from the previous full and fresh
panels. Their mission bytes, requested goals and legacy Summary hashes remain
unchanged. The former fresh panel is now exposed regression data. Pilot inputs
are core cases 100, 110, 125 and 135 with `p35-v70`, in both inspected legacy
and opt-in native-pair modes. Two fixed corrected pilot captures are 100/125.
The pilot receives ten million steps and five minutes after binary readiness.

The main comparison receives 70 million steps and 16 fixed corrected captures:
four angles × low/high energy × clean/+75 m miss, all with `p35-v70`.
Direct programs are preservation controls, not a new timing policy.

Witness construction uses 21 declared recipes: vertical motion at 15/30/45 m/s
sink, plus lateral, braking and attitude-preparation motion at those sinks in
both reflected directions. Reflections are paired diagnostics. Every recipe
runs six seconds; preparation recipes coast/point for the first half second.
Native endpoints define goals with the existing 30 m and velocity tolerances.
Construction must satisfy the unchanged complete fly-through predicate before
being admitted. Exclusions are retained without replacement. Witness, legacy
controller and corrected controller each execute every admitted goal. Capture
the independent witness and corrected controller for each, regardless of
outcome: at most 42 witness-phase captures and 20 million steps.

The exact declaration and input hashes precede any measured execution.
Maximum capture roles across phases are 60. Native calibration covers minimum
fractions 0/0.25/1, six fuel states, seven acceleration demands, aligned/rotating
attitude and both profiles: 504 command pairs and 1,008 native steps.
