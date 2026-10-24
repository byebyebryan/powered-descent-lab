# Arrival and landing qualification

The authorized pass establishes a bounded arrival-to-landing capability before
arc-shaping RL. Test the existing corrected controller first; a guidance change
is conditional on demonstrated need. The [actuation results](arrival_actuation_execution.md)
retain the 21/21 local witness result and broader landing regressions.

## Contract and preservation

Use flat airborne episodes, the current vehicle/Earth/120–60 Hz plant, unchanged
arrival tolerances and 0.72 rad tilt bound, and the existing named first-feasible
Poly coast/duration selector. The planner supplies position and velocity goals;
the controller owns commands. Keep one arrival followed by landing. Legacy
defaults, authored routes, Poly arithmetic and ordinary wire formats stay fixed.

Switch to continuation at the first actual arrival resolution, preserving the
entire live plant, attitude/angular rate, fuel and original clocks. The six-second
witness endpoint is not the handoff. Compare new and old commands before the
handoff and native observations through it. Admission is not feasibility;
finite no-plan is not a physical impossibility certificate.

## Gates

1. Compose each of the 21 existing independent native witness recipes and the
   corrected controller with unchanged Poly continuation. Preserve all inputs,
   first-resolution predicates and original deadlines. Run legacy comparisons
   separately. Capture all 42 witness/corrected executions, including misses.
2. Inspect the fixed 14 retained failure programs: nine newly lost native
   landing observations, one lost arrival, two reached-without-continuation
   and two expirations. Distinguish projection/minimum-thrust, reference/recovery
   and goal/continuation-search explanations. Native counterfactuals are bounded
   diagnostics, not new policy qualification. Make at most one justified opt-in
   controller profile; preserve calibrated force-to-setting allocation and local
   witness skill. If composition already works, qualify a restricted envelope
   without a speculative guidance change.
3. Define a continuous input-only operating envelope and two goal rules using
   development evidence, then freeze source, input generation and choices before
   fresh confirmation. Compare direct landing and both arrival choices on 60 new
   states, retaining all 180 executions and complete proof. Vary sink, lateral
   motion, attitude and goal horizon within declared bounds. Reflections remain
   paired diagnostics. No replacements, tolerance changes or confirmation-driven
   retuning. The engineering gate requires both arrival and landing for every
   claimed composite execution, without a 99% population-reliability claim.
4. Assess whether goal choice has meaningful value across states. Keep direct
   landing available. A fuel advantage over the first-feasible direct selector
   is conditional headroom, not an optimum or a fair optimized-timing claim.
   No useful choice or a failed confirmation gate is an honest stop before RL.

Every captured execution needs complete physical/mission/integrity/replay proof,
controller reconstruction, input-goal reconstruction and Summary equality. Rich
reports use the established mission tree and detail templates. Keep diagnostics,
known reachability, exposed regressions and fresh confirmation separate.

## Allocation and stop boundary

The create-only root is `outputs/arrival-composition/loop-20261024T051904Z`.
Start Unix 1792819144; outer deadline 1792829944. Limits are 150 million measured
native/model/forecast/replay steps, 256 full captures, 4 GiB retained artifacts,
one native worker, zero optimizer steps and a 40 GiB free-space floor.
Software checks are separate. Freeze exact phase allocations/capture roles after
a small cost pilot and before the corresponding outcomes, reserving proof work.

Local commits and new report delivery are authorized. No RL training, default
promotion, push, old report refresh or report-server lifecycle changes. Preserve
all pre-existing work. The [execution record](arrival_composition_execution.md)
owns outcomes, source epochs, resource receipts and any failed gates.
