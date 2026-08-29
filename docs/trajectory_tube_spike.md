# Trajectory-tube shadow spike

Status: research-only archival note. The temporary evaluator, diagnostic packs,
and example executable from the route-angle research branch are intentionally
excluded from `main`. This note preserves the design evidence without making
the experiment part of the planner, controller, schema, fixture, or acceptance
contracts. Simulation remains authoritative for mission outcomes.

The work is retained for provenance in the `bde34c5` route-angle expansion and
`b390d69` trajectory-tube research commits on the
`codex/route-angle-trackability-spike` branch. The local summary and bundle
snapshots used for the study were disposable research inputs, not maintained
repository evidence.

## Research question

The nominal-radius `r-60 | r+60` route-angle expansion was useful as a
diagnostic boundary, but its center-to-center route geometry did not say
whether guidance could acquire and track each handoff. The spike asked whether
a small deterministic trajectory witness could provide a useful feasibility
screen while preserving the closed V1 planner boundary.

It could not. The result is a production stop rule for this model, not a new
planner rejection claim.

## Temporary model and invariants

The shadow study used the following fixed physical-input evaluator:

- The scored route was the actual initial state through the final waypoint.
  The target pad supplied outbound direction and source geometry only; the
  final landing leg was deliberately out of scope.
- The witness was a deterministic sequence of quintic segments. Each segment
  started at the preceding endpoint velocity, waypoint terminal velocity was
  aligned with the contracted outbound tangent, and segment endpoint
  acceleration was zero.
- Handoff speeds came from a finite contracted grid. Duration used the covered
  path length divided by the minimum average speed, capped by the maximum
  duration, with fixed factors `1.00`, `0.85`, and `0.70`. The corrected bound
  prevented an arbitrarily slow witness from manufacturing clearance.
- Exactly three fixed profiles were evaluated: permissive, moderate, and
  conservative. They varied tube/clearance, reserve, duration/speed, tilt, and
  explicit maximum-thrust utilization caps of `1.00`, `0.90`, and `0.80`.
- The complete waypoint handoff envelope was checked: position tube,
  velocity error against outbound cross-speed, heading uncertainty at the
  actual candidate speed, lower/upper total speed, outbound progress, and
  optional vertical-speed margins.
- Each segment used a deterministic 96-interval sample grid. Samples checked
  terrain against the projected vehicle hull and position tube, including a
  source touchdown-footprint taper. Required thrust included gravity and was
  checked against utilization and tilt caps; attitude rate was checked against
  the vehicle rotation-rate limit. Fuel consumption was not modeled.
- Direct routes were reported as skipped rather than silently classified. No
  labels, seeds, controller identifiers, manifest outcomes, or recorded
  results entered the pure evaluator.

These invariants made the experiment useful for falsification, but they did not
turn it into a dynamic-feasibility proof or a calibrated tracking bound.

## Corrected-run results

The corrected run compared the evaluator's feasible/infeasible result with the
serialized simulation outcome. `TP/TN/FP/FN` are therefore classification
counts; skipped direct routes are excluded.

| profile | corpus | total | feasible | infeasible | skipped | TP | TN | FP | FN |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| permissive | baseline | 36 | 12 | 24 | 0 | 12 | 0 | 0 | 24 |
| permissive | expansion | 24 | 7 | 17 | 0 | 0 | 6 | 7 | 11 |
| moderate | baseline | 36 | 6 | 30 | 0 | 6 | 0 | 0 | 30 |
| moderate | expansion | 24 | 4 | 20 | 0 | 0 | 9 | 4 | 11 |
| conservative | baseline | 36 | 2 | 34 | 0 | 2 | 0 | 0 | 34 |
| conservative | expansion | 24 | 3 | 21 | 0 | 0 | 10 | 3 | 11 |

The limiting-constraint distribution included every case in each profile and
corpus; `none` means that the witness passed the shadow checks.

