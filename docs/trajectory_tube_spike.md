# Trajectory-tube shadow spike

Status: research-only shadow tool. This note and `pd-plan/examples/trajectory_tube_spike.rs`
do not change planner, controller, schema, or fixture behavior. The simulation remains the
authority for mission outcomes.

## Reproduction

From the repository root, run:

```text
rtk cargo run -q -p pd-plan --example trajectory_tube_spike -- \
  baseline=/tmp/pd-planning-phase5-contract-quantized-20260818/summary.json \
  expansion=/tmp/pd-planning-angle-expansion-contract-final-boundary-20260818/summary.json
```

The two `/tmp` paths are retained local snapshots used for this spike; they are not committed
evidence. Each summary record is resolved to its bundle, and the tool reads `scenario.json`,
`route_plan.json`, and `manifest.json`. Labels, run IDs, manifest outcomes, seeds, metadata, and
controller configuration are aggregation/output data and never enter the pure evaluator.

## Model and invariants

- The scored route is the prefix from the actual initial state through the last waypoint. The
  target pad supplies outbound direction/source geometry only; the final landing leg is out of
  scope.
- The witness is a deterministic sequence of quintics. Each segment starts at the preceding
  segment's endpoint velocity, and each waypoint tangent-aligned terminal velocity is reused as
  the next segment's initial velocity. Segment endpoint acceleration is explicitly zero.
- Handoff speeds use a finite contracted grid. Total duration uses
  `min(covered_length / minimum_average_speed, maximum_duration)` and factors `1.00`, `0.85`,
  and `0.70`, so a candidate cannot become an arbitrarily slow witness.
- Exactly three fixed research profiles are evaluated: permissive, moderate, and conservative.
  They vary the tube/clearance, reserve, duration/speed, tilt, and explicit maximum thrust
  utilization caps of `1.00`, `0.90`, and `0.80`.
- The full waypoint handoff envelope is checked: position tube, velocity-error budget against
  outbound cross-speed, heading uncertainty at the actual candidate speed, lower/upper total
  speed, outbound progress, and optional vertical-speed margins.
- Each segment is sampled at 96 deterministic intervals. Samples check terrain plus the projected
  vehicle hull and position tube, with a source touchdown-footprint taper. Required thrust
  includes gravity and is checked against utilization and tilt caps; attitude-rate is checked
  against the vehicle rotation-rate limit. Fuel consumption is omitted.
- Direct routes are reported as skipped, never silently classified. The evaluator does not branch
  on labels or recorded outcomes.

## Corrected-run classification

The final corrected run produced the following exact counts. TP/TN/FP/FN compare the evaluator's
feasible/infeasible result with the serialized simulation outcome; skipped cases are excluded
from those four confusion counts.

| profile | corpus | total | feasible | infeasible | skipped | TP | TN | FP | FN |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| permissive | baseline | 36 | 12 | 24 | 0 | 12 | 0 | 0 | 24 |
| permissive | expansion | 24 | 7 | 17 | 0 | 0 | 6 | 7 | 11 |
| moderate | baseline | 36 | 6 | 30 | 0 | 6 | 0 | 0 | 30 |
| moderate | expansion | 24 | 4 | 20 | 0 | 0 | 9 | 4 | 11 |
| conservative | baseline | 36 | 2 | 34 | 0 | 2 | 0 | 0 | 34 |
| conservative | expansion | 24 | 3 | 21 | 0 | 0 | 10 | 3 | 11 |

The compact limiting-constraint distributions below include every case in each profile/corpus;
`none` is a feasible witness.

| profile | corpus | limiting distribution |
| --- | --- | --- |
| permissive | baseline | `none:12, terrain_hull_tube:24` |
| permissive | expansion | `gravity_inclusive_thrust:3, none:7, terrain_hull_tube:14` |
| moderate | baseline | `gravity_inclusive_thrust:10, none:6, terrain_hull_tube:20` |
| moderate | expansion | `gravity_inclusive_thrust:12, none:4, terrain_hull_tube:8` |
| conservative | baseline | `gravity_inclusive_thrust:20, none:2, terrain_hull_tube:14` |
| conservative | expansion | `gravity_inclusive_thrust:7, handoff_envelope:6, none:3, terrain_hull_tube:3, tilt_cap:5` |

No profile preserves the maintained baseline: the permissive profile has 12/36 baseline TP,
while moderate and conservative have 6/36 and 2/36. On expansion, permissive produces 7 FP;
the stricter profiles trade those for fewer FP (4 and 3) while increasing baseline FN (30 and
34). These are observations from the fixed physical-input evaluator, not label-tuned thresholds.

## Clearance interpretation and superseded evidence

The first implementation incorrectly scored an invented zero-velocity segment from the last
waypoint to the target. Several maintained baseline attempts then failed on tiny negative
clearance on that unscored terminal approach. The corrected prefix-only run supersedes those
results.

The corrected diagnostic records elapsed time, segment index, normalized segment fraction, and
position for the minimum sampled clearance, and marks whether the source touchdown taper is
still active. A representative permissive baseline mismatch is:

```text
clearance_m=-0.01, clearance_location=t=0.63, seg=0, tau=0.021,
pos=(-692.82,405.01), source_contact=true
```

All 72 printed terrain-limited mismatch lines marked the minimum
`source_contact=true`; the examples occur immediately on segment 0 (roughly `tau=0.010` to
`0.083`). This points toward a source-departure/acquisition artifact as a leading explanation for
the negative clearance, rather than the removed terminal landing leg. It is not proof that every
interior sample is safe: only the best failed candidate for each mismatch is printed, and the
sampled check is not an exact continuous minimum.

## Limitations and production gate

This is not a production feasibility model. It has center-only handoff states; no neutral
source-departure/acquisition state; dense rather than exact polynomial clearance; an
initial-mass/average-thrust simplification; no fuel; no landing leg; selected routes only rather
than all candidates; and profiles that are not a calibrated tracking-error bound.

Do not integrate this model into planning or control. Next design work must define a
planner-owned source-departure/acquisition capability and a tracking-tube or paired-controller
contract, then rerun on held-out evidence. If that contract cannot preserve maintained passes
without route-label tuning, escalate to a small convex-feasibility or paired-reachability
formulation. Candidate replay needs separate candidate exposure before it can be used as evidence.

## Suggested next investigation

Keep the next pass design-first and separate capability definition from model fitting:

1. Mine the maintained and expansion bundles at the transition into waypoint guidance. Record
   position, velocity, attitude, angular rate, mass, and clearance relative to the source and
   first outbound leg. Do not turn controller phase names or current thresholds into planner
   inputs.
2. Decide whether those states support one neutral source-departure/acquisition envelope. The
   envelope must be versioned, physical, seed-blind, and independent of controller identity; if
   the evidence requires route-family branches, stop and revisit ownership.
3. Pair that envelope with an explicit tracking-error tube or executor capability. Sequential
   composition must carry the terminal state and error set from waypoint `N` into waypoint
   `N + 1`; independent waypoint screens are insufficient.
4. Split evidence before choosing thresholds. Use maintained cases to establish non-regression,
   reserve held-out route-angle/radius cases for discrimination, and reject any model that needs
   route labels or recorded outcomes inside evaluation.
5. Add bounded candidate exposure only for research replay. Determine whether an existing
   candidate passes the richer model before changing planner node generation or ranking.
6. If the richer contract still cannot preserve maintained passes and reject unsafe selections,
   stop adding scalar reserves. Compare a small convex-feasibility formulation with a paired
   reachability/tracking model as the next architecture decision.
