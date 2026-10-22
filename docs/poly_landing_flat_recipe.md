# Poly Landing flat timed-entry recipe

[Design](poly_landing_controller_plan.md) · [Execution](poly_landing_execution.md)

October 22, 2026. Prospective input-only recipe for checkpoint D and, if its
gates pass, E/F. No new local flight has informed these bounds. Historical
entries are separately exposed regression data; this new task has its own
conditional population and is not an arbitrary-airborne-state qualification.

## Primary task

Use the accepted 7,200 kg dry vehicle and native 120/60 Hz Earth plant. Ground
and pad surface are the same flat plane, pad width is 36 m, episode deadline is
40 s, and all native states are sampled at 120 Hz for independent burn-shape
assessment. Activation is immediate at tick zero. A request supplies an even
reference-end tick and the mission pad; the controller does not choose timing.

Draw six independent uniform scalars from each proposal's declared
seed/cohort/proposal/axis hash stream:

| Parameter | Raw proposal bounds | Coupling |
| --- | --- | --- |
| Reference duration | 8–24 s, rounded down to complete two-tick intervals | All subsequent expressions use the quantized duration |
| Forward speed / duration | 0.5–3 m/s² | Initial horizontal velocity is this scalar times duration |
| Downward entry speed | 8–55 m/s | Initial vertical velocity is its negative |
| Horizontal distance / (speed × duration) | 0.28–0.42 | Initial position is this distance before pad center |
| Vertical displacement mixture | 0.4–0.6 | Height above the reference endpoint is duration × (mixture × downward speed + (1−mixture) × 1.5 m/s) |
| Fuel | 1,800–6,300 kg | Original fuel and subsequent native consumption are retained |

Prepare initial attitude to the first paired reference vector, with zero angular
rate. This is an explicit preparation condition. The controller receives the
live observation, target and declared endpoint time.

The technical-repair protocol retains the 40 s original hard deadline and binds
command coverage to the supplied reference end plus the existing four ticks.
An airborne endpoint is an ordinary, replayable bounded miss. Checked backend
`polynomial_landing_v2` also scans all paired commands and fuel deductions at
admission using pure arithmetic; it never forecasts native motion or changes
the reference duration. This does not change the raw recipe or input predicate.

Development is structured: its first 64 raw proposals enumerate the six-axis
endpoint corners, the next 12 vary one endpoint axis with the other five at
their midpoints, and later proposals use the declared uniform hash stream.
Infeasible corners remain explicit input rejections. The fresh population uses
only independent uniform proposals and the same fixed admission predicate.

## Admission and complete proposals

This is a **reference-defined conditional task**. No flight outcome participates
in input admission. Retain every proposal, including input-only rejection and
its reason, before any native flight. Pure reference arithmetic must establish:

- Descending reference motion and approach without horizontal reversal at every
  discrete tick, including the original four-tick residual window.
- Positive vertical thrust. Every held vector's norm exceeds minimum engine
  acceleration at the conservative lowest remaining mass and stays below 92.5%
  of maximum acceleration at initial mass.
- Positive fuel even assuming maximum consumption through the residual window.
- Successive requested angles differ by at most the native servo's one-physics-
  tick rotation bound, so both held transitions can realize the requested angle.
  Entry fields and flat-plane clearances remain finite and positive.

Velocity sign comparisons use a fixed `1e-9 m/s` arithmetic tolerance, both in
the pure input audit and independently in the native burn-shape assessment.
Report the raw maxima rather than hiding them behind the tolerance.

No native forecast or controller update participates. These fixed margins
define the conditional support; they are not a claim that other inputs cannot
land. A scheduled NoPlan, miss, shape failure or missing proof remains scored.

Each cohort requests a fixed admitted-original count and permits at most eight
proposals per original. Underfill is a retained preparation failure. Pilot,
development and confirmation use distinct counter streams. Diagnostics and
transformations keep parent lineage and are not fresh independent trials.

## Allocations and criteria

D allocates 12 admitted pilot originals and at most 96 proposals, one worker,
60 s per attempt, ten minutes for the stage, 512 MiB of stage output and a
50 GiB free-space floor. Cost includes all replay/reconstruction work and native
shape samples. Outcomes do not authorize changing bounds or the controller.

E/F exact schedules and ceilings are released from pilot cost only, within the
design's 256 primary / 96 diagnostic development-original ceilings and one
600-original fresh confirmation. Before fresh release, freeze source, recipe,
mixture, zero shape-violation criterion and the 99% reliability criterion with
a one-sided 95% exact-binomial lower confidence bound.

Primary acceptance requires the complete physical/mission/integrity/replay
tuple, controller/planner reconstruction, one continuously powered burn, no
descending-to-climbing reversal, one construction/request, fixed reference time,
no settling/fallback owner and preserved episode clocks/fuel. Compute actual
applied thrust from adjacent native velocities and observed masses; command
mapping alone is not physical evidence when fuel is depleted. Report raw contact
speeds/attitude/margins, timing residual and fuel separately.