| profile | corpus | limiting distribution |
| --- | --- | --- |
| permissive | baseline | `none:12, terrain_hull_tube:24` |
| permissive | expansion | `gravity_inclusive_thrust:3, none:7, terrain_hull_tube:14` |
| moderate | baseline | `gravity_inclusive_thrust:10, none:6, terrain_hull_tube:20` |
| moderate | expansion | `gravity_inclusive_thrust:12, none:4, terrain_hull_tube:8` |
| conservative | baseline | `gravity_inclusive_thrust:20, none:2, terrain_hull_tube:14` |
| conservative | expansion | `gravity_inclusive_thrust:7, handoff_envelope:6, none:3, terrain_hull_tube:3, tilt_cap:5` |

No profile preserved the maintained baseline. The permissive profile preserved
only `12 / 36` baseline successes, while moderate and conservative preserved
`6 / 36` and `2 / 36`. On the expansion, permissive produced `7 / 13` false
accepts; the stricter profiles reduced false accepts to `4 / 13` and `3 / 13`
while preserving none of the `11` successful expansion runs. These are fixed
physical-input observations, not label-tuned thresholds.

## Superseded errors and source-contact interpretation

The first implementation scored an invented zero-velocity segment from the
last waypoint to the target even though that landing leg was outside the
question. It also reversed the minimum-average-speed duration bound. Several
early baseline failures therefore came from a route segment that was not
scored by the intended contract. The corrected handoff-prefix run supersedes
those results.

The corrected diagnostics recorded elapsed time, segment index, normalized
segment fraction, position, and whether the source touchdown taper was still
active. A representative permissive-baseline mismatch was:

```text
clearance_m=-0.01, clearance_location=t=0.63, seg=0, tau=0.021,
pos=(-692.82,405.01), source_contact=true
```

All `72` printed terrain-limited mismatch lines marked
`source_contact=true`; the examples occurred immediately on segment zero,
roughly `tau=0.010` through `0.083`. This makes an undeclared
source-departure/acquisition transition the leading explanation for the
negative clearance, rather than the removed terminal landing leg. It does not
prove every interior sample safe: only the best failed candidate for each
mismatch was printed, and the sampled check was not an exact continuous
minimum.

## Limitations and production gate

The center-only polynomial model is not a production feasibility oracle. It
has no neutral source-departure/acquisition state, uses dense rather than exact
polynomial clearance, simplifies authority with initial mass and average
thrust, omits fuel and the landing leg, evaluates selected routes rather than
all candidates, and uses profiles that are not calibrated tracking-error
bounds.

Do not integrate or tune this model into planning or control. The next design
checkpoint must define a planner-owned source-departure/acquisition capability
and an explicit tracking-error tube or paired-executor contract, then validate
it on held-out evidence. If that contract cannot preserve maintained passes
without route-label tuning, escalate to a small convex-feasibility or paired-
reachability formulation. Candidate replay requires separately bounded
candidate exposure before it can provide evidence about selection versus
candidate-generation limits.

## Suggested next investigation

Keep the next pass design-first and separate capability definition from model
fitting:

1. Mine maintained and expansion bundles at the transition into waypoint
   guidance. Record position, velocity, attitude, angular rate, mass, and
   clearance relative to the source and first outbound leg. Do not turn
   controller phase names or current thresholds into planner inputs.
2. Decide whether those states support one neutral source-departure/acquisition
   envelope. The envelope must be versioned, physical, seed-blind, and
   independent of controller identity; if the evidence requires route-family
   branches, stop and revisit ownership.
3. Pair that envelope with an explicit tracking-error tube or executor
   capability. Sequential composition must carry the terminal state and error
   set from waypoint `N` into waypoint `N + 1`; independent waypoint screens
   are insufficient.
4. Split evidence before choosing thresholds. Use maintained cases to
   establish non-regression, reserve held-out route-angle/radius cases for
   discrimination, and reject any model that needs route labels or recorded
   outcomes inside evaluation.
5. Add bounded candidate exposure only for research replay. Determine whether
   an existing candidate passes the richer model before changing planner node
   generation or ranking.
6. If the richer contract still cannot preserve maintained passes and reject
   unsafe selections, stop adding scalar reserves. Compare a small
   convex-feasibility formulation with a paired reachability/tracking model as
   the next architecture decision.
